# HTTP API 参考

只在三种情况用 HTTP：MCP 没有该能力、MCP 连不上、要写脚本/自动化。其余优先 MCP。

所有路径都对照 `crates/hotdownloader-server/src/http.rs` 核对过。

## 认证

| 入口 | 认证方式 |
| --- | --- |
| `POST /mcp` | `?key=<mcp key>` 或 Basic/Bearer |
| `GET /api/shortcut/parse` | `?key=<shortcut key>` |
| `/api/*`（其余） | Basic 或 Bearer |
| `GET /healthz`、`GET /api/auth/mode` | 无需认证 |
| `GET /api/login/qr-image/{platform}.png` | 需认证 |

- Basic 凭据：`.env` 的 `AUTH_USERNAME`/`AUTH_PASSWORD`，或数据目录 `auth.json`；都没有时默认 `admin` / `admin123`（首次部署后应改）。
- Bearer：`HOTDOWNLOADER_TOKEN` 配置时可用 `Authorization: Bearer <token>`。
- 失败统一 `401 {"error":"访问凭据无效"}`。

```bash
BASE=$(python - <<'PY'
import json, os, re
p = os.path.expanduser("~/.workbuddy/mcp.json")
url = json.load(open(p, encoding="utf-8")).get("mcpServers", {}).get("hotdownloader", {}).get("url", "")
print(re.sub(r"/mcp.*$", "", url).rstrip("/"))
PY
)
AUTH="-u ${AUTH_USER:?}:${AUTH_PASS:?}"   # 先 export AUTH_USER/AUTH_PASS，别把密码写进命令历史
curl -s $AUTH "$BASE/api/tasks"
```

## 路由总表

### 音乐查询

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/api/music/{action}` | action 见下表，body 为 `MusicRequest` |

`MusicRequest` 字段（都可选，按 action 取用）：`platform` `keyword` `input` `id` `dirid` `songId`(整数) `songMid`(字符串) `page`(默认 1) `limit`(默认 20，上限 100)。

| action | 平台 | 需要的字段 |
| --- | --- | --- |
| `songs/search` | 全五平台 | `platform` `keyword` |
| `albums/search` `artists/search` `playlists/search` | 除汽水 | `platform` `keyword` |
| `albums/fetch` | 除汽水 | `platform` `id` |
| `artists/songs` `artists/albums` | 除汽水 | `platform` `id` `page` `limit` |
| `playlists/fetch` | 除汽水 | `platform` `input` |
| `playlists/created` | netease/qqmusic | `platform` |
| `playlists/created/fetch` | netease/qqmusic | `platform` `id`（QQ 另带 `dirid`） |
| `hot-keywords` `suggestions` | 除汽水（汽水返回空） | `platform` |
| `songs/detail` | **仅汽水** | `platform` `songMid` |
| `cover` | netease/kuwo | `platform` `songId` |
| `lyrics` | 五平台 | `platform` + `songId`（netease/qq/kuwo）或 `songMid`（kugou/qishui） |
| `links/parse` | 任意 | `input` `platform`(兜底，可省) |

`links/parse` 的返回与 MCP `parse_link` 完全一致（`{platform, kind, data}`）。

汽水补档位（MCP 没有这个能力）：

```bash
curl -s $AUTH -X POST "$BASE/api/music/songs/detail" \
  -H 'content-type: application/json' \
  -d '{"platform":"qishui","songMid":"7689447702921349121"}'
# → {"song":{…,"qualities":[{"quality":"hires",…},{"quality":"flac",…}]}}
```

### 任务

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/tasks` | 全部任务 |
| POST | `/api/tasks` | 建任务，body 同 MCP `create_download_task` |
| POST | `/api/tasks/remove` | `{"taskIds":[…],"deleteFile":false}` → `{succeeded,failed,errors}` |
| POST | `/api/tasks/{id}/pause` | → `{ok:true}` |
| POST | `/api/tasks/{id}/resume` | → `{ok:true}` |
| POST | `/api/tasks/{id}/retry` | → `{retried:bool}` |
| POST | `/api/tasks/{id}/cancel` | body 可选 `{"deleteFile":true}` |
| POST | `/api/tasks/{id}/remove` | body 可选 `{"deleteFile":true}` |

