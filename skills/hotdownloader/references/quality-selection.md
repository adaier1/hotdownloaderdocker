# 音质决策

音质规则只在两处生效，**不要自己在对话里另立一套**：

1. **建任务时**（`create_download_task`）选初始档位，`task/rules.rs::initial_quality`；
2. **运行/重试时**降级，`download/fallback.rs` + `task/service.rs::retry_task_with_quality`。

agent 的职责是：把用户的话翻译成**标签**，确认这首歌真的有这档，选不到时说清楚。

## 标签 ≠ 文件名（最常见的错）

`desiredQuality` 必须是**标签**。文件名只出现在 `qualities[].filename` 和 `stream.filename` 里。

| 传这个 | 结果 |
| --- | --- |
| `"desiredQuality": "flac"` | ✅ 正确 |
| `"desiredQuality": "320kaac"` | ✅ 正确 |
| `"desiredQuality": "320.m4a"` | ❌ 建任务直接 `error`：「所选音质不可用」 |

唯一例外：带 `stream` 直链时服务端不校验音质列表，任务音质取 `stream.filename` 的主干（`320.m4a` → `320`）。即使如此也建议顺手传对应标签，任务记录更好读。

## 各平台真实标签

| 平台 | 标签（从低到高） | 文件名形态 |
| --- | --- | --- |
| `netease` | `128kmp3` `320kmp3` `flac` `hires` `高清环绕声` `沉浸环绕声` `超清母带` | `{id}_{level}.{mp3\|flac}` |
| `qqmusic` | `48kaac` `96kaac` `192kaac` `96kogg` `192kogg` `128kmp3` `320kmp3` `ape` `flac` `hires` `臻品全景声` `臻品全景声 5.1` `臻品母带` | `M800{mediaMid}.mp3`、`F0M0{mediaMid}.mflac`、`AIM0{vs}.mflac` 等 |
| `kugou` | `128kmp3` `320kmp3` `flac` `hires` | `{hash}_{albumId}_{audioId}_{quality}.{ext}` |
| `kuwo` | `48kaac` `100kogg` `192kogg` `300kogg` `128kmp3` `320kmp3` `flac` `hires` | `320.mp3`、`2000.flac`（按码率命名） |
| `qishui` | `64kaac` `128kaac` `320kaac` `spatial` `flac` `hires` | `320.m4a`、`spatial.m4a`、`flac.flac` |

「臻品全景声」「超清母带」这类中文标签**是真实标签**，可以原样传 `desiredQuality`。QQ、网易云、酷我都会返回这类会员档（实测酷我「晴天」直接列出 `臻品母带`/`臻品全景声 7.1.4`），但**列出不等于能取到流**。

## 用户说法 → 标签

| 用户说 | 目标标签 | 说明 |
| --- | --- | --- |
| 最高音质 / 最好的 | 该曲实际可用的最高档 | 见下方「可用最高」算法 |
| 无损 / lossless | `flac`（有则优先），其次 `hires` | 无损容器优先 |
| Hi-Res / 高解析 / 母带 | `hires` → `超清母带`/`臻品母带` → `flac` | |
| 320 / 高音质 | `320kmp3` 或 `320kaac`（按平台有没有） | 汽水只有 aac |
| 标准 / 普通 / 随便 | 设置里的 `defaultQuality` | 不要硬编码 128k |
| 只要能听 | 保留最低档即可 | 仍然走正常流程 |
| 空间音频 / 全景声 | `spatial`（汽水）/ `臻品全景声`（QQ）/ `沉浸环绕声`（网易云） | |

## 可用最高档的算法

**不要用降级顺序表的第一项当"最高"**——那张表是跨平台的全局偏好，不是这首歌的实际档位。正确做法：

```
available = song.qualities.map(q => q.quality)
target = 按「平台标签（从低到高）」取 available 里最高的那项
```

