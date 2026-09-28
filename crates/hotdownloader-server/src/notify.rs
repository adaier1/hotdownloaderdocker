//! 通知推送（飞书机器人）。
//!
//! 配置独立持久化在数据目录 `notify.json`（不混入上游设置，避免改动核心的
//! `settings::patch` 字段白名单）。开启后，任务完成/失败时向飞书机器人推送。
//!
//! 飞书 Webhook 与报文格式参考青龙 `notify.py`：
//! `POST https://open.feishu.cn/open-apis/bot/v2/hook/{FSKEY}`，
//! 报文 `{"msg_type":"text","content":{"text":"标题\n\n内容"}}`，
//! 返回 `StatusCode == 0` 或 `code == 0` 视为成功。

use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// 通知配置（camelCase 与前端一致）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyConfigData {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub fs_key: String,
}

pub struct NotifyConfig {
    path: PathBuf,
    data: RwLock<NotifyConfigData>,
    client: reqwest::Client,
}

impl NotifyConfig {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let path = data_dir.join("notify.json");
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<NotifyConfigData>(&raw).ok())
            .unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|error| format!("创建通知客户端失败: {error}"))?;
        Ok(Self {
            path,
            data: RwLock::new(data),
            client,
        })
    }

    pub fn snapshot(&self) -> NotifyConfigData {
        self.data.read().unwrap().clone()
    }

    pub fn save(&self, data: NotifyConfigData) -> Result<NotifyConfigData, String> {
        *self.data.write().unwrap() = data.clone();
        self.persist()?;
        Ok(data)
    }

    /// 任务完成/失败时调用；未启用或未配置 FSKEY 时静默跳过。
    pub async fn send(&self, title: &str, content: &str) -> Result<(), String> {
        let config = self.snapshot();
        if !config.enabled || config.fs_key.trim().is_empty() {
            return Ok(());
        }
        send_feishu(&self.client, &config.fs_key, title, content).await
    }

    /// 发送测试消息（忽略启用开关，但要求已配置 FSKEY）。
    pub async fn send_test(&self) -> Result<(), String> {
        let config = self.snapshot();
        if config.fs_key.trim().is_empty() {
            return Err("请先填写飞书机器人 FSKEY".into());
        }
        send_feishu(
            &self.client,
            &config.fs_key,
            "HotDownloader 通知测试",
            "这是一条来自 HotDownloader 的测试通知，收到即表示配置成功。",
        )
        .await
    }

    fn persist(&self) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(&self.snapshot()).map_err(|error| error.to_string())?;
        std::fs::write(&self.path, raw).map_err(|error| format!("写入 notify.json 失败: {error}"))
    }
}

/// 兼容「只填 key」与「填完整 Webhook 地址」两种用法。
fn hook_url(fs_key: &str) -> String {
    let key = fs_key.trim();
    if key.starts_with("http://") || key.starts_with("https://") {
        key.to_string()
    } else {
        format!("https://open.feishu.cn/open-apis/bot/v2/hook/{key}")
    }
}

async fn send_feishu(
    client: &reqwest::Client,
    fs_key: &str,
    title: &str,
    content: &str,
) -> Result<(), String> {
    let body = serde_json::json!({
        "msg_type": "text",
        "content": { "text": format!("{title}\n\n{content}") },
    });
    let response = client
        .post(hook_url(fs_key))
        .header("Content-Type", "application/json;charset=utf-8")
        .body(body.to_string())
        .send()
        .await
        .map_err(|error| format!("请求飞书失败: {error}"))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("飞书返回 HTTP {status}: {text}"));
    }
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let ok = value.get("StatusCode").and_then(|value| value.as_i64()) == Some(0)
        || value.get("code").and_then(|value| value.as_i64()) == Some(0);
    if ok {
        Ok(())
    } else {
        Err(format!("飞书返回错误: {text}"))
    }
}
