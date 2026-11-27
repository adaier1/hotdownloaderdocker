# 下载工作流

## 三条入口路径

```
A 用户给歌名       搜索 → 匹配(唯一) → parse_link 补档 → create_download_task
B 用户给链接/文案   parse_link → kind 分流 ─┬ song     → create_download_task
                                          ├ playlist → batch-download.md
                                          └ album    → batch-download.md
C 用户给歌单/专辑  fetch_playlist_songs / fetch_album_songs → batch-download.md
```

**永远不要跳过解析。** 只有 `parse_link`（或搜索/详情）返回的 `song` 才带真实 `qualities`；自己拼的 `song` 十有八九建出 `error` 任务。

**不要用纯 ID 走解析。** `parse_link` 只认链接：`{"input":"186016","platform":"netease"}` 会被当成歌单 ID 兜底，实测返回了另一张 159 首的歌单。按 ID 下载就拼标准链接 `https://music.163.com/song?id=<id>`。

## 单曲下载：完整调用

```jsonc
{
  "song": { /* parse_link 的 data.song 原样 */ },
  "desiredQuality": "flac",
  "duplicateAction": "rename",     // 可选，见「重名」
  "stream": { … }                  // 可选，仅直链投递（汽水）
}
```

必填只有 `song` 和 `desiredQuality`。`song` 的字段要求见 `mcp-tools.md`：`platform` `id` `mid` `title` `artist` `album` `qualities` 七个都得有，`id` 是整数、`mid` 是字符串。

`duplicateAction` 三个值：

| 值 | 行为 |
| --- | --- |
| `overwrite` | 覆盖同名文件；**若该文件正被另一个任务写入**，返回错误「同名任务正在下载，请选择保留两份」 |
| `rename` | 自动加序号（`歌名 (1).flac`） |
| `cancel` | 直接放弃，返回 `cancelled` |

不传时按设置 `duplicateStrategy`（默认 `ask`）。

## outcome 处理

| outcome | 含义 | 正确反应 |
| --- | --- | --- |
| `created` | 已入队 | 记下 `task.id`、`task.quality`、`task.filename`。若 `quality` ≠ 目标档，说明已降级，要告知用户 |
| `existing` | 已有**未结束**的同曲任务（`waiting`/`downloading`/`paused`/`processing`/`interrupted`） | **不是失败**。告知进度，等它结束；不要重建 |
| `needs_confirmation` | 目标文件已存在，**或同曲已 `completed` 且文件还在** | 问用户 overwrite/rename/cancel，然后用同一份参数 + `duplicateAction` 重调一次 |
| `cancelled` | 用户选了取消 | 结束，不要重试 |

刚建完任务的状态可能是 `error` 而不是 `waiting`——那是服务端"音质不可用"或"直链已过期"的可见记录（`errorMsg` 有原因），**任务确实存在**，不要以为没建成而重发。

> 实测区分：同曲任务还在跑 → `existing`；同曲**已完成、文件还在** → `needs_confirmation`（不是 `existing`）。所以用户说"再下一遍"时，走的是确认分支，要带上 `duplicateAction`。

反过来也要注意：**`waiting` 不代表能下成**。`qualities` 里有目标档只说明"平台声明有这一档"，真正取流时可能被拒（未登录/无权限/下架）。实测：网易云未登录时用 `flac` 建任务，建档成功且状态 `waiting`，随后 worker 逐档降级到 `128kmp3` 仍失败，最终变成：

```
error  quality=128kmp3
音质候选已耗尽，最后音质 128kmp3 获取链接失败: 网易云歌曲不可用或当前账号无权限
```

所以"有档位"≠"能取流"，汇报时不要说"已加入队列，马上就好"。

## 重名与路径

- 目标路径由服务端按设置 `downloadDir` + `namingTemplate` 算，**agent 不传路径**。
- 冲突判断同时看磁盘文件和"已被排队任务占用"的路径，所以两个任务不会抢同一个文件名。
- `rename` 生成 `歌名 (1).ext`；`overwrite` 会被正在写入的任务拒绝。
- 文件名过长时服务端会截断并在 `pathWarning` 里写说明，必要时转达用户。

## 直链投递（`stream`）

用于"解析在一台机器、下载在另一台"：

```jsonc
{"url":"https://…","playAuth":"","filename":"320.m4a","size":8414936,"expiresAt":0}
```

- 校验：`url` 必须以 `http` 开头，`filename` 必须含 `.`（形如 `320.m4a`）。
- 带上后服务端**不请求平台、不校验音质列表**，任务音质取 `filename` 主干。
- `expiresAt` 是毫秒时间戳；过期后建任务会直接是 `error`「直链已过期，请在具备签名能力的机器上重新解析后投递」，所以要**解析完立刻投**。
- 直链任务的失败重试**不能自动降级**（服务端没有平台凭据去取别的档），失败只能重新解析投递。

汽水的完整玩法见 `qishui.md`。

## 各平台差异（下载侧）

| 平台 | 特点 |
| --- | --- |
| `qqmusic` | 会员曲需要登录；`hires`/臻品系列要会员；加密容器 `.mflac`/`.mgg` 由服务端解密 |
| `netease` | 未登录只能 128k；Cookie 通过 `save_cookie` 写入后立即生效 |
| `kugou` | `mid` 是 32 位 hash，文件名里带 hash 段；歌词按 hash 取 |
| `kuwo` | 文件名按码率命名（`320.mp3`/`2000.flac`），无单独封面接口需求；**匿名也能下 320kmp3**（实测周杰伦「晴天」10.8MB 成功），缺登录时可优先试它 |
| `qishui` | 优先本机解析直链投递；服务端自签只能拿低档；会员曲 60s |
| `bilibili` | 只能 `parse_link` 视频链接，产出单曲任务；无搜索、无歌单 |

## 建任务前自检

- [ ] `song.platform` 与来源一致（别把汽水的 song 标成 netease）
- [ ] `song.qualities` 非空，且是我刚刚解析出来的，不是上一轮的缓存
- [ ] `desiredQuality` 是标签，且在该平台的真实标签表里（`quality-selection.md`）
- [ ] 已 `list_tasks` 确认没有同曲在跑（避免 `existing`）
- [ ] 直链任务：`expiresAt` 还没过

## 建任务后

1. `list_tasks` 复核一次状态（`waiting` 正常，`error` 看 `errorMsg`）；
2. 告诉用户：任务已建、目标音质、实际音质（若降级）、预计文件名；
3. 长任务不要阻塞：把 `task.id` 记下来，用户问进度时再查，或用 SSE（`architecture.md`）。