即：先按该平台自己的档位阶梯排序，取 `available` 中位置最靠后的一项。跨平台比较才用全局顺序表。

## 全局降级顺序（服务端真实值）

```
超清母带 > 沉浸环绕声 > 高清环绕声 > 臻品母带 > 臻品全景声 7.1.4 > 臻品全景声 5.1
> 臻品全景声 > hires > flac > spatial > ape > 320kaac > 320kmp3 > 128kaac > 128kmp3
> 64kaac > 300kogg > 192kogg > 100kogg > 96kogg > 192kaac > 96kaac > 48kaac
```

规则（`TaskRules`）：

- **只降不升**：从目标档的**下一项**往后找，绝不回头拿更高档。
- `initial_quality`：可用列表里有目标档 → 直接用；没有 → 若 `autoDowngrade` 为真则按上表往下找，否则返回空（→ 建 `error` 任务）。
- 顺序表来自设置 `qualityDowngradeOrder`，用户可以改；未知值会被过滤掉。因此**目标档不在表里时直接不可用**——传标签前先确认它在该平台真实标签表里。
- 运行中的降级（`fallback.rs`）在下载失败后按同顺序切下一档，**不创建新任务**、不占额外并发位；目标文件被占用时自动加序号保留两份。

## 三个必须遵守的机制

1. **不可用时找替代**：目标档拿不到 → 先看同平台低一档，再看其它平台同曲是否有该档（跨平台要用户同意，因为文件/来源不同）。
2. **降级必须告知**：任务记录里的 `quality` 才是实际档位。如果它低于用户要求，回复里必须写明"你要 X，实际 Y（原因）"。批量下载时在报告里单列"降级曲目"。
3. **不把标称当实际**：`qualities[].size` 是平台给的标称大小，汽水还会用 `码率 × 时长` 估算。**判断实际质量看磁盘上的文件大小**（`list_library` 的 `size`）；明显小于标称（比如 flac 只有 2MB）说明拿到的是试听片段或降级流。注意任务的 `fileSize` 是**下载字节数**，不是最终文件大小——写了标签/封面之后文件会更大（实测 10,789,847 下载 → 11,513,721 落盘），两者不要混着比。

还有一层：**`qualities` 里列出的档位不代表能取到流**。实测网易云未登录时，「晴天」的 `qualities` 明确有 `flac`，建任务成功、状态 `waiting`，但 worker 逐档降级到 `128kmp3` 仍然失败，最终 `error`：

```
音质候选已耗尽，最后音质 128kmp3 获取链接失败: 网易云歌曲不可用或当前账号无权限
```

所以"档位齐全"只能说明平台声称有，**能不能下取决于登录态与版权**。给用户承诺前，先看 `list_accounts` 的登录态。

## 特殊情形

| 情形 | 处理 |
| --- | --- |
| 会员曲，非会员账号（汽水尤甚） | 只能拿 60s 片段。`song.preview` 为 `true` 就是这个情况，**下载前就告诉用户**，别下完再说 |
| 汽水搜索结果的 `128kaac` | 占位值，不要据此判断"只有 128"。先 `parse_link` / `songs/detail` |
| 要 flac 但全库最高 320kmp3 | 不要静默降级：说明情况，问是否接受 |
| 设置里 `autoDowngrade` 为 false | 目标档不可用时会建 `error` 任务而不是降级，回复要按这个事实说 |
| 用户改过 `qualityDowngradeOrder` | 以 `get_settings` 的当前值为准，不要用本文的表 |

## 修改音质的边界

- **已建任务不能改音质**：只能 `remove_task` 后重新建（或 `retry_task` 等它降级）。
- `retry_task` 不接受音质参数（MCP/HTTP 都没有暴露 `retry_task_with_quality`）；它会按原档重试最多 3 次，第 4 次起自动降到下一档并把 `retryCount` 归零。
- 想直接换档 → 删任务重建，并告知用户会重新下载。
