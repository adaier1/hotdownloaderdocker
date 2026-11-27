# 架构与通道决策

## 分层

```
用户指令
  │
  SKILL.md              判断意图、规划步骤、决定调用顺序
  │
  references/*.md       每类业务的具体规则（按需加载，不进主上下文）
  │
  ┌────────────┴────────────┐
  MCP 工具（优先）      HTTP /api/*（备用）
  Streamable HTTP        REST + Basic/Bearer
  │                        │
  └────────────┬────────────┘
        HotDownloader 服务（同一套 Rust 核心）
        TaskService / DownloadEngine / 平台客户端
```

两条通道**不是两套逻辑**：MCP 的每个工具就是 HTTP 处理函数的薄包装（`mcp.rs::run_music` → `music::execute`，`create_download_task` → `TaskService`）。所以同一请求换通道结果一致，出问题的概率也一致。

## 通道决策树

```
需要的能力
├─ 有对应 MCP 工具？
│   ├─ 是 → 调 MCP
│   │       ├─ 成功 → 按结果继续
│   │       ├─ 业务错误（音质不可用/任务不存在/未登录/参数非法）
│   │       │        → 停止重试，按错误文案处理或问用户
│   │       └─ 连接失败/401 → 转 HTTP 试一次（同一个调用，只试一次）
│   └─ 否 → 该能力只在 HTTP 有 → 调 HTTP
└─ HTTP 也 404 → 这个版本没有该能力，不要编造
```

**只在 HTTP 存在的能力**（MCP 无对应工具）：

| 能力 | 接口 |
| --- | --- |
| 汽水歌曲详情（补真实档位） | `POST /api/music/songs/detail` |
| 封面单独获取（网易云/酷我） | `POST /api/music/cover` |
| 个人歌单列表 / 内容 | `POST /api/music/playlists/created[/fetch]` |
| 汽水签名器状态 | `GET /api/qishui/signer` |
| 运行日志 | `GET|POST /api/logs`、`POST /api/logs/clear` |
| 任务实时事件流 | `GET /api/events`（SSE） |
| 通知渠道配置 | `GET|POST /api/notify`、`POST /api/notify/test` |
| 手机捷径一键入库 | `GET /api/shortcut/parse?key=…&url=…` |
| 读当前 MCP key / 重置 | `GET /api/mcp`、`POST /api/mcp/reset` |
| 修改密码 | `POST /api/auth/password` |

**只在 MCP 存在的能力**（HTTP 没有等价路由）：

| 能力 | 工具 |
| --- | --- |
| 曲库按关键词过滤 + 分页 | `list_library`（HTTP `/api/library` 返回全量） |
| 曲库文件重命名 | `rename_library_file` |
| 曲库文件写标签 | `write_library_metadata` |

这两条差异很关键：**曲库改名/写标签只能用 MCP**，别去 REST 找。

## 结果契约要统一

不管走哪条通道，业务结果形状一致，agent 只需要认这几种：

- **建任务**：`{outcome: created|existing|needs_confirmation|cancelled, task?: TaskRecord}`（`cancelled` 无 body）
- **任务列表**：`TaskRecord[]`（`status` 七种，见 `task-management.md`）
- **批量删除**：`{succeeded, failed, errors[]}`
- **重试**：`{retried: true|false}` —— `false` 代表"状态不是 error，无需重试"，**不是失败**
- **音乐查询**：`{songs:[…], has_more: bool}` / `{song:{…}}` / `{platform, kind, data}`
- **错误**：MCP 是 `isError: true` + 文本；HTTP 是 `4xx` + `{"error":"…"}`

## 别做的事

- 不要为了"更保险"把同一个写操作在两条通道各发一次（会重复建任务/重复删文件）。
- 不要把 MCP 的业务错误当成"通道问题"转 HTTP 重试。
- 不要并发狂刷建任务：并发上限见 `batch-download.md`。
- 不要假设旧版本的接口还在：本文所有接口都对照当前源码核对过，改动后跑 `tests/validate_skill.py`。

## 代码位置（维护用）

| 关注点 | 位置 |
| --- | --- |
| MCP 工具定义与分发 | `crates/hotdownloader-server/src/mcp.rs`（`tool_definitions()` / `call_tool()`） |
| HTTP 路由与认证 | `crates/hotdownloader-server/src/http.rs`、`auth.rs` |
| 音乐查询分派（唯一权威 action 列表） | `crates/hotdownloader-server/src/music.rs` |
| 链接识别（平台/类型） | `crates/hotdownloader-core/src/platforms/link_parse.rs` |
| 平台枚举 | `crates/hotdownloader-core/src/platforms/platform.rs` |
| 任务创建/重试/降级规则 | `task/service.rs`、`task/rules.rs`、`task/contract.rs` |
| 运行中降级 | `download/fallback.rs` |
| 音质标签产出 | `platforms/<平台>/parser.rs` |
| 曲库文件操作 | `hotdownloader-core/src/library.rs` |
| 快捷指令入口 | `crates/hotdownloader-server/src/shortcut.rs` |

## 服务地址

唯一权威来源是 `~/.workbuddy/mcp.json` 里 `hotdownloader` 的 `url`：

```bash
BASE=$(python - <<'PY'
import json, os, re
p = os.path.expanduser("~/.workbuddy/mcp.json")
url = json.load(open(p, encoding="utf-8")).get("mcpServers", {}).get("hotdownloader", {}).get("url", "")
print(re.sub(r"/mcp.*$", "", url).rstrip("/"))
PY
)
```

`?key=` 就是 `/mcp` 的凭据（等同管理员权限），不要外传；`/api/*` 用 Basic/Bearer，凭据在 `.env` 的 `AUTH_USERNAME/AUTH_PASSWORD` 或数据目录 `auth.json`（默认 `admin/admin123`，首次部署后应改）。

## MCP 连不上

1. `curl -s "$BASE/healthz"` 服务在不在；
2. `curl -s -u user:pass "$BASE/api/mcp"` 取当前 key；
3. 核对 `~/.workbuddy/mcp.json` 的 `url` 是否 `http://<host>:<port>/mcp?key=<key>`；
4. 客户端里点"信任"新服务器，重开会话（新加的 MCP 不会热加载）；
5. 仍然不行就先走 HTTP，功能不受影响（除曲库改名/写标签）。
