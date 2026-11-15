//! 酷狗登录与设备信息。凭据只保存在运行时的独立文件，不返回浏览器或任务记录。
use super::client::{number, request_with, text};
use crate::platforms::{
    qqmusic::login::{FileLoginStore, LoginCredentialStore},
    CLIENT,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

pub struct Store {
    path: PathBuf,
    file: FileLoginStore,
}

impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            file: FileLoginStore::new(&path),
            path,
        }
    }
    fn load(&self) -> Result<Value, String> {
        self.file.load().map_err(|e| e.replace("QQ", "酷狗"))
    }
    fn save(&self, value: &Value) -> Result<(), String> {
        self.file.save(value).map_err(|e| e.replace("QQ", "酷狗"))
    }
    /// 每次取链接读取最新登录态，退出后不再使用旧令牌。
    pub fn request_params(&self) -> Result<Value, String> {
        let state = self.load()?;
        let mut params = state.get("device").cloned().unwrap_or(json!({}));
        if let Some(auth) = state["auth"].as_object() {
            params
                .as_object_mut()
                .ok_or("酷狗设备信息损坏")?
                .extend(auth.clone());
        }
        Ok(params)
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualCredentials {
    pub userid: String,
    pub token: String,
    #[serde(default)]
    pub mid: String,
    #[serde(default)]
    pub dfid: String,
}

struct Session {
    key: String,
    path: PathBuf,
    generation: u64,
    device: Value,
    created: Instant,
}
// 同时保护会话、设备初始化和凭据变更；网络请求期间释放锁。
static SESSIONS: Lazy<Mutex<HashMap<String, Session>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// 通过账号歌单接口验证当前令牌，只请求一条；不将保存文件的存在当作在线登录成功。
pub async fn check_account(store: &Store) -> Result<(), String> {
    let mut params = store.request_params()?;
    let body = json!({"userid":number(&params["userid"]),"token":params["token"],"total_ver":979,"type":2,"page":1,"pagesize":1});
    params["plat"] = json!(1);
    let result = super::client::request(
        "/v7/get_all_list",
        "cloudlist.service.kugou.com",
        params,
        Some(body),
    )
    .await?;
    if !result["data"]["info"].is_array() {
        return Err("酷狗账号校验响应缺少列表".into());
    }
    Ok(())
}

fn device(state: &mut Value) -> Result<Value, String> {
    if state.get("device").is_none() {
        state["device"] = json!({"mid":crate::platforms::get_guid().to_lowercase(),"dfid":"-","dev":crate::platforms::get_guid().to_lowercase()});
    }
    if !state["device"].is_object() {
        return Err("酷狗设备信息损坏".into());
    }
    Ok(state["device"].clone())
}

fn public_status(state: &Value) -> Value {
    let userid = text(&state["auth"]["userid"]);
    json!({"logged_in":!userid.is_empty() && state["auth"]["token"].as_str().is_some_and(|s|!s.is_empty()),"userid":userid})
}

fn qr_image(url: &str) -> Result<String, String> {
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
    let width = code.width();
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/><g fill=\"black\">",width+8,width+8);
    for (index, color) in code.to_colors().iter().enumerate() {
        if *color == qrcode::Color::Dark {
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\"/>",
                index % width + 4,
                index / width + 4
            ));
        }
    }
    svg.push_str("</g></svg>");
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg)
    ))
}

