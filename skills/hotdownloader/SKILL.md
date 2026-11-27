---
name: hotdownloader
display_name: HotDownloader 智能音乐下载
display_name_en: HotDownloader Music Download Assistant
description: 把用户的音乐需求变成可执行的下载任务并盯到完成：判断意图、搜索与匹配资源、解析分享链接、按音质规则决策、创建与管理下载任务、批量处理歌单专辑、诊断失败原因、管理曲库与账号。能力全部来自 HotDownloader 服务（MCP 工具优先，/api/* HTTP 接口备用，服务地址由 MCP 接入地址推导），不修改后端。含五平台（QQ 音乐 / 网易云 / 酷狗 / 酷我 / 汽水）能力矩阵、真实音质标签与降级顺序、七种任务状态与异常处理树、汽水音乐本机直链投递与签名、以及文档与源码一致性的自检脚本。触发词：下载歌曲、搜歌、搜歌手、下歌单、批量下载、这个链接帮我下、最高音质、无损、下载进度、任务失败重试、曲库整理、汽水音乐、网易云链接、QQ 音乐链接、HotDownloader。
version: 2.0.0
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
examples_zh:
  - "帮我把周杰伦的晴天下载成无损"
  - "这条网易云分享链接解析一下，能下就直接下"
  - "把这张歌单整张加到下载队列，已经有文件的跳过"
  - "看下下载进度，失败的挑出来重试"
  - "汽水音乐怎么登录、怎么下高音质"
---

# HotDownloader：从需求到下载完成

你不是"API 调用翻译器"。用户说的是"下这首无损"，你要自己完成：**判断意图 → 查可用资源 → 匹配到唯一那首 → 按音质规则选档 → 建任务 → 盯状态 → 处理失败 → 汇报结果**。

主文档只讲**怎么判断、怎么规划、怎么调用**。每类业务的细节在 `references/` 里按需读，不要一次性全读。

## 三条铁律

1. **先定意图，再选工具。** 同一句"晴天"可能是搜索、下载、查进度。没有确定意图前不要调写操作。
2. **先解析，再下载。** 所有建任务都用 `parse_link` 或搜索返回的**完整 song 对象**（必须含 `qualities`），不要自己拼 `song`。
3. **音质传标签，不传文件名。** `desiredQuality` 是 `320kaac` / `flac` / `hires` 这类标签；传 `320.m4a` 会直接判"所选音质不可用"。

## 第 0 步：确认服务可用（每次会话一次即可）

```bash
BASE=$(python - <<'PY'
import json, os, re
p = os.path.expanduser("~/.workbuddy/mcp.json")
url = json.load(open(p, encoding="utf-8")).get("mcpServers", {}).get("hotdownloader", {}).get("url", "")
print(re.sub(r"/mcp.*$", "", url).rstrip("/"))
PY
)
curl -s "$BASE/healthz"    # {"status":"ok"}
```

`$BASE` 只从 MCP 接入地址推导（去掉 `/mcp` 和 `?key=`），**不要写死端口**，也不要用文档里的示例地址。取不到时兜底：`.env` 的 `WEB_PORT`；再不行问用户部署地址。

同时确认两件事（缺一不可）：

- **MCP 是否连通**：调 `list_accounts` 能返回平台列表即通；不通看 `references/architecture.md` 的「MCP 连不上」。
- **各平台登录态**：同一份 `list_accounts` 结果里看 `loggedIn` / `note`。未登录常见后果：搜不到、只有试听、高音质拿不到。

## 意图路由表

先归到下面一类，再按对应文档执行。**跨类时按顺序做**（如"下载这个歌单的最高音质" = 歌单下载 → 音质决策）。

| 用户说法 | 意图 | 执行主线 | 细节 |
| --- | --- | --- | --- |
| 搜一下 / 有没有 / 帮我找 | 搜索 | `search_songs` 等 → 结果表 | `references/search.md` |
| 下载 X / 下这首 / 要这首 | 单曲下载 | 搜索 → 匹配唯一 → `parse_link` 补档 → 建任务 | `references/resource-matching.md` |
| 这个链接 / 这段分享文案 | 链接下载 | `parse_link` → 按 `kind` 分流 | `references/download-workflow.md` |
| 整张歌单 / 专辑 / 这个歌手全部 | 批量下载 | 取列表 → 去重 → 分批建任务 → 汇总报告 | `references/batch-download.md` |
| 最高音质 / 无损 / 320 / 随便 | 音质指定 | 查实际档位 → 决策表选档 → 建任务 | `references/quality-selection.md` |
| 下到哪了 / 进度 / 失败了没 | 任务查询 | `list_tasks` → 状态汇总表 | `references/task-management.md` |
| 重试 / 继续 / 暂停 / 删掉 | 任务控制 | 动作前置条件校验 → 执行 → 复核 | `references/task-management.md` |
| 我下过什么 / 清理 / 改名 / 去重 | 曲库 | `list_library` → 操作 | `references/library-management.md` |
| 登录 / 扫码 / Cookie / 账号失效 | 账号 | `list_accounts` → 对应登录方式 | `references/account-management.md` |
| 汽水 / 报错 / 下不下来 | 诊断 | 按异常树定位 → 给结论 | `references/troubleshooting.md` |

## 通道规则（MCP 优先，HTTP 备用）

| 情况 | 选哪条 | 原因 |
| --- | --- | --- |
| 有对应 MCP 工具且调用成功 | **MCP** | 参数已校验，返回结构化 |
| MCP 工具不存在（如汽水详情、签名器状态、运行日志） | HTTP `/api/*` | 这些能力只在 REST 暴露 |
| MCP 返回**业务错误**（音质不可用 / 任务不存在 / 未登录） | **不要**转 HTTP 重试 | 换个通道只会得到同样错误，还可能重复建任务 |
| MCP 连不上 / 认证失败 | HTTP | 同一套核心逻辑，结果等价 |

**副作用去重**：`create_download_task`、`remove_task`、`pause_task` 等写操作，一次判断只调一次。MCP 超时/无响应时先 `list_tasks` 查是否已生效，**不要直接重发**。批量场景见 `references/batch-download.md` 的并发上限。

## 统一结果契约

建任务返回 `outcome`，必须按它决定下一步，不能一律当成功：

| outcome | 含义 | 下一步 |
| --- | --- | --- |
| `created` | 已入队 | 记录 `task.id` + 最终 `quality` |
| `existing` | 已有同一首歌的任务（按平台+mid/mediaMid 判定） | 需要重下才带 `duplicateAction` |
| `needs_confirmation` | 目标文件已存在，等用户选 | 问用户 overwrite / rename / cancel 后**同参数重调一次** |
| `cancelled` | 用户选择取消 | 不重试 |

任务对象关键字段：`id` `platform` `songTitle` `artist` `quality` `filename` `status` `downloaded` `fileSize` `retryCount` `errorMsg` `filePath` `availableQualities`。

**七种状态**：`waiting` `downloading` `paused` `processing` `completed` `error` `interrupted`。别自己发明状态，也别把 `processing`（收尾写标签）当卡住。

## 汇报格式

- **下载完成**：成功 N / 失败 M / 跳过 K；逐条给 `歌名 - 歌手 · 音质 · 大小`；失败项带一句原因和下一步。
- **搜索**：前 5 条 `歌名 - 歌手 · 专辑 · 时长 · 可用档位`，标出你推荐的条目和理由；不确定唯一时**让用户选**。
- **进度**：`完成 / 下载中 / 等待 / 错误` 计数 + 当前速度；只在任务变化时刷新，不要空转轮询。
- **降级必须说**：实际音质低于用户要求时明确写"你要 flac，该曲只有 320kmp3，已按降级顺序选后者"。

## 参考索引（按需加载）

| 文档 | 什么时候读 |
| --- | --- |
| `references/architecture.md` | 通道选择拿不准、MCP 连不上、要看代码位置 |
| `references/mcp-tools.md` | 需要某个工具的准确参数名与返回结构 |
| `references/http-api.md` | 走 REST、脚本化、或需要 MCP 没有的能力 |
| `references/search.md` | 搜索参数、分页、平台能力差异 |
| `references/resource-matching.md` | 多条结果里挑哪一条 |
| `references/quality-selection.md` | 音质标签、可用档位、降级顺序 |
| `references/download-workflow.md` | 建任务的完整字段与边界条件 |
| `references/batch-download.md` | 歌单/专辑/多曲的调度与去重 |
| `references/task-management.md` | 状态机、动作前置条件、失败后怎么办 |
| `references/library-management.md` | 曲库查询、去重、改名、写标签 |
| `references/account-management.md` | 扫码、Cookie、凭据登录、登出 |
| `references/qishui.md` | 汽水音乐专项（Cookie、签名、直链投递） |
| `references/signer.md` | 签名协议与 App 端请求契约 |
| `references/troubleshooting.md` | 任何报错的定位表 |

## 排错起手式

报错先看**是哪一层**，别直接改参数重试：

1. 服务在不在 → `curl -s "$BASE/healthz"`
2. 登录态够不够 → `list_accounts`
3. 这首歌到底有哪些档位 → `parse_link` 看 `qualities`
4. 失败原因看任务的 `errorMsg`，别猜 → `list_tasks`

常见现象速查在 `references/troubleshooting.md`；汽水相关先跑 `python scripts/qishui_doctor.py`。

## 配套脚本

脚本在仓库 `scripts/`（skill 内不复制，避免版本漂移）：`qishui_login.py` 读客户端 Cookie、`qishui_stream.py` 本机解析直链、`qishui_doctor.py` 六项自检、`qishui-signer-server.mjs` 宿主机签名服务。用法见 `references/qishui.md`。

## 自检

改动本 skill 后跑一次，确保文档里的接口名和源码一致：

```bash
python .workbuddy/skills/hotdownloader/tests/validate_skill.py
```
