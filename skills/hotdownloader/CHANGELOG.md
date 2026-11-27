# 变更记录

## 2.0.0

重写为「判断 → 规划 → 调用」型技能：主文档只讲意图识别、通道决策与硬规则，业务细节拆到 `references/`，并新增与 Rust 源码对齐的自检脚本。

### 接口核对（对照 `crates/hotdownloader-server` 当前源码）

- 补齐 MCP **33 个工具**的准确参数与返回，修正 `create_download_task` 的 `song` 必填字段（`platform` `id` `mid` `title` `artist` `album` `qualities`，其中 `id` 为整数、`mid` 为字符串）。
- 修正任务状态：**七种** `waiting/downloading/paused/processing/completed/error/interrupted`（此前只列五种，漏掉 `processing` 与 `interrupted`）。
- 修正 HTTP 任务路由：删除/取消是 `POST /api/tasks/{id}/remove|cancel`，不是 `DELETE`。
- 新增 **MCP 与 HTTP 能力差异表**：曲库改名 `rename_library_file` 与写标签 `write_library_metadata` 只有 MCP；汽水详情 `songs/detail`、封面 `cover`、个人歌单 `playlists/created`、签名器状态 `/api/qishui/signer`、日志 `/api/logs`、SSE `/api/events`、快捷指令 `/api/shortcut/parse` 只有 HTTP。
- 记录 `get_lyric_by_id` 对酷狗/汽水无效（这两家按 `songMid` 取词，工具只有 `songId`），改走 `POST /api/music/lyrics`。
- 明确 `retry_task` 返回 `{retried: bool}`，`false` 表示"状态不是 error"，不是失败。

### 业务规则

- 新增**意图路由表**（11 类意图 → 主线 → 对应文档）与三条铁律（先定意图 / 先解析再下载 / 音质传标签）。
- 新增 `resource-matching.md`：五维匹配、决策顺序、判重键（`platform + mid`，`mid` 空时用 `id`/`mediaMid`）、歧义提问模板。
- 重写 `quality-selection.md`：五平台**真实音质标签**、24 档全局降级顺序、`initial_quality` 语义（只降不升、目标不在顺序表即不可用）、「不可用替代 / 降级必告知 / 不把标称当实际」三条机制。
- 新增 `batch-download.md`：七步流水线、按批串行建任务（每批 5–10，禁止无限并发）、断点继续（重跑即跳过已完成）、失败单列重试、报告格式。
- 新增 `task-management.md`：七状态动作矩阵与真实错误文案、`retry_count` 4 次后降级、异常处理树。
- 新增 `library-management.md`：曲库是磁盘文件而非任务记录、删除的安全边界、去重清单式流程。
- 新增 `account-management.md`：四平台登录方式矩阵（汽水只能 Cookie）、扫码轮询 `done` 语义、凭据不回显。

### 真实环境实测（2026-11-27，对照线上服务）

按 `tests/test_workflows.md` 跑通"搜索 → 匹配 → 解析 → 建任务 → 观察 → 清理"闭环后，修正了三处与实测不符的表述：

- **`parse_link` 不接受纯数字 ID**：`{"input":"186016","platform":"netease"}` 返回 `kind: "playlist"`（id 186016 恰好也是一张 159 首歌单）。原因：离线无法判断类型时会按歌单兜底。文档改为"只传链接"，按 ID 下载要拼 `https://music.163.com/song?id=<id>`。
- **"有档位"不等于"能取流"**：网易云未登录时用 `flac` 建任务返回 `created` + `waiting`，随后 worker 逐档降级到 `128kmp3` 仍失败，最终 `error`。文档补充"建档成功不代表能下成"，此前只写了建档即报 `所选音质不可用` 的情况。
- **补全真实错误文案**：`音质候选已耗尽，最后音质 X 获取链接失败: <原因>`、`获取下载链接失败: <原因>`、`QQ 音乐拒绝提供下载链接（result=104003）…` 此前完全缺失，异常树按这些前缀重写；并说明 `error` 任务的 `quality` 会显示"最后尝试的档位"而非用户选择。

另外补充两条实战经验：

- 热门老歌（「晴天」等）关键词搜索前 10 条可能全是翻唱/别名/拼盘，原版一条没有；正确姿势是 `search_artists` 拿歌手 ID 后 `fetch_artist_songs`，用 **ID 而非名字**判断同一歌手（别名 "Jay" 是另一个 ID）。
- `GET /api/login/status` 返回 snake_case（`logged_in`），与 MCP `list_accounts` 的 camelCase（`loggedIn`）不同，写脚本别混用。

第二轮实测（酷我成功路径 + 汽水 + 设置）又修正四处：

- **`existing` 只针对未结束的任务**：同曲已完成且文件还在时返回的是 `needs_confirmation`，不是 `existing`。此前把两者混在一起写，会导致 agent 在"再下一遍"时说"已完成不用下"而实际拿不到文件确认。
- **汽水搜索失败是明确错误，不是空列表**：服务端已把上游的 `200 + 空 body` 翻译成「汽水搜索需要本机汽水客户端提供签名…」/「汽水搜索暂不可用：接口返回空响应…」。此前写"返回空列表"会误导 agent 去换关键词。
- **`fileSize` 是下载字节数，不是文件大小**：写标签后文件变大（实测 10,789,847 → 11,513,721），判断实际质量要看 `list_library` 的 `size`。
- **臻品系列标签不只 QQ/网易云**：酷我同样返回 `臻品母带`/`臻品全景声 7.1.4`。
- 补充「酷我匿名也能下 320kmp3」（实测成功），作为未登录时的备选；`/api/settings` 只返回已存字段（实测仅 4 项），未出现的字段 `expected` 传 `null`。

### 结构

- `SKILL.md` 由 API 说明改为路由层（约 170 行），细节按需加载。
- 新增 `references/architecture.md`、`mcp-tools.md`、`http-api.md`、`search.md`、`troubleshooting.md`。
- 新增 `tests/validate_skill.py`（文档工具名/HTTP 路径与源码交叉校验）、`tests/validate_references.py`（链接与引用完整性）、`tests/test_workflows.md`（人工验收用例 A–K 共 53 条）。
- `test_workflows.md` 的 **K 组（K1–K7）是 2026-11-27 线上实测回归**：每条对应下面「真实环境实测」里一处曾经写错的地方，并记录了实测值作为判定基准；K1–K5 列为每次改文档后的必测项。
- 脚本仍只放在仓库 `scripts/`，skill 内不复制，避免版本漂移。

### 修正的旧表述

- 汽水"第 2 页起返回空"补充原因（游标无法由页码推导）与"专辑/歌手/歌单搜索返回未知操作"。
- 补上此前完全缺失的平台 **bilibili**（只能链接解析，产出单曲）。
- 补上 `kuwo` 无个人歌单、`qqmusic/netease` 才有 `playlists/created`。

## 1.2.0

- 汽水音乐专项（客户端 Cookie + `bdms.node` 签名、本机解析直链投递）。
- 服务地址统一从 MCP 接入地址推导。
- 补签名协议参考与排错表。

## 1.1.0

- 增加批量下载与音质降级说明。

## 1.0.0

- 初版：搜索 / 解析 / 下载 / 任务 / 曲库的基础用法。