pub async fn execute(
    store: &Store,
    action: &str,
    id: &str,
    input: Option<ManualCredentials>,
) -> Result<Value, String> {
    match action {
        "status" => Ok(public_status(&store.load()?)),
        "manual" => {
            let input = input.ok_or("缺少酷狗凭据")?;
            let userid = input.userid.trim();
            let token = input.token.trim();
            if userid.parse::<u64>().ok().filter(|v| *v > 0).is_none()
                || token.is_empty()
                || token.len() > 4096
                || token.chars().any(char::is_control)
            {
                return Err("请填写有效的酷狗 userid 和 token".into());
            }
            for value in [&input.mid, &input.dfid] {
                if value.len() > 128
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                {
                    return Err("设备信息只能包含字母、数字或短横线".into());
                }
            }
            let mut sessions = SESSIONS.lock().await;
            let mut state = store.load()?;
            device(&mut state)?;
            if !input.mid.is_empty() {
                state["device"]["mid"] = json!(input.mid);
            }
            if !input.dfid.is_empty() {
                state["device"]["dfid"] = json!(input.dfid);
            }
            state["auth"] = json!({"userid":userid,"token":token});
            state["generation"] = json!(number(&state["generation"]) + 1);
            store.save(&state)?;
            sessions.retain(|_, s| s.path != store.path);
            Ok(public_status(&state))
        }
        "logout" => {
            let mut sessions = SESSIONS.lock().await;
            let mut state = store.load()?;
            state.as_object_mut().ok_or("酷狗凭据损坏")?.remove("auth");
            state["generation"] = json!(number(&state["generation"]) + 1);
            store.save(&state)?;
            sessions.retain(|_, s| s.path != store.path);
            Ok(public_status(&state))
        }
        "qr" => {
            let (mut params, generation) = {
                let _lock = SESSIONS.lock().await;
                let mut state = store.load()?;
                let params = device(&mut state)?;
                store.save(&state)?;
                (params, number(&state["generation"]))
            };
            let device = params.clone();
            params.as_object_mut().unwrap().extend(json!({"appid":1001,"type":1,"plat":4,"qrcode_txt":"https://h5.kugou.com/apps/loginQRCode/html/index.html?appid=1005&","srcappid":2919}).as_object().unwrap().clone());
            let result = request_with(
                &CLIENT,
                "https://login-user.kugou.com",
                "/v2/qrcode",
                "",
                params,
                None,
            )
            .await?;
            let key = text(&result["data"]["qrcode"]);
            if key.is_empty() {
                return Err("酷狗未返回二维码标识".into());
            }
            let session_id = crate::platforms::get_guid();
            let mut url =
                reqwest::Url::parse("https://h5.kugou.com/apps/loginQRCode/html/index.html")
                    .unwrap();
            url.query_pairs_mut().append_pair("qrcode", &key);
            let image = qr_image(url.as_str())?;
            let mut sessions = SESSIONS.lock().await;
            sessions.retain(|_, s| {
                s.created.elapsed() < Duration::from_secs(300) && s.path != store.path
            });
            sessions.insert(
                session_id.clone(),
                Session {
                    key,
                    path: store.path.clone(),
                    generation,
                    device,
                    created: Instant::now(),
                },
            );
            Ok(json!({"qrcode_id":session_id,"qr_image":image}))
        }
        "check" => {
            let (key, generation, mut params) = {
                let sessions = SESSIONS.lock().await;
                let Some(session) = sessions.get(id).filter(|s| {
                    s.path == store.path && s.created.elapsed() < Duration::from_secs(300)
                }) else {
                    return Ok(json!({"status":"expired"}));
                };
                (
                    session.key.clone(),
                    session.generation,
                    session.device.clone(),
                )
            };
            params.as_object_mut().unwrap().extend(
                json!({"appid":1005,"srcappid":2919,"plat":4,"qrcode":key})
                    .as_object()
                    .unwrap()
                    .clone(),
            );
            let result = request_with(
                &CLIENT,
                "https://login-user.kugou.com",
                "/v2/get_userinfo_qrcode",
                "",
                params,
                None,
            )
            .await?;
            let status = match number(&result["data"]["status"]) {
                1 => "waiting",
                2 => "scanned",
                4 => "confirmed",
                _ => "expired",
            };
            let mut sessions = SESSIONS.lock().await;
            if !sessions.contains_key(id) {
                return Ok(json!({"status":"expired"}));
            }
            if status == "confirmed" {
                let userid = text(&result["data"]["userid"]);
                let token = text(&result["data"]["token"]);
                if userid.parse::<u64>().unwrap_or(0) == 0 || token.is_empty() {
                    return Err("酷狗登录响应缺少凭据".into());
                }
                let mut state = store.load()?;
                if number(&state["generation"]) != generation {
                    sessions.remove(id);
                    return Ok(json!({"status":"expired"}));
                }
                state["auth"] = json!({"userid":userid,"token":token});
                state["generation"] = json!(generation + 1);
                store.save(&state)?;
                sessions.remove(id);
                return Ok(json!({"status":status,"userid":userid}));
            }
            if status == "expired" {
                sessions.remove(id);
            }
            Ok(json!({"status":status}))
        }
        _ => Err("未知酷狗登录操作".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn credentials_are_private_and_logout_preserves_device() {
        let path = std::env::temp_dir().join(format!(
            "hotdownloader-kugou-{}.json",
            crate::platforms::get_guid()
        ));
        let store = Store::new(&path);
        let result = execute(
            &store,
            "manual",
            "",
            Some(ManualCredentials {
                userid: "123".into(),
                token: "test-private-token".into(),
                mid: "same-device".into(),
                dfid: "test-dfid".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(result["logged_in"], true);
        assert!(!result.to_string().contains("test-private-token"));
        let params = store.request_params().unwrap();
        assert_eq!(params["mid"], "same-device");
        assert_eq!(params["userid"], "123");
        assert_eq!(params["token"], "test-private-token");
        let result = execute(&store, "logout", "", None).await.unwrap();
        assert_eq!(result["logged_in"], false);
        let params = store.request_params().unwrap();
        assert_eq!(params["mid"], "same-device");
        assert!(params.get("token").is_none());
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    #[ignore = "手动验证真实酷狗二维码创建和待扫码状态"]
    async fn live_qr_login() {
        let path = std::env::temp_dir().join(format!(
            "hotdownloader-kugou-qr-{}.json",
            crate::platforms::get_guid()
        ));
        let store = Store::new(&path);
        let result = execute(&store, "qr", "", None).await;
        let check = match &result {
            Ok(value) => execute(&store, "check", value["qrcode_id"].as_str().unwrap(), None).await,
            Err(error) => Err(error.clone()),
        };
        let _ = execute(&store, "logout", "", None).await;
        let _ = std::fs::remove_file(path);
        assert!(result.unwrap()["qr_image"]
            .as_str()
            .unwrap()
            .starts_with("data:image/svg+xml;base64,"));
        assert_eq!(check.unwrap()["status"], "waiting");
    }
}
