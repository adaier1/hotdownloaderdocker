# 批量下载

歌单、专辑、歌手全部、多选歌曲，都走同一套流水线。核心是**先算清单，再批量建任务，最后出报告**——不要一边解析一边建任务。

## 流水线

```
1 解析容器        parse_link / fetch_playlist_songs / fetch_album_songs / fetch_artist_songs
2 取全量列表      按需翻页（除汽水），得到 songs[]
3 去重           同平台同 mid / mediaMid 合并；跨平台不合并
4 预检           登录态、每首的 qualities、preview 标记、音质可达性
5 核对已有任务    list_tasks → 已完成的跳过，在跑的跳过，error 的按需重试
6 分批建任务     每批 5–10 首，串行调用，批间不留 sleep 死等
7 汇总报告       成功 / 失败 / 跳过 / 降级，附可操作原因
```

## 1–2. 取全量列表

| 来源 | 调用 |
| --- | --- |
| 歌单链接 | `parse_link` → 若 `kind: playlist`，用 `data.songs`；短链（`kind: unknown`）直接 `fetch_playlist_songs` |
| 歌单 ID | `fetch_playlist_songs`（`platform` + `input`） |
| 专辑链接/ID | `parse_link` 或 `fetch_album_songs` |
| 歌手全部 | `fetch_artist_songs`，`page` 递增直到 `has_more: false` |
| 多选歌曲 | 用户给的歌名列表 → 逐个走单曲流程（`resource-matching.md`） |

翻页上限：歌手全量可能几百首，**先取前 2 页给用户确认范围**，别默默把 500 首全下了。

汽水不支持歌单/专辑接口：只能逐首拿 `mid` 后按单曲处理。

## 3. 去重

- 键：`platform + mid`（`mid` 空则 `id`，或 `mediaMid`）。
- 同一歌单里重复出现的同曲（常见于精选集）只留一条。
- 不同平台**不合并**（音质/完整性不同），但如果用户说"能下一份就行"，按 `resource-matching.md` 选一个平台后只留那批。
- 归一化标题不参与去重，只用于报告展示。

## 4. 预检

批量失败最贵的不是重试，是"下到一半才发现全都不可下"。建任务前扫一遍：

| 检查 | 不通过怎么办 |
| --- | --- |
| `qualities` 是否为空 | 该首标记 `不可下载`，跳过并在报告里列出 |
| `song.preview === true` | 会员曲试听，标记 `仅试听`，问用户是否仍要 |
| 目标音质是否可达 | 记录将降级到哪一档，进报告的"降级"栏 |
| 平台登录态 | 未登录先 `list_accounts` 确认，必要时提示登录后再批量 |

## 5. 核对已有任务

```jsonc
list_tasks → 按 platform+mid 建索引
├─ completed 且文件存在 → 跳过（除非用户要重下）
├─ waiting / downloading / paused / processing → 跳过，计入"进行中"
├─ error → 默认跳过；用户说"重下失败的"才 retry_task
└─ 无记录 → 进待建队列
```

这一步就是"断点继续"：用户第二天说"继续下昨天那张歌单"，重跑流水线即可，已完成的自动跳过。

## 6. 分批建任务

**硬规则：不要无限并发调建任务。** 服务端的下载并发由设置 `maxConcurrent`（1–32）管，但你并发建任务会导致：

- 重名检查互相撞（本该 `rename` 的变成 `needs_confirmation`）；
- MCP 超时后无法判断是否已创建；
- 上下文里堆几百条返回。

推荐做法：

```
每批 5–10 首
  └─ 串行 create_download_task
       ├─ created        → 记入成功
       ├─ existing       → 记入跳过（附旧任务状态）
       ├─ needs_confirmation → 记入待确认（不要自动 overwrite）
       └─ 错误           → 记入失败（附 errorMsg）
批次之间直接继续，不要 sleep 等待下载完成
```

- `duplicateAction` 统一传 `"rename"`，避免批量过程中反复弹确认（`ask` 会让一批里出现多个 `needs_confirmation`）。
- 超时/无响应的那一首：先 `list_tasks` 看是否已建，再决定是否补建，**不要盲目重发**。
- 中途用户取消 → 停止建后续批次，已建的保留，报告里写"已建 N 首，剩余 M 首未提交"。

## 7. 汇总报告

```
歌单《XX》共 42 首
✅ 已提交 34 首 · ⏭ 跳过 5 首（已完成）· ⚠️ 降级 2 首 · ❌ 失败 3 首

降级：
- 晴天 · 要 flac，实际 320kmp3（该曲无无损）
- 七里香 · 要 hires，实际 flac

失败：
- 夜的第七章：所选音质不可用（该平台仅 128kmp3）
- 稻香：链接已过期，需要在装有汽水客户端的机器上重新解析
- 某曲：平台未登录（网易云）

要我把失败的 3 首换平台重试吗？
```

报告要求：

- 数字必须自洽（提交 + 跳过 + 失败 = 总数）；
- 失败项**每条给一句原因**，不要只给数量；
- 降级项单独列，不要混在成功里；
- 结尾给一个明确的下一步选项。

## 失败歌曲单独重试

```jsonc
// 只重试指定几首
retry_task {"taskId": "…"}        // → {retried: true|false}
```

- 只对 `status: error` 生效；其它状态返回 `retried: false`（代表"无需重试"，不是失败）。
- 同曲已有未结束任务时会报错「此歌曲已有未结束的下载任务」——先处理那条。
- 重试 4 次后自动降一档（`quality-selection.md`），降级后 `retryCount` 归零。
- 直链任务（汽水）不能自动降级，失败要回本机重新解析投递。

## 清理

```jsonc
remove_tasks {"taskIds": ["…","…"], "deleteFile": false}   // → {succeeded, failed, errors[]}
```

批量清理时**默认不删文件**；用户明确说"连文件一起删"才传 `deleteFile: true`。`failed` 是数组（每项带 `path`/`error`），不是数字。

## 别做的事

- 不要为每首歌 `parse_link` 一遍——歌单列表已经带 `qualities` 时可直接用；只有汽水（占位档位）和档位明显异常时才补解析。
- 不要在批量里用 `overwrite`，一次误覆盖会毁掉用户已有文件。
- 不要把"跳过"报成"成功"，用户会以为全下好了。
- 不要一次提交 200 首后就说"全部完成"——下载是异步的，说"已提交"。
