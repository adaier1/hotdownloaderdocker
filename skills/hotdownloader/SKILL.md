---
name: hotdownloader
display_name: HotDownloader 使用（搜索 / 解析 / 下载）
display_name_en: HotDownloader Usage (search, parse, download)
description: HotDownloader 的日常使用技能：怎么搜索、怎么解析分享链接、怎么建下载任务、怎么管任务与曲库，以及各平台的能力差异和登录前置条件。所有能力都通过 MCP 工具（search_songs / parse_link / create_download_task / list_tasks …）或 `/api/music/*` HTTP 接口调用；服务默认端口取 `.env` 的 `WEB_PORT`。含按平台的搜索能力矩阵、音质标签约定（不是文件名）、批量与降级规则、汽水音乐专项（客户端 Cookie + bdms.node 签名 + 直链投递，见 references/qishui.md）、签名协议（references/signer.md）、以及搜不到/建不了任务/下载失败的排错表。触发词：HotDownloader 怎么用、搜索歌曲、解析分享链接、下载这首歌、建下载任务、批量下载、任务失败、汽水音乐、网易云链接、QQ音乐链接、MCP 工具、qishui、hotdownloader。
version: 1.0.0
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
examples_zh:
  - "帮我在 HotDownloader 里搜一下周杰伦"
  - "这条网易云分享链接帮我在服务端解析并下载"
  - "批量把这张歌单加到下载队列"
  - "汽水音乐怎么登录、怎么下高音质"
---

# HotDownloader：搜索 / 解析 / 下载

所有操作都走两条等价通道：

- **MCP 工具**（agent 用，推荐）：`search_songs`、`parse_link`、`create_download_task`……
- **HTTP 接口**（脚本/调试用）：`POST /api/music/{action}`、`POST /api/tasks`，Basic 认证取 `.env` 的 `AUTH_USERNAME/AUTH_PASSWORD`，端口取 `WEB_PORT`（默认 8791）。

## 开始前：三步确认

```bash
curl -s http://127.0.0.1:8791/healthz              # 1. 服务在跑
```

2. **MCP 是否连通**：调用 `list_accounts`，能返回平台列表即连通。没连通看「MCP 配置」一节。
3. **平台登录态**：同一份 `list_accounts` 结果里看各平台 `loggedIn`／`note`。某些平台不登录就只能拿试听或干脆搜不到（见下方矩阵）。

## 怎么搜索

| 工具 | 用途 | 关键参数 |
| --- | --- | --- |
| `search_songs` | 搜歌曲 | `platform, keyword, page, limit` |
| `search_albums` | 搜专辑 | 同上 |
| `search_artists` | 搜歌手 | 同上 |
| `search_playlists` | 搜歌单 | 同上 |
| `fetch_album_songs` / `fetch_artist_songs` / `fetch_artist_albums` / `fetch_playlist_songs` | 按 ID 或链接取内容 | `platform, id\|input, page, limit` |
| `fetch_hot_keywords` / `fetch_suggestions` | 热搜词 / 联想 | `platform` |

平台能力矩阵（代码里真实存在的分支）：

| 平台 | 歌曲 | 专辑 | 歌手 | 歌单 | 个人歌单 |
| --- | --- | --- | --- | --- | --- |
| netease | ✅ | ✅ | ✅ | ✅ | ✅ |
| qqmusic | ✅ | ✅ | ✅ | ✅ | ✅ |
| kugou | ✅ | ✅ | ✅ | ✅ | ❌ |
| kuwo | ✅ | ✅ | ✅ | ✅ | ❌ |
| qishui | ✅（需签名） | ❌ | ❌ | ❌ | ❌ |

返回结构（`search_songs`）：

```json
{"has_more": false, "songs": [
  {"mid": "6696534426169378817", "title": "布拉格广场", "artist": "蔡依林、周杰伦",
   "album": "看我72变", "coverUrl": "https://…", "duration": 294,
   "qualities": [{"quality": "320kaac", "filename": "320.m4a", "size": 8414936}]}
]}
```

注意：

- **搜索结果里的 `qualities` 可能只是占位档位**（汽水固定给 128kaac）。要准确档位就先 `parse_link` 拿详情。
- 汽水第 2 页起返回空（游标无法由页码推导），其它平台正常分页。
- 汽水搜索依赖本机签名，公网服务器默认搜不了 —— 见 `references/qishui.md`。

## 怎么解析

`parse_link` 接受任意平台的**分享链接、整段分享文案或纯 ID**，平台由内容自身判断，无需先选平台：

```jsonc
// 入参
{"input": "https://music.163.com/song?id=1974443814"}
// 入参也可带兜底平台（链接识别不出来时）
{"input": "1974443814", "platform": "netease"}
```

返回：

```json
{"platform": "netease", "kind": "song",
 "data": {"song": {"mid": "1974443814", "title": "我记得", "artist": "赵雷",
   "qualities": [{"quality": "128kmp3", …}, {"quality": "320kmp3", …}, {"quality": "flac", "size": 65211331}]}}}
```

- `kind` 可能是 `song` / `playlist` / `album` 等，歌单类结果要再用 `fetch_playlist_songs` 或直接遍历。
- 解析结果**可直接喂给 `create_download_task`**，这是最稳的"链接 → 下载"路径。
- 汽水短链（`qishui.douyin.com/s/xxx`）也支持，内部会跟随跳转。

## 怎么下载

`create_download_task` 三个必填项 + 一个可选直链：

