# 🎵 HotDownloader-docker

> 基于 **Web 前端（Vue 3）+ Rust API 服务（axum）** 架构的音乐下载器，支持 **Docker 一键部署**。
> 原项目作者链接https://github.com/lerdb/HotDownloader#-hotdownloader

![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)
![Node](https://img.shields.io/badge/node-22+-green.svg)

原项目为 Tauri 2 桌面应用，本仓库已重构为 **Web 前端 + Rust API 服务** 架构并容器化：

```
┌──────────────┐      /api/      ┌──────────────────────────┐
│  浏览器       │  ────────────►  │ Rust API 服务（axum）      │
│  Vue 3 SPA   │  ◄────────────  │ · 搜索/歌单/登录/下载链接   │
│  (nginx)     │    SSE 事件推送  │ · 下载引擎（并发/断点续传）  │
└──────────────┘                 │ · 存储（设置/历史/任务）    │
                                 │ · 文件下载接口              │
                                 └──────────────────────────┘
```

- **Web 前端**：Vue 3 + TypeScript + Vite + Pinia + Naive UI，由 nginx 托管静态资源并把 `/api` 反向代理到 Rust 服务（同源，无 CORS 问题）。
- **Rust API 服务**：axum + Tokio，`POST /api/invoke` 统一 RPC 分发（命令与参数约定与 Tauri invoke 一致），`GET /api/events` 通过 SSE 实时推送下载进度/完成/错误事件，`GET /api/files` 提供已下载文件的浏览器下载。

---

## ✨ 特性

- 🔍 **音乐搜索**：关键词搜索、搜索建议、热搜词，支持搜索结果分页加载更多
- 📋 **歌单导入**：支持音乐歌单链接/ID 导入并批量下载
- ⬇️ **智能下载**：多任务并发、断点续传、链接过期自动重试，下载链接实时获取
- 🔄 **自动降级**：指定音质不可用时按预设顺序自动降级
- 📊 **实时速度**：任务列表显示实时下载速度（SSE 推送）
- 🎵 **音频解密**：支持加密格式音频解密（mgg/mflac）
- 🎤 **歌词与标签**：获取歌词并写入音频文件标签（支持封面、歌词），可独立下载 `.lrc` 歌词文件
- 🔐 **账号登录**：支持扫码登录和手动输入 `uin` / `authst` 登录，解锁会员歌曲与更高音质
- 🔁 **重复文件处理**：下载前检测同名文件，支持询问、覆盖、保留两份、取消四种策略
- 📋 **任务管理**：等待/下载/暂停/完成/错误状态分类，支持批量删除、重试、取消、恢复；完成后可一键下载文件到本地浏览器
- ⚙️ **个性化设置**：默认音质、自动降级、文件命名模板、并发数、自动跳转任务页、写入歌曲标签、保存 LRC 歌词等
- 🎨 **深色模式**：跟随系统主题
- 💾 **服务端持久化**：任务、设置、搜索历史、登录状态保存在服务端（Docker 卷），浏览器关掉也不丢

---

## 🖥️ 技术栈

| 前端                        | 后端                        |
| --------------------------- | --------------------------- |
| Vue 3 (Composition API)     | Rust (axum + Tokio)         |
| TypeScript                  | Reqwest (HTTP 客户端)       |
| Vite                        | lofty (音频标签写入)        |
| Pinia                       | rumqttc (MQTT 登录)         |
| Vue Router (Hash 模式)      | umc_qmc (QQ 加密音频解密)   |
| Naive UI                    | SSE 事件推送                |
| nginx (静态托管 + 反向代理)  | JSON 文件持久化（/data）     |

---

## 🚀 Docker 部署（推荐）

前置要求：Docker ≥ 24、Docker Compose v2。

### Compose
# 🎵 HotDownloader-docker

> 基于 **Web 前端（Vue 3）+ Rust API 服务（axum）** 架构的音乐下载器，支持 **Docker 一键部署**。
> 原项目作者链接https://github.com/lerdb/HotDownloader#-hotdownloader

![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)
![Node](https://img.shields.io/badge/node-22+-green.svg)

原项目为 Tauri 2 桌面应用，本仓库已重构为 **Web 前端 + Rust API 服务** 架构并容器化：

```
┌──────────────┐      /api/      ┌──────────────────────────┐
│  浏览器       │  ────────────►  │ Rust API 服务（axum）      │
│  Vue 3 SPA   │  ◄────────────  │ · 搜索/歌单/登录/下载链接   │
│  (nginx)     │    SSE 事件推送  │ · 下载引擎（并发/断点续传）  │
└──────────────┘                 │ · 存储（设置/历史/任务）    │
                                 │ · 文件下载接口              │
                                 └──────────────────────────┘
```

- **Web 前端**：Vue 3 + TypeScript + Vite + Pinia + Naive UI，由 nginx 托管静态资源并把 `/api` 反向代理到 Rust 服务（同源，无 CORS 问题）。
- **Rust API 服务**：axum + Tokio，`POST /api/invoke` 统一 RPC 分发（命令与参数约定与 Tauri invoke 一致），`GET /api/events` 通过 SSE 实时推送下载进度/完成/错误事件，`GET /api/files` 提供已下载文件的浏览器下载。

---

## ✨ 特性

- 🔍 **音乐搜索**：关键词搜索、搜索建议、热搜词，支持搜索结果分页加载更多
- 📋 **歌单导入**：支持音乐歌单链接/ID 导入并批量下载
- ⬇️ **智能下载**：多任务并发、断点续传、链接过期自动重试，下载链接实时获取
- 🔄 **自动降级**：指定音质不可用时按预设顺序自动降级
- 📊 **实时速度**：任务列表显示实时下载速度（SSE 推送）
- 🎵 **音频解密**：支持加密格式音频解密（mgg/mflac）
- 🎤 **歌词与标签**：获取歌词并写入音频文件标签（支持封面、歌词），可独立下载 `.lrc` 歌词文件
- 🔐 **账号登录**：支持扫码登录和手动输入 `uin` / `authst` 登录，解锁会员歌曲与更高音质
- 🔁 **重复文件处理**：下载前检测同名文件，支持询问、覆盖、保留两份、取消四种策略
- 📋 **任务管理**：等待/下载/暂停/完成/错误状态分类，支持批量删除、重试、取消、恢复；完成后可一键下载文件到本地浏览器
- ⚙️ **个性化设置**：默认音质、自动降级、文件命名模板、并发数、自动跳转任务页、写入歌曲标签、保存 LRC 歌词等
- 🎨 **深色模式**：跟随系统主题
- 💾 **服务端持久化**：任务、设置、搜索历史、登录状态保存在服务端（Docker 卷），浏览器关掉也不丢

---

## 🖥️ 技术栈

| 前端                        | 后端                        |
| --------------------------- | --------------------------- |
| Vue 3 (Composition API)     | Rust (axum + Tokio)         |
| TypeScript                  | Reqwest (HTTP 客户端)       |
| Vite                        | lofty (音频标签写入)        |
| Pinia                       | rumqttc (MQTT 登录)         |
| Vue Router (Hash 模式)      | umc_qmc (QQ 加密音频解密)   |
| Naive UI                    | SSE 事件推送                |
| nginx (静态托管 + 反向代理)  | JSON 文件持久化（/data）     |

---

## 🚀 Docker 部署（推荐）

前置要求：Docker ≥ 24、Docker Compose v2。

### Compose

```bash
# 可覆盖的环境变量：
# IMAGE_TAG     镜像标签，默认 latest（也可 main / sha-xxxxxxx / v1.2.3）
# WEB_PORT      宿主机端口，默认 8080
# DOWNLOAD_PATH 音乐保存目录
# DATA_PATH     data保存目录
# TZ            时区，默认 Asia/Shanghai
# RUST_LOG      日志级别，默认 info

services:

# Rust API 服务（下载引擎、搜索/歌单/登录接口、SSE 事件推送）
server:
  image: ghcr.io/adaier1/hotdownloaderdocker-server:${IMAGE_TAG:-latest}
  restart: unless-stopped
  environment:
    DATA_DIR: /data
    DOWNLOAD_DIR: /downloads
    BIND_ADDR: 0.0.0.0:8080
    RUST_LOG: ${RUST_LOG:-info}
    TZ: ${TZ:-Asia/Shanghai}
  volumes:
    # 配置/任务/登录态（务必持久化，否则重启丢失）
    - ${DATA_PATH:-./data}:/data
    # 下载的音乐文件目录
    - ${DOWNLOAD_PATH:-./downloads}:/downloads

# Web 前端（nginx 静态托管 + /api 反向代理到 server）
web:
  image: ghcr.io/adaier1/hotdownloaderdocker-web:${IMAGE_TAG:-latest}
  restart: unless-stopped
  ports:
    - "${WEB_PORT:-8080}:80"
  environment:
    TZ: ${TZ:-Asia/Shanghai}
  depends_on:
    - server

```

### 1. 获取源码

```bash
git clone https://github.com/lerdb/HotDownloader.git
cd HotDownloader
```

> `libs/um_crypto` 已随仓库直接分发（非 git 子模块），无需额外初始化。

### 2. 构建并启动

```bash
docker compose up -d --build
```

### 3. 访问

打开浏览器访问 **http://localhost:8080**。

- 修改监听端口：编辑 `docker-compose.yml` 中 `web` 服务的 `ports: ["8080:80"]`。
- 数据持久化：`downloader-data`（设置/历史/任务）与 `downloader-music`（下载的音乐）两个 Docker 卷，删除容器不丢失。
- 音乐文件所在位置：卷 `downloader-music` 挂载于服务端 `/downloads`，可 `docker volume inspect` 查看宿主机路径。

### 4. 常用命令

```bash
docker compose ps             # 查看状态
docker compose logs -f server # 查看 API 服务日志
docker compose down           # 停止（数据保留在卷中）
docker compose up -d --build  # 更新代码后重新构建
```

### 服务端环境变量

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DATA_DIR` | `/data` | 数据持久化目录（设置/历史/任务） |
| `DOWNLOAD_DIR` | `/downloads` | 音乐下载目录 |
| `BIND_ADDR` | `0.0.0.0:8080` | API 监听地址 |
| `RUST_LOG` | `info` | 日志级别 |

---

## 🛠️ 本地开发

```bash
# 1. 启动 Rust API 服务（默认监听 0.0.0.0:8080）
DATA_DIR=/tmp/hd-data DOWNLOAD_DIR=/tmp/hd-music RUST_LOG=debug \
  cargo run --manifest-path server/Cargo.toml

# 2. 启动前端开发服务器（/api 自动代理到 http://localhost:8080）
npm install
npm run dev
```

打开 http://localhost:5173 即可。若 API 服务不在 8080，可用 `VITE_PROXY_TARGET` 指定：

```bash
VITE_PROXY_TARGET=http://localhost:9090 npm run dev
```

### 前端生产构建

```bash
npm run build    # 产物输出到 dist/
```

---

## 📁 项目结构

```
HotDownloader/
├── src/                 # Web 前端（Vue 3 + TS + Vite）
│   └── api/client.ts    # Tauri invoke/listen 的 HTTP + SSE 替代层
├── server/              # Rust API 服务（axum）
│   ├── Cargo.toml
│   ├── src/main.rs      # 服务入口（路由注册、引擎启动）
│   ├── src/api.rs       # /api/invoke RPC 分发、SSE、文件下载、服务信息
│   ├── src/ctx.rs       # AppCtx（存储 + 事件总线，替代 Tauri AppHandle）
│   ├── src/commands/    # 命令层（搜索/下载/任务/设置/登录…）
│   ├── src/download/    # 下载引擎（并发调度、断点续传、解密）
│   ├── src/platforms/   # 平台实现（QQ 音乐、酷我）
│   └── src/storage/     # JSON 文件持久化
├── libs/um_crypto/      # QQ 加密音频解密库（随仓库分发）
├── Dockerfile.server    # Rust API 服务镜像
├── Dockerfile.web       # Web 前端镜像（nginx）
├── docker-compose.yml   # 一键编排
└── nginx.conf           # 静态托管 + /api 反向代理（SSE 已关缓冲）
```

---

## 🔌 HTTP API 一览

| 端点 | 说明 |
| --- | --- |
| `POST /api/invoke` | 统一 RPC 分发（`{"cmd": "...", "args": {...}}` → `{"code":0,"data":...}`） |
| `GET /api/events` | SSE 事件流（download-progress / download-completed / download-error …） |
| `GET /api/files?name=<文件名>` | 下载服务端已下载的文件（仅允许下载目录内的文件） |
| `GET /api/info` | 服务信息（版本、下载目录） |

主要命令（`/api/invoke`）：`search_songs`、`fetch_download_link`、`fetch_playlist_songs`、`search_playlists`、`fetch_suggestions`、`fetch_hot_keywords`、`get_lyric_by_id`、`check_download_path`、`create_qr_login`、`check_qr_login`、`login_with_uin_authst`、`logout`、`get_login_status`、`load_settings`、`save_settings`、`load_history`、`save_history`、`load_tasks`、`save_tasks`、`add_download_task`、`enqueue_task`、`pause_task`、`resume_task`、`cancel_task`、`remove_tasks`、`set_max_concurrent`、`get_default_download_dir`、`check_update`。

---

## ⚠️ 与桌面版的差异（Web 化适配说明）

| 能力 | 桌面版 | Web 版 |
| --- | --- | --- |
| 事件推送 | Tauri 进程内事件 | SSE 事件流（断线自动重连 + 历史回放） |
| 持久化 | 本地 data.json | 服务端 /data/data.json（Docker 卷） |
| 下载目录 | 用户选择本地目录 | 服务端挂载卷（DOWNLOAD_DIR），前端只读展示 |
| 打开文件位置 | 系统文件管理器 | 浏览器下载到本地（`/api/files`） |
| 系统通知 | Tauri 通知插件 | 浏览器 Notification API |
| 关闭确认 | 拦截窗口关闭 | `beforeunload` 提示（服务端任务不受影响） |

---

## 📄 许可证

[Apache License 2.0](LICENSE)

---

## ⚠️ 免责声明

**HotDownloader 仅用于学习和研究目的。**

用户需自行承担使用本软件所带来的法律责任。请确保你下载的音乐文件拥有合法的使用权，遵守相关音乐平台的版权规定。本项目开发者不对任何侵权行为负责。

"
### 1. 获取源码

```bash
git clone https://github.com/lerdb/HotDownloader.git
cd HotDownloader
```

> `libs/um_crypto` 已随仓库直接分发（非 git 子模块），无需额外初始化。

### 2. 构建并启动

```bash
docker compose up -d --build
```

### 3. 访问

打开浏览器访问 **http://localhost:8080**。

- 修改监听端口：编辑 `docker-compose.yml` 中 `web` 服务的 `ports: ["8080:80"]`。
- 数据持久化：`downloader-data`（设置/历史/任务）与 `downloader-music`（下载的音乐）两个 Docker 卷，删除容器不丢失。
- 音乐文件所在位置：卷 `downloader-music` 挂载于服务端 `/downloads`，可 `docker volume inspect` 查看宿主机路径。

### 4. 常用命令

```bash
docker compose ps             # 查看状态
docker compose logs -f server # 查看 API 服务日志
docker compose down           # 停止（数据保留在卷中）
docker compose up -d --build  # 更新代码后重新构建
```

### 服务端环境变量

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DATA_DIR` | `/data` | 数据持久化目录（设置/历史/任务） |
| `DOWNLOAD_DIR` | `/downloads` | 音乐下载目录 |
| `BIND_ADDR` | `0.0.0.0:8080` | API 监听地址 |
| `RUST_LOG` | `info` | 日志级别 |

---

## 🛠️ 本地开发

```bash
# 1. 启动 Rust API 服务（默认监听 0.0.0.0:8080）
DATA_DIR=/tmp/hd-data DOWNLOAD_DIR=/tmp/hd-music RUST_LOG=debug \
  cargo run --manifest-path server/Cargo.toml

# 2. 启动前端开发服务器（/api 自动代理到 http://localhost:8080）
npm install
npm run dev
```

打开 http://localhost:5173 即可。若 API 服务不在 8080，可用 `VITE_PROXY_TARGET` 指定：

```bash
VITE_PROXY_TARGET=http://localhost:9090 npm run dev
```

### 前端生产构建

```bash
npm run build    # 产物输出到 dist/
```

---

## 📁 项目结构

```
HotDownloader/
├── src/                 # Web 前端（Vue 3 + TS + Vite）
│   └── api/client.ts    # Tauri invoke/listen 的 HTTP + SSE 替代层
├── server/              # Rust API 服务（axum）
│   ├── Cargo.toml
│   ├── src/main.rs      # 服务入口（路由注册、引擎启动）
│   ├── src/api.rs       # /api/invoke RPC 分发、SSE、文件下载、服务信息
│   ├── src/ctx.rs       # AppCtx（存储 + 事件总线，替代 Tauri AppHandle）
│   ├── src/commands/    # 命令层（搜索/下载/任务/设置/登录…）
│   ├── src/download/    # 下载引擎（并发调度、断点续传、解密）
│   ├── src/platforms/   # 平台实现（QQ 音乐、酷我）
│   └── src/storage/     # JSON 文件持久化
├── libs/um_crypto/      # QQ 加密音频解密库（随仓库分发）
├── Dockerfile.server    # Rust API 服务镜像
├── Dockerfile.web       # Web 前端镜像（nginx）
├── docker-compose.yml   # 一键编排
└── nginx.conf           # 静态托管 + /api 反向代理（SSE 已关缓冲）
```

---

## 🔌 HTTP API 一览

| 端点 | 说明 |
| --- | --- |
| `POST /api/invoke` | 统一 RPC 分发（`{"cmd": "...", "args": {...}}` → `{"code":0,"data":...}`） |
| `GET /api/events` | SSE 事件流（download-progress / download-completed / download-error …） |
| `GET /api/files?name=<文件名>` | 下载服务端已下载的文件（仅允许下载目录内的文件） |
| `GET /api/info` | 服务信息（版本、下载目录） |

主要命令（`/api/invoke`）：`search_songs`、`fetch_download_link`、`fetch_playlist_songs`、`search_playlists`、`fetch_suggestions`、`fetch_hot_keywords`、`get_lyric_by_id`、`check_download_path`、`create_qr_login`、`check_qr_login`、`login_with_uin_authst`、`logout`、`get_login_status`、`load_settings`、`save_settings`、`load_history`、`save_history`、`load_tasks`、`save_tasks`、`add_download_task`、`enqueue_task`、`pause_task`、`resume_task`、`cancel_task`、`remove_tasks`、`set_max_concurrent`、`get_default_download_dir`、`check_update`。

---

## ⚠️ 与桌面版的差异（Web 化适配说明）

| 能力 | 桌面版 | Web 版 |
| --- | --- | --- |
| 事件推送 | Tauri 进程内事件 | SSE 事件流（断线自动重连 + 历史回放） |
| 持久化 | 本地 data.json | 服务端 /data/data.json（Docker 卷） |
| 下载目录 | 用户选择本地目录 | 服务端挂载卷（DOWNLOAD_DIR），前端只读展示 |
| 打开文件位置 | 系统文件管理器 | 浏览器下载到本地（`/api/files`） |
| 系统通知 | Tauri 通知插件 | 浏览器 Notification API |
| 关闭确认 | 拦截窗口关闭 | `beforeunload` 提示（服务端任务不受影响） |

---

## 📄 许可证

[Apache License 2.0](LICENSE)

---

## ⚠️ 免责声明

**HotDownloader 仅用于学习和研究目的。**

用户需自行承担使用本软件所带来的法律责任。请确保你下载的音乐文件拥有合法的使用权，遵守相关音乐平台的版权规定。本项目开发者不对任何侵权行为负责。
