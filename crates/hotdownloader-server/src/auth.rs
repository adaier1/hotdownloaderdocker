//! 独立服务的访问认证。配置在启动时确定，所有 API 和 SSE 使用同一种模式。

use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

pub enum AccessAuth {
    None,
    Token(String),
    Password { username: String, password: String },
}

/// `auth.json` 的持久化结构；可被界面「修改密码」功能更新后写回。
#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum StoredAuth {
    None,
    Token { token: String },
    Password { username: String, password: String },
}

/// 未配置任何凭据时的默认账号，便于首次部署直接登录（可在设置页修改）。
pub const DEFAULT_USERNAME: &str = "admin";
pub const DEFAULT_PASSWORD: &str = "admin123";

impl AccessAuth {
    pub fn from_env() -> Result<Self, String> {
        Self::from_values(
            std::env::var("AUTH_USERNAME").unwrap_or_default(),
            std::env::var("AUTH_PASSWORD").unwrap_or_default(),
            std::env::var("HOTDOWNLOADER_TOKEN").unwrap_or_default(),
        )
    }

    /// `auth.json` 在数据目录中的位置。
    pub fn file_path(data_dir: &Path) -> PathBuf {
        data_dir.join("auth.json")
    }

    /// 加载认证配置：`auth.json` 优先，其次环境变量，最后默认 `admin/admin123`。
    /// 采用环境变量或默认值时会写入 `auth.json`，以便后续在界面中修改密码。
    pub fn load_or_default(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|error| format!("创建数据目录失败: {error}"))?;
        let path = Self::file_path(data_dir);
        if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .map_err(|error| format!("读取认证配置失败: {error}"))?;
            let stored: StoredAuth =
                serde_json::from_str(&raw).map_err(|error| format!("解析认证配置失败: {error}"))?;
            return Ok(Self::from_stored(stored));
        }
        // 环境变量未配置任何凭据时，落盘一个默认账号，界面可随后修改。
        let auth = match Self::from_env()? {
            Self::None => Self::Password {
                username: DEFAULT_USERNAME.to_string(),
                password: DEFAULT_PASSWORD.to_string(),
            },
            other => other,
        };
        auth.store(&path)?;
        Ok(auth)
    }

    /// 将当前认证配置写回 `auth.json`。
    pub fn store(&self, path: &Path) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(&self.stored()).map_err(|error| error.to_string())?;
        std::fs::write(path, raw).map_err(|error| format!("写入认证配置失败: {error}"))
    }

    fn from_stored(stored: StoredAuth) -> Self {
        match stored {
            StoredAuth::None => Self::None,
            StoredAuth::Token { token } => Self::Token(token),
            StoredAuth::Password { username, password } => Self::Password { username, password },
        }
    }

    fn stored(&self) -> StoredAuth {
        match self {
            Self::None => StoredAuth::None,
            Self::Token(token) => StoredAuth::Token { token: token.clone() },
            Self::Password { username, password } => StoredAuth::Password {
                username: username.clone(),
                password: password.clone(),
            },
        }
    }

    /// 校验给定密码是否与当前密码一致（用于修改密码时的二次确认）。
    pub fn verify_password(&self, password: &str) -> bool {
        match self {
            Self::Password {
                password: expected, ..
            } => constant_time_eq(expected, password),
            _ => false,
        }
    }

    fn from_values(username: String, password: String, token: String) -> Result<Self, String> {
        if !username.is_empty() || !password.is_empty() {
            if username.is_empty() || password.is_empty() {
                return Err("AUTH_USERNAME 和 AUTH_PASSWORD 必须同时设置".into());
            }
            if username.contains(':') {
                return Err("AUTH_USERNAME 不能包含冒号".into());
            }
            return Ok(Self::Password { username, password });
        }
        if !token.is_empty() {
            Ok(Self::Token(token))
        } else {
            Ok(Self::None)
        }
    }

    pub fn mode(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Token(_) => "token",
            Self::Password { .. } => "password",
        }
    }

    pub fn valid_for_external_access(&self) -> bool {
        match self {
            Self::None => false,
            Self::Token(token) => token.len() >= 16,
            Self::Password { .. } => true,
        }
    }

    pub fn accepts(&self, authorization: Option<&str>) -> bool {
        match self {
            Self::None => true,
            Self::Token(expected) => authorization
                .and_then(|value| value.strip_prefix("Bearer "))
                .is_some_and(|supplied| constant_time_eq(expected, supplied)),
            Self::Password { username, password } => authorization
                .and_then(|value| value.strip_prefix("Basic "))
                .and_then(|encoded| STANDARD.decode(encoded).ok())
                .and_then(|decoded| String::from_utf8(decoded).ok())
                .and_then(|credentials| {
                    credentials
                        .split_once(':')
                        .map(|(name, secret)| (name.to_string(), secret.to_string()))
                })
                .is_some_and(|(name, secret)| {
                    constant_time_eq(username, &name) & constant_time_eq(password, &secret)
                }),
        }
    }
}

fn constant_time_eq(expected: &str, supplied: &str) -> bool {
    let expected = expected.as_bytes();
    let supplied = supplied.as_bytes();
    let mut difference = expected.len() ^ supplied.len();
    for (a, b) in expected.iter().zip(supplied) {
        difference |= usize::from(a ^ b);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::AccessAuth;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    #[test]
    fn password_configuration_takes_priority_over_token() {
        let auth =
            AccessAuth::from_values("admin".into(), "短密码".into(), "old-token".into()).unwrap();
        assert_eq!(auth.mode(), "password");
        assert!(auth.valid_for_external_access());
        assert!(auth.accepts(Some(&format!("Basic {}", STANDARD.encode("admin:短密码")))));
        assert!(!auth.accepts(Some("Bearer old-token")));
        assert!(!auth.accepts(Some(&format!("Basic {}", STANDARD.encode("admin:wrong")))));
    }

    #[test]
    fn incomplete_password_configuration_does_not_fall_back_to_token() {
        assert!(AccessAuth::from_values("admin".into(), "".into(), "old-token".into()).is_err());
        assert!(AccessAuth::from_values("".into(), "secret".into(), "old-token".into()).is_err());
        assert!(AccessAuth::from_values("bad:name".into(), "secret".into(), "".into()).is_err());
    }

    #[test]
    fn token_configuration_remains_supported() {
        let auth =
            AccessAuth::from_values("".into(), "".into(), "1234567890123456".into()).unwrap();
        assert_eq!(auth.mode(), "token");
        assert!(auth.valid_for_external_access());
        assert!(auth.accepts(Some("Bearer 1234567890123456")));
        assert!(!auth.accepts(Some("Bearer wrong")));
        assert!(!auth.accepts(Some(&format!("Basic {}", STANDARD.encode("admin:secret")))));
    }
}