```jsonc
{
  "song": { /* parse_link 或 search_songs 返回的歌曲对象，必须含 qualities */ },
  "desiredQuality": "320kaac",          // 传「音质标签」，不是文件名
  "duplicateAction": "rename",          // overwrite / rename / cancel，可省略
  "stream": { "url": "…", "filename": "320.m4a", "size": 8414936 }  // 可选，见下
}
```

**最容易错的一点：`desiredQuality` 要传音质标签**（`320kaac`、`flac`、`320kmp3`），传文件名（`320.m4a`）会在建任务阶段直接判「所选音质不可用」。

返回 `outcome` 语义：

| outcome | 含义 | 下一步 |
| --- | --- | --- |
| `created` | 任务已入队 | 用 `list_tasks` 看进度 |
| `existing` | 已有同曲任务，返回旧的 | 需要重下就带 `duplicateAction: "overwrite"` |
| `needs_confirmation` | 同名文件已存在，等你选 | 带上 `duplicateAction` 重新调用 |
| `cancelled` | 你选了取消 | — |

- **降级由 Rust 决定**：所选档位不可用时，按设置自动往下选，任务记录里能看到最终档位。
- **批量**：逐首调用 `create_download_task`；清理用 `remove_tasks`（可 `deleteFile`）。
- **直链投递 `stream`**：当服务端没有签名能力（公网部署常见）时，先在能签名的机器上解析出 CDN 直链，再随任务投递，服务端只负责下载。详见 `references/qishui.md` 的「公网部署」一节。

## 任务与曲库

| 工具 | 用途 |
| --- | --- |
| `list_tasks` | 列出任务（`status`: waiting / downloading / paused / completed / error） |
| `pause_task` / `resume_task` / `retry_task` | 暂停 / 继续 / 重试（重试会按设置降级） |
| `remove_task` / `remove_tasks` | 删除任务，可选 `deleteFile` |
| `list_library` | 列下载目录里的音频（支持关键词过滤、分页） |
| `rename_library_file` / `delete_library_files` / `write_library_metadata` | 改名 / 删除 / 写标签（标题/歌手/专辑/歌词） |
| `get_default_download_dir` | 服务端默认下载目录 |

## 账号

| 工具 | 用途 |
| --- | --- |
| `list_accounts` | 四平台登录状态与可用登录方式 |
| `start_qr_login` / `check_qr_login` | 扫码登录（QQ / 网易云 / 酷狗）；`check_qr_login` 每 2 秒轮询，`done: true` 停止 |
| `login_with_credentials` | QQ（uin+authst）/ 酷狗（userid+token）凭据登录 |
| `save_cookie` | 写 Cookie（网易云、汽水） |
| `logout_account` | 退出登录 / 清除汽水 Cookie |

**汽水音乐是特例**：不支持扫码，Cookie 要从本机客户端读，搜索与高音质还需要签名。完整流程见 `references/qishui.md`。

## Web UI 与 agent 的分工

| 能力 | Web UI | agent / MCP |
| --- | --- | --- |
| 搜索 | 除汽水外可用 | 全平台（汽水需签名） |
| 解析分享链接 | ✅ | ✅ `parse_link` |
| 建任务 / 下载 | ✅ | ✅ |
| 批量 | ✅ 勾选 | ✅ 循环调用 |
| 曲库管理 | ✅ | ✅ |
| 汽水登录 / 签名 | ❌ | ✅ 脚本 + 客户端 |

## 排错

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| 搜索返回空列表 | 平台需登录 / 汽水需签名 / 第 2 页 | 看 `list_accounts`；汽水看 `references/qishui.md` |
| `parse_link` 报"未能提取" | 链接不是单曲/歌单页，或需要登录态 | 换正式分享链接；或先 `platform` 兜底 |
| 建任务报「所选音质不可用」 | `desiredQuality` 传了文件名，或该档位真没有 | 传标签（`320kaac`）；先 `parse_link` 看可用 `qualities` |
| 任务 `error` 且提示音质不可用 | 平台侧无该档位 / 需要会员 | 换低档位或登录会员账号 |
| 任务一直 `waiting` | 队列并发已满 / 被暂停 | 看设置里的并发数，或 `resume_task` |
| 下载中断后重试仍失败 | 链接过期（汽水/QQ 常见） | `retry_task`；汽水直链任务需重新解析 |
| 页面打不开 | 容器没起/端口不对 | `docker ps` 看是否 healthy，端口取 `.env` |

## MCP 配置（agent 侧连不上时）

从项目 `data/mcp.json` 取 `key`，写进 `~/.workbuddy/mcp.json`：

```json
{"mcpServers":{"hotdownloader":{"type":"http","url":"http://127.0.0.1:8791/mcp?key=<key>"}}}
```

- 端口取 `.env` 的 `WEB_PORT`；公网部署换成你的域名/地址（`/mcp` 等于管理员凭据，建议反向代理里只对自己放行）；
- 写完需要在「连接器管理」点**信任**，然后重开会话；
- 兜底：所有 MCP 工具都有等价 HTTP 接口（`/api/music/*`、`/api/tasks`、`/api/qishui/cookie`…），Basic 认证即可，不必强依赖 MCP。

## 参考

- `references/qishui.md`：汽水音乐专项（能力对照、三步接入、签名器位置、公网直链投递、排错）
- `references/signer.md`：签名与 App 端请求契约（bdms.node、远程签名服务、track_v2 参数）
- `scripts/qishui_login.py`：从客户端读 Cookie 并落库（`client` / `save` / `status`）
- `scripts/qishui_stream.py`：本机一次性解析汽水直链（喂给 `stream`，不需要常驻签名服务）
- `scripts/qishui_doctor.py`：六项环境自检
- `scripts/qishui-signer-server.mjs`：宿主机签名服务（供容器调用）
