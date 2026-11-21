# 🎵 HotDownloader-docker

> 基于原项目 [lerdb/HotDownloader](https://github.com/lerdb/HotDownloader)
> 保留上游共享 Rust 下载核心与 Vue 3 前端，**Docker 部署**。

![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.77.2+-orange.svg)
![Node](https://img.shields.io/badge/node-22.12+-green.svg)

---

## 🏗️ 架构（单镜像 / 单容器）

Rust 服务同时托管 Web 页面与 API，浏览器只访问一个端口，无需 nginx，也无需分别部署 web / server。

```
                   浏览器
                      │ HTTP
                      ▼
        ┌──────────────────────────────────┐
        │  HotDownloader（单容器 :8787）     │
        │                                  │
        │  Rust 服务（crates/hotdownloader-server）
        │    ├── /             → Web 静态资源 (dist)
        │    ├── /api/music/*  → 搜索/歌单/专辑/歌手/歌词
        │    ├── /api/tasks*   → 任务创建/暂停/恢复/删除/重试
        │    ├── /api/settings → 设置（字段级合并）
        │    ├── /api/logs     → 运行日志（服务端）
        │    ├── /api/login/*  → 扫码/手动登录
        │    ├── /api/events   → SSE 任务/设置实时推送
        │    └── /healthz      → 健康检查
        │                                  │
        │  共享核心（crates/hotdownloader-core）│
        │    · 下载引擎（并发/断点续传/降级）  │
        │    · 平台实现       │
        └──────────────────────────────────┘
                      │
                      ▼
                 /data  /downloads
```

- **后端**：`crates/hotdownloader-core`（共享核心）+ `crates/hotdownloader-server`（HTTP/SSE 服务），与上游 v2.0.0 保持一致。
- **前端**：Vue 3 + Naive UI，「温石纸面」浅色主题（`src/style.css`、`src/config/theme.ts`、`src/components/NavLayout.vue`）；解析/下载/连接事件写入独立日志页（`src/stores/logStore.ts`、`src/views/LogView.vue`）。
- **访问认证**：对外监听时必须设置用户名密码或令牌（见下）。

---
| 总览 | 下载任务 | 搜索 | 曲库 |
| --- | --- | --- | --- |
| <img src="image/总览.png"> | <img src="image/下载任务.png"> | <img src="image/搜索.png"> | <img src="image/曲库.png"> |

| 日志 | 账号 | 设置 | 登录 |
| --- | --- | --- | --- |
| <img src="image/日志.png"> | <img src="image/账号.png"> | <img src="image/设置.png"> | <img src="image/登录.png"> |

## 🆕 界面与交互更新

- **接入汽水音乐**：粘贴单曲分享链接（含 `qishui.douyin.com` 短链）即可解析歌名、封面、逐字歌词并下载 320k / 128k / 64k AAC；会员曲目匿名只下发 60 秒试听（解析页会明确提示），关键词搜索需在「账号设置 → 汽水音乐」填入 Cookie。
- **全新「纸面」视觉**：暖灰画布 + 纯白卡片 + 近黑主操作，统一圆角、细描边与等宽数字，去掉花哨渐变。
- **总览页重做**：四格关键指标、近 7 天完成/失败趋势细柱图、最近下载列表与曲库构成。
- **独立日志页**：解析、下载、账号、连接事件统一留痕，可按「解析 / 下载 / 仅失败」筛选，**只保留最近 200 条**，超出自动丢弃；日志由服务端环形缓冲保存到 `/data/logs.json`，换设备、清缓存或更换访问端口后仍在（桌面端没有独立服务，仍写本机）。
- **任务列表最新在前**：新加入的任务永远排在列表顶部，各状态标签一致生效。
- **下载不再打断操作**：移除「添加任务后跳转至任务页」开关，下载后停留在当前页面继续挑歌。
- **品牌标识统一**：站点图标、侧栏与登录页统一为 H 标记，Web 访问页重排为纸面卡片。

---

## 🚀 Docker 部署（推荐）

前置要求：Docker ≥ 24、Docker Compose v2。

### 1. 获取源码

```bash
git clone https://github.com/adaier1/hotdownloaderdocker.git
cd hotdownloaderdocker
```

### 2. 登录凭据

**默认账号：`admin` / `admin123`**。登录后可在 **设置 → 安全设置 → 修改密码** 中修改，
结果保存到 `/data/auth.json`，重启后仍然有效。

### 3. Compose

```bash
services:
  hotdownloader:
    image: ghcr.io/adaier1/hotdownloaderdocker:latest
    container_name: hotdownloader
    restart: unless-stopped

    ports:
      - "8080:8787"

    environment:
      HOTDOWNLOADER_BIND: "0.0.0.0:8787"
      HOTDOWNLOADER_DATA_DIR: "/data"
      HOTDOWNLOADER_WEB_DIR: "/app/dist"
      HOTDOWNLOADER_DOWNLOAD_DIR: "/downloads"

      AUTH_USERNAME: "admin"
      AUTH_PASSWORD: "admin123"

      TZ: "Asia/Shanghai"
      RUST_LOG: "info"

    volumes:
      - ./data:/data
      - ./downloads:/downloads
```

### 4. 访问

打开浏览器访问 **http://<主机IP>:8080**，使用默认账号 **`admin` / `admin123`**

### 环境变量

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `HOTDOWNLOADER_BIND` | `0.0.0.0:8787` | 服务监听地址（容器内） |
| `HOTDOWNLOADER_DATA_DIR` | `/data` | 数据目录（任务/设置/凭据） |
| `HOTDOWNLOADER_WEB_DIR` | `/app/dist` | Web 静态资源目录 |
| `HOTDOWNLOADER_DOWNLOAD_DIR` | `/data/downloads` | 音乐下载目录（compose 中设为 `/downloads`） |
| `AUTH_USERNAME` / `AUTH_PASSWORD` | `admin` / `admin123` | 初始用户名密码（仅 `auth.json` 不存在时生效） |
| `HOTDOWNLOADER_TOKEN` | 空 | 令牌认证（≥16 字符，与用户名密码二选一） |
| `RUST_LOG` | `info` | 日志级别 |
| `WEB_PORT` | `8080` | 宿主机端口（compose） |
| `DATA_PATH` | `./data` | 宿主机数据目录（compose） |
| `DOWNLOAD_PATH` | `./downloads` | 宿主机下载目录（compose） |
| `TZ` | `Asia/Shanghai` | 时区（compose） |

### 持久化文件

| 路径 | 内容 |
| --- | --- |
| `/data/settings.json` | 用户设置 |
| `/data/tasks.json` | 下载任务 |
| `/data/auth.json` | 访问账号密码（界面修改密码后写入） |
| `/data/mcp.json` | MCP 接入密钥（设置页可重置） |
| `/data/shortcut.json` | 快捷指令密钥（设置页可重置） |
| `/data/notify.json` | 通知配置（飞书 FSKEY） |
| `/data/qq-credentials.json` | QQ 音乐登录凭据 |
| `/downloads/` | 下载的音乐文件 |

> 从旧版 Web 版（axum `/api/invoke`）升级：旧 `/data/data.json` 不会自动读取。可将其中的 `settings`、`tasks` 字段分别写成 `settings.json`、`tasks.json`（旧任务记录字段与 v2.0.0 兼容）。搜索历史为浏览器本地数据，存于 `localStorage`。

---

## ⚡ 快捷指令（iOS 捷径 / 脚本）

把分享链接交给手机上的「分享 → 运行捷径」，服务端解析后**直接创建下载任务**，不需要打开网页逐首确认。
入口在 **设置 → 快捷指令**（Web 版），可复制调用地址、重置 Key。

### 调用地址

```
http://<主机IP>:8080/api/shortcut/parse?key=<密钥>&url=<URL 编码后的分享链接>
```

- 只接受 `GET`，其它方法返回 `405`。
- `key` 由服务端生成并保存在 `/data/shortcut.json`，点击「重置 Key」后旧地址立即失效。
- 该接口用 `key` 认证，**不经过用户名密码**，所以这条地址等同于凭据，别发到群里或截图外传。

### 参数

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `key` | 是 | 快捷指令密钥，见设置页 |
| `url` | 是 | 分享链接或整段分享文案，需整体 URL 编码（`:` `/` `?` `&` 都要转义） |
| `limit` | 否 | 单次最多创建的任务数，默认 `20`，范围 `1–100` |

### 返回

```json
{
  "ok": true,
  "message": "已创建 12 个下载任务",
  "platform": "qqmusic",
  "kind": "playlist",
  "total": 48,
  "createdCount": 12,
  "failedCount": 0,
  "songs": [{ "taskId": "…", "title": "晴天", "artist": "周杰伦", "quality": "flac" }],
  "failed": []
}
```

- `message` 可以直接放进通知或弹窗；`songs` 是已创建的任务（带 `existing: true` 表示命中已有任务，没有重复建单）。
- 音质遵循设置里的「默认音质」；该档不可用时按任务规则自动降级。设置为「每次询问」时，取该曲最高可用音质（捷径没有选择界面）。
- 重名文件自动「保留两份」，不会停在确认弹窗上。

### iOS 捷径搭建

1. 新建捷径，接收分享表单的 `URL`（或用「获取剪贴板」）。
2. 添加「URL 编码」动作，把分享链接编码。
3. 添加「获取 URL 内容」：
   - 地址：`http://<主机IP>:8080/api/shortcut/parse?key=<密钥>&url=` 拼接第 2 步结果
   - 方法：`GET`
4. 添加「获取字典值」，取 `message`，再用「显示通知」或「显示提醒」输出结果。

命令行验证：

```bash
curl "http://127.0.0.1:8080/api/shortcut/parse?key=<密钥>&url=https%3A%2F%2Fy.qq.com%2Fn%2Fryqq%2Fplaylist%2F123456"
```

---

## 🤖 MCP（Model Context Protocol）

服务在**同一个端口**上额外提供 MCP Streamable HTTP 入口，供支持 MCP 的 AI 客户端调用；它独立于原有 Web/API，不影响既有功能、Docker 单镜像与上游核心。

- 端点：`POST /mcp`（JSON-RPC 2.0；通知类消息返回 202，其它方法返回 405）
- 设置页：**设置 → MCP 接入 → MCP 接入配置**，可查看/复制带密钥的完整链接并重置密钥（密钥存于 `/data/mcp.json`）
- 认证：二选一
  - URL 携带密钥：`http://<主机IP>:8080/mcp?key=<密钥>`
  - 请求头：`Authorization: Basic base64(用户名:密码)`，或 `Authorization: Bearer <HOTDOWNLOADER_TOKEN>`（令牌模式）

提供的工具（共 32 个）：

- 搜索：`search_songs`、`search_albums`、`search_artists`、`search_playlists`、`fetch_album_songs`、`fetch_artist_songs`、`fetch_artist_albums`、`fetch_playlist_songs`、`fetch_hot_keywords`、`fetch_suggestions`、`get_lyric_by_id`
- 任务：`list_tasks`、`create_download_task`、`pause_task`、`resume_task`、`retry_task`、`remove_task`、`remove_tasks`
- 设置：`get_settings`、`patch_settings`、`get_default_download_dir`、`get_login_status`
- 曲库：`list_library`、`delete_library_files`、`rename_library_file`、`write_library_metadata`
- 账号：`list_accounts`、`start_qr_login`、`check_qr_login`、`save_cookie`、`login_with_credentials`、`logout_account`

### 账号：在对话里扫码登录 / 写 Cookie

MCP 侧的登录流程与网页一致，但二维码是**直接作为图片返回**的，可以原样转发到聊天窗口让用户扫：

1. `start_qr_login`（`platform` = `qqmusic` / `netease` / `kugou`）→ 返回 PNG 图片内容、`sessionId`、`imagePath`、`imageUrl`（`/api/login/qr-image/<platform>.png`，需与 `/mcp` 相同的认证头）。
2. 用户用手机 App 扫码；期间每 2 秒调一次 `check_qr_login`（`platform` + `sessionId`），状态依次是 `waiting` → `scanned` → `confirmed`，`done: true` 时停止。
3. `confirmed` 时凭据已自动写入（QQ 的扫码接口只回传令牌，MCP 负责落库），随后搜索/下载立即带登录态。

其它入口：

- `save_cookie`：把浏览器复制的 Cookie 直接写入网易云或汽水（汽水未配置 Cookie 时搜索不可用）。
- `login_with_credentials`：QQ 用 `uin` + `authst`，酷狗用 `userid` + `token`，适合已经有令牌的场景。
- `list_accounts` / `logout_account`：查看四个平台登录态、退出登录。

> 二维码图片固定写在数据目录的 `qr/<platform>.png`，重复扫码只覆盖旧图；有效期约 3 分钟（酷狗 5 分钟），过期后重新调用 `start_qr_login`。


客户端配置示例（远程 HTTP，推荐直接用设置页生成带密钥的链接）：

```json
{
  "mcpServers": {
    "hotdownloader": {
      "type": "http",
      "url": "http://<主机IP>:8080/mcp?key=<设置页中的密钥>"
    }
  }
}
```

也可用请求头方式（将 `用户名:密码` 做 Base64）：

```json
{
  "mcpServers": {
    "hotdownloader": {
      "type": "http",
      "url": "http://<主机IP>:8080/mcp",
      "headers": { "Authorization": "Basic <base64(用户名:密码)>" }
    }
  }
}
```

---

## 🔔 通知设置（飞书机器人）

在 **设置 → 通知设置** 中配置：

- **启用飞书通知**：开关，开启后任务完成/失败时推送。
- **FSKEY**：飞书自定义机器人的 FSKEY（也可直接粘贴完整 Webhook 地址）。
- **发送测试**：保存配置并立即发送一条测试消息，用于验证。

获取 FSKEY：飞书群 → 设置 → 群机器人 → 添加「自定义机器人」，Webhook 形如
`https://open.feishu.cn/open-apis/bot/v2/hook/<FSKEY>`，其中 `<FSKEY>` 即所需值。

---

## 🛠️ 本地开发

```bash
# 1) 启动独立服务（默认监听 127.0.0.1:8787）
HOTDOWNLOADER_BIND=0.0.0.0:8787 \
HOTDOWNLOADER_DATA_DIR=/tmp/hd-data \
HOTDOWNLOADER_WEB_DIR=./dist \
cargo run --manifest-path crates/hotdownloader-server/Cargo.toml

# 2) 启动前端开发服务器（vite 会把 /api 代理到服务）
npm install
npm run dev
```

---


## 📄 许可证

本项目基于 [Apache License 2.0](LICENSE) 开源，第三方组件许可见 `NOTICE` 与 `THIRD_PARTY_LICENSES.txt`。

---

## ⚠️ 免责声明

**HotDownloader 仅用于学习和研究目的。** 用户需自行承担使用本软件所带来的法律责任。
请确保你下载的音乐文件拥有合法的使用权，遵守相关音乐平台的版权规定。本项目开发者不对任何侵权行为负责。
