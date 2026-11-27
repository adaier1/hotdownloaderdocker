# 搜索

## 能力矩阵（真实分支，来自 `music.rs`）

| 平台 | 歌曲 | 专辑 | 歌手 | 歌单 | 热搜 | 联想 | 个人歌单 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `qqmusic` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `netease` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `kugou` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `kuwo` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `qishui` | ✅（需签名） | ❌ | ❌ | ❌ | 空 | 空 | ❌ |
| `bilibili` | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

汽水不支持的那几项不是"没数据"，而是服务端直接返回「未知音乐操作」。B 站只支持链接解析（见 `download-workflow.md`）。

## 参数

```jsonc
{"platform":"netease","keyword":"周杰伦 晴天","page":1,"limit":20}
```

- `page` ≥ 1；`limit` 1–100，默认 20。超出报「无效的分页参数」。
- `keyword` 建议 `歌手 + 歌名` 或其中一个；带空格的组合搜索命中率高于只给歌名。
- 搜索结果字段：`mid`（平台内唯一 ID）、`title`、`artist`、`album`、`coverUrl`、`duration`（秒）、`qualities`、`platform`。
- `has_more` 为 false 就是最后一页。**不要用 `page` 无限翻**：汽水从第 2 页起恒为空。

## 分页策略

1. 首屏 `limit: 20` 足够排序；
2. 首屏没有目标 → 换关键词（加/减歌手名、加专辑名），比翻页有效；
3. **热门老歌别死磕关键词**：实测搜「周杰伦 晴天」前 10 条全是翻唱/别名/拼盘，原版一条没有。改用 `search_artists` 拿歌手 ID，再 `fetch_artist_songs` 找目标歌（见 `resource-matching.md` 的实战段）；
4. 仍未命中 → 用 `fetch_suggestions` 看平台的联想词，或用 `fetch_hot_keywords` 找正确写法；
5. 明确知道要给用户"全部结果"时才翻页，且最多 3 页。

## 按 ID / 链接取内容

| 场景 | 工具 | 参数 |
| --- | --- | --- |
| 专辑歌曲 | `fetch_album_songs` | `platform` `id` `page` `limit` |
| 歌手歌曲 | `fetch_artist_songs` | `platform` `id` `page` `limit` |
| 歌手专辑 | `fetch_artist_albums` | `platform` `id` `page` `limit` |
| 歌单歌曲 | `fetch_playlist_songs` | `platform` `input`（链接或 ID） |
| 任意链接 | `parse_link` | `input` |

`fetch_playlist_songs` 直接吃分享链接，比让用户报 ID 可靠。`id` 来自搜索结果里的 `mid` 或 `id`（专辑/歌手用 `id`）。

汽水的 `mid` 是 19 位数字字符串，超出 JS 安全整数范围，**必须按字符串传**，不要 `parseInt`。

## 搜索结果的可信度

- `qualities` 在**汽水搜索里只是占位**（通常 `128kaac`），真实档位要 `parse_link` 或 `songs/detail`。
- 其它平台的 `qualities` 来自详情接口，可当准。
- `duration` 为 0 常见于汽水部分接口，不代表可下载性。
- 网易云未登录时部分歌曲的 `qualities` 只剩 `128kmp3`，那是权限不是资源缺失。

## 多平台并行

同一首歌可以在多个平台搜，用于：

- 目标平台没有高音质 → 去别家找；
- 目标平台未登录/搜不到 → 换平台；
- 需要比对时长/专辑版本（找录音室版而非现场版）。

并行时**不要超过 3 个平台**，结果统一按 `resource-matching.md` 的规则排序后再给用户。

## 常见空结果

| 现象 | 原因 |
| --- | --- |
| 全平台都空 | 关键词带特殊符号/歌词整句；换歌名 |
| 汽水报「汽水搜索需要本机汽水客户端提供签名…」 | 签名不可用。不是"没这首歌"，是"没法搜"→ 见 `qishui.md` |
| 汽水报「汽水搜索暂不可用：接口返回空响应…」 | 签名可用但上游返回空（风控）。换分享链接解析 |
| 网易云只有 128k | 未登录，或该曲需会员 |
| 酷我/酷狗某类空 | 关键词是英文别名，平台用中文名索引 |
