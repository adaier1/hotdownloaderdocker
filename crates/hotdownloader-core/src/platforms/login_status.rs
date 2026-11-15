//! 运行时共用的账号在线校验。公开结果只含账号标识，不返回任何令牌。
use super::{kugou, netease, qqmusic::login as qq, Platform};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub logged_in: bool,
    pub user_id: String,
    pub nickname: String,
    pub expired: bool,
    pub message: String,
}

impl AccountStatus {
    fn logged_out() -> Self {
        Self {
            logged_in: false,
            user_id: String::new(),
            nickname: String::new(),
            expired: false,
            message: String::new(),
        }
    }
    fn from_legacy(value: Value) -> Self {
        Self {
            logged_in: value["logged_in"].as_bool().unwrap_or(false),
            user_id: value["userid"].as_str().unwrap_or("").into(),
            nickname: value["nickname"].as_str().unwrap_or("").into(),
            ..Self::logged_out()
        }
    }
    fn expired(user_id: String) -> Self {
        Self {
            user_id,
            expired: true,
            message: "登录已失效，请重新登录".into(),
            ..Self::logged_out()
        }
    }
}

/// 仅明确的认证拒绝算失效；网络、限流、平台验证和解析失败都保留为检查失败。
pub fn is_auth_error(platform: Platform, error: &str) -> bool {
    match platform {
        Platform::QqMusic => qq::is_credential_rejection(error),
        Platform::Netease => {
            error.starts_with("网易云请求失败 (301)")
                || error.starts_with("网易云请求失败 (302)")
                || error == "网易云登录已失效，请重新登录"
        }
        Platform::Kugou => {
            let text = error.to_lowercase();
            error.starts_with("酷狗请求失败")
                && (text.contains("未登录")
                    || text.contains("登录失效")
                    || text.contains("请登录")
                    || (text.contains("token")
                        && [
                            "失效",
                            "错误",
                            "过期",
                            "无效",
                            "验证失败",
                            "校验失败",
                            "不正确",
                            "invalid",
                            "expired",
                        ]
                        .iter()
                        .any(|word| text.contains(word))))
        }
        Platform::Kuwo => false,
    }
}