注意 `cancel` / `remove` 都是 **POST**，不是 DELETE；`{id}` 在前、动作在后。

### 曲库

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/library` | `{directory, files:[…]}`，**全量**，不支持过滤分页 |
| POST | `/api/library/delete` | `{"paths":[…]}` → `{deleted, failed:[{path,error}]}` |

重命名/写标签 HTTP **没有**，只能用 MCP。

### 设置

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/settings` | `{revision, settings}` |
| PATCH | `/api/settings` | `{"changes":{…},"expected":{…}}`；冲突返回 `409` + `{error, fields, snapshot}` |
| GET | `/api/settings/default-download-dir` | 字符串 |

### 账号

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/login/status` | 仅 QQ |
| POST | `/api/login/qr` | QQ 二维码（base64） |
| GET | `/api/login/qr/{id}` | QQ 扫码状态轮询 |
| POST | `/api/login/manual` | QQ `{"uin","authst",…}` |
| POST | `/api/login/kugou` | `{"action":"qr|check|manual|status|logout","id":"…","credentials":{…}}` |
| POST | `/api/login/netease` | 同上结构 |
| POST | `/api/login/logout` | 仅 QQ |
| GET | `/api/login/qr-image/{platform}.png` | 最近一次生成的二维码图 |

四平台统一入口（含网易云/酷狗扫码与汽水 Cookie）建议直接用 MCP 的 `list_accounts` / `start_qr_login` / `save_cookie`，HTTP 侧是历史分平台接口，参数更碎。

> 字段命名不一致：`GET /api/login/status` 返回的是上游原始 JSON，**snake_case**（`{"logged_in":true,"uin":"…"}`），而 MCP `list_accounts` 是 camelCase（`loggedIn`）。写脚本时别混用。

### 汽水

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/qishui/cookie` | 只返回 `{configured, length, keys}`，不回显值 |
| POST | `/api/qishui/cookie` | `{"cookie":"…"}` 立即生效 |
| DELETE | `/api/qishui/cookie` | 清除 |
| GET | `/api/qishui/signer` | `{available, mode}`，`mode` 为 `local`/`remote`/`none` |

### 运维

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/healthz` | `{"status":"ok"}` |
| GET | `/api/events` | SSE：`task-snapshot` `settings-snapshot` `task-updated` 等，20s 心跳 |
| GET/POST | `/api/logs` | 运行日志读写；`POST /api/logs/clear` 清空 |
| GET/POST | `/api/notify` | 通知渠道配置；`POST /api/notify/test` 测试 |
| GET | `/api/mcp` | 当前 MCP key |
| POST | `/api/mcp/reset` | 重置 MCP key |
| GET | `/api/shortcut` | 当前快捷指令 key |
| POST | `/api/shortcut/reset` | 重置 |
| POST | `/api/auth/password` | `{"currentPassword","newPassword"}` |

**SSE 用法**（想看实时进度而不是轮询）：

```bash
curl -sN $AUTH "$BASE/api/events"
# event: task-snapshot
# data: [{…}]
# event: task-updated
# data: {…}
```

## 快捷指令

`GET /api/shortcut/parse?key=<shortcut key>&url=<分享链接>&limit=20`

用 URL key 认证，**解析后直接建任务**（不需要确认），单次最多 100 首（默认 20）。只适合手机捷径这类"点一下就要结果"的场景；agent 不要用它替代 `parse_link` + `create_download_task`，因为它无法处理重名确认。

## 与 MCP 的差异（记住这几条就够）

1. **曲库改名/写标签只有 MCP**；**汽水详情/封面/个人歌单/签名器状态/日志/SSE 只有 HTTP**。
2. `/api/library` 不支持 `keyword`/`offset`/`limit`（MCP 支持）。
3. 任务取消/删除是 `POST /api/tasks/{id}/cancel|remove`，不是 DELETE。
4. MCP 的工具错误是 `isError` + 文本；HTTP 是 4xx + `{"error"}`。判断逻辑要分开写。
5. `PATCH /api/settings` 冲突返回 `409` 并附最新 snapshot，拿到后重新取原值再改，不要盲目重试。
