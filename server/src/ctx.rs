//! 应用上下文（AppCtx）：替代 Tauri 桌面端的 `AppHandle`。
//!
//! 服务端不再依赖 Tauri，因此需要一个统一的对象来承载：
//! - 数据持久化目录（`data.json`，结构与 tauri-plugin-store 兼容）；
//! - 音乐下载目录；
//! - 事件总线（供 `/api/events` 的 SSE 推送与断线重连回放）。
//!
//! `AppCtx` 内部使用 `Arc`，克隆成本极低，可随意在各异步任务间传递。

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;

/// 事件历史回放的最大条数（断线重连时补发，避免错过关键事件）。
const EVENT_HISTORY_LIMIT: usize = 256;
/// 广播通道容量（消费者积压时丢弃最旧事件）。
const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// 事件总线：broadcast 通道 + 最近事件历史。
struct EventBus {
    tx: broadcast::Sender<(String, String)>,
    history: Mutex<VecDeque<(String, String)>>,
}

impl EventBus {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        EventBus {
            tx,
            history: Mutex::new(VecDeque::new()),
        }
    }

    fn publish(&self, event: String, payload: String) {
        {
            let mut history = self.history.lock().unwrap();
            if history.len() >= EVENT_HISTORY_LIMIT {
                history.pop_front();
            }
            history.push_back((event.clone(), payload.clone()));
        }
        // 没有订阅者时发送会失败，属于正常情况，忽略即可
        let _ = self.tx.send((event, payload));
    }

    fn subscribe(&self) -> (broadcast::Receiver<(String, String)>, Vec<(String, String)>) {
        let rx = self.tx.subscribe();
        let history = self.history.lock().unwrap().iter().cloned().collect();
        (rx, history)
    }
}

/// 全局应用上下文。
#[derive(Clone)]
pub struct AppCtx {
    inner: Arc<Inner>,
}

struct Inner {
    data_dir: PathBuf,
    download_dir: PathBuf,
    events: EventBus,
    /// 串行化 `data.json` 的读改写，避免并发写导致数据丢失。
    storage_lock: Mutex<()>,
}

impl AppCtx {
    pub fn new(data_dir: PathBuf, download_dir: PathBuf) -> Self {
        AppCtx {
            inner: Arc::new(Inner {
                data_dir,
                download_dir,
                events: EventBus::new(),
                storage_lock: Mutex::new(()),
            }),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.inner.data_dir
    }

    pub fn download_dir(&self) -> &Path {
        &self.inner.download_dir
    }

    /// 推送一个事件（payload 序列化为 JSON 字符串后经 SSE 下发）。
    pub fn emit<T: Serialize>(&self, event: &str, payload: T) {
        let data = serde_json::to_string(&payload).unwrap_or_else(|_| "null".to_string());
        self.inner.events.publish(event.to_string(), data);
    }

    /// 订阅事件：返回 `(实时接收端, 历史事件快照)`。
    pub fn subscribe(&self) -> (broadcast::Receiver<(String, String)>, Vec<(String, String)>) {
        self.inner.events.subscribe()
    }

    fn store_path(&self) -> PathBuf {
        self.inner.data_dir.join("data.json")
    }

    fn read_store(&self) -> Value {
        let content = std::fs::read_to_string(self.store_path()).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    }

    fn write_store(&self, value: &Value) -> Result<(), String> {
        if let Err(e) = std::fs::create_dir_all(&self.inner.data_dir) {
            return Err(format!("创建数据目录失败: {}", e));
        }
        let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        std::fs::write(self.store_path(), text).map_err(|e| format!("写入数据文件失败: {}", e))
    }

    /// 读取字符串键（兼容原 tauri-plugin-store 的 `data.json` 结构）。
    pub fn load_string(&self, key: &str) -> Result<String, String> {
        let _guard = self.inner.storage_lock.lock().unwrap();
        let store = self.read_store();
        Ok(store
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// 写入字符串键。
    pub fn save_string(&self, key: &str, value: &str) -> Result<(), String> {
        let _guard = self.inner.storage_lock.lock().unwrap();
        let mut store = self.read_store();
        store[key] = Value::String(value.to_string());
        self.write_store(&store)
    }
}