/// 启动时 refresh=true：QQ 按需续期，网易云调用续期接口，酷狗校验已保存令牌。
/// 没有凭据时不访问平台；所有存储由 Docker/Tauri 宿主注入。
pub async fn check(
    platform: Platform,
    refresh: bool,
    qq_store: &dyn qq::LoginCredentialStore,
    netease_store: &netease::login::Store,
    kugou_store: &kugou::login::Store,
) -> Result<AccountStatus, String> {
    match platform {
        Platform::QqMusic => {
            let saved = qq_store.load()?;
            let uin = saved["loginUin"].as_str().unwrap_or("").to_string();
            if uin.is_empty() || saved["authst"].as_str().unwrap_or("").is_empty() {
                return Ok(AccountStatus::logged_out());
            }
            let checked = qq::check_credential_expired(qq_store).await;
            if !qq::credential_snapshot_matches(&qq_store.load()?, &saved) {
                return Err("账号已更新，请重新检查登录状态".into());
            }
            match checked {
                Ok(false) => Ok(AccountStatus {
                    logged_in: true,
                    user_id: uin,
                    ..AccountStatus::logged_out()
                }),
                Ok(true) => {
                    if refresh {
                        if let Ok(credentials) = qq::refresh_credential(qq_store).await {
                            return Ok(AccountStatus {
                                logged_in: true,
                                user_id: credentials.uin,
                                ..AccountStatus::logged_out()
                            });
                        }
                        let latest = qq_store.load()?;
                        if latest["loginUin"]
                            .as_str()
                            .is_some_and(|uin| !uin.is_empty())
                            && !qq::credential_snapshot_matches(&latest, &saved)
                        {
                            return Err("账号已更新，请重新检查登录状态".into());
                        }
                    }
                    Ok(AccountStatus::expired(uin))
                }
                Err(_) => Err("QQ 音乐登录状态检查失败，请稍后重试".into()),
            }
        }
        Platform::Netease => {
            let cookie = netease_store.cookie()?;
            if cookie.is_empty() {
                return Ok(AccountStatus::logged_out());
            }
            let checked = netease::login::execute(
                netease_store,
                if refresh { "refresh" } else { "status" },
                "",
                None,
            )
            .await;
            if !refresh && netease_store.cookie()? != cookie {
                return Err("账号已更新，请重新检查登录状态".into());
            }
            match checked {
                Ok(value) => Ok(AccountStatus::from_legacy(value)),
                Err(error) if is_auth_error(platform, &error) => {
                    Ok(AccountStatus::expired(String::new()))
                }
                Err(_) => Err("网易云音乐登录状态检查失败，请稍后重试".into()),
            }
        }
        Platform::Kugou => {
            let params = kugou_store.request_params()?;
            let userid = params["userid"].as_str().unwrap_or("").to_string();
            if userid.is_empty() || params["token"].as_str().unwrap_or("").is_empty() {
                return Ok(AccountStatus::logged_out());
            }
            let checked = kugou::login::check_account(kugou_store).await;
            if kugou_store.request_params()? != params {
                return Err("账号已更新，请重新检查登录状态".into());
            }
            match checked {
                Ok(()) => Ok(AccountStatus {
                    logged_in: true,
                    user_id: userid,
                    ..AccountStatus::logged_out()
                }),
                Err(error) if is_auth_error(platform, &error) => Ok(AccountStatus::expired(userid)),
                Err(_) => Err("酷狗音乐登录状态检查失败，请稍后重试".into()),
            }
        }
        Platform::Kuwo => Err("该平台暂不支持登录".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn network_limits_and_permissions_do_not_prove_expiration() {
        for (platform, rejected) in [
            (Platform::QqMusic, "接口错误: code=104401"),
            (Platform::Netease, "网易云请求失败 (301): 未登录"),
            (Platform::Kugou, "酷狗请求失败 (20010): token验证失败"),
        ] {
            assert!(is_auth_error(platform, rejected));
        }
        for (platform, error) in [
            (Platform::QqMusic, "网络错误: timeout"),
            (Platform::QqMusic, "接口错误: code=2000"),
            (Platform::Netease, "网易云请求失败 (429): 请求过多"),
            (Platform::Kugou, "酷狗请求失败 (20028): 需要验证"),
            (Platform::Kugou, "酷狗网络错误: timeout"),
        ] {
            assert!(!is_auth_error(platform, error));
        }
    }

    #[tokio::test]
    async fn missing_credentials_never_start_online_requests_or_create_files() {
        let root = std::env::temp_dir().join(format!(
            "hotdownloader-account-check-{}",
            crate::platforms::get_guid()
        ));
        let qq = qq::FileLoginStore::new(root.join("qq.json"));
        let netease = netease::login::Store::new(root.join("netease.json"));
        let kugou = kugou::login::Store::new(root.join("kugou.json"));
        for refresh in [false, true] {
            for platform in [Platform::QqMusic, Platform::Netease, Platform::Kugou] {
                let status = check(platform, refresh, &qq, &netease, &kugou)
                    .await
                    .unwrap();
                assert!(!status.logged_in);
                assert!(!status.expired);
            }
        }
        assert!(!root.exists());
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("qq.json"), "{").unwrap();
        assert!(check(Platform::QqMusic, true, &qq, &netease, &kugou)
            .await
            .is_err());
        assert_eq!(std::fs::read_to_string(root.join("qq.json")).unwrap(), "{");
        std::fs::remove_file(root.join("qq.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn public_status_has_no_tokens_and_setting_is_valid_for_both_runtimes() {
        let status = AccountStatus::from_legacy(
            json!({"logged_in":true,"userid":"123","nickname":"示例", "token":"secret", "cookie":"secret"}),
        );
        assert!(!serde_json::to_string(&status).unwrap().contains("secret"));
        for scope in [
            crate::settings::patch::SettingsScope::Web,
            crate::settings::patch::SettingsScope::Tauri,
        ] {
            let patch = serde_json::from_value(json!({"changes":{"autoRefreshLoginOnStartup":false},"expected":{"autoRefreshLoginOnStartup":null}})).unwrap();
            let result = crate::settings::patch::apply_patch(&json!({}), patch, scope).unwrap();
            assert_eq!(result.snapshot.settings["autoRefreshLoginOnStartup"], false);
        }
    }
}
