# MCP 工具参考

共 33 个工具，全部来自 `crates/hotdownloader-server/src/mcp.rs::tool_definitions()`。参数名区分大小写（camelCase）。

接入：`POST {BASE}/mcp?key=<key>`，Streamable HTTP，JSON-RPC 2.0。认证可用 `?key=` 或 Basic/Bearer。

## 音乐查询（12）

| 工具 | 参数 | 返回 |
| --- | --- | --- |
| `search_songs` | `platform`* `keyword`* `page`=1 `limit`=20 | `{songs:[…], has_more}` |
| `search_albums` | 同上 | `{albums:[…], has_more}` |
| `search_artists` | 同上 | `{artists:[…], has_more}` |
| `search_playlists` | 同上 | `{playlists:[…], has_more}` |
| `fetch_album_songs` | `platform`* `id`* `page` `limit` | `{songs:[…], has_more}` |
| `fetch_artist_songs` | `platform`* `id`* `page` `limit` | `{songs:[…], has_more}` |
| `fetch_artist_albums` | `platform`* `id`* `page` `limit` | `{albums:[…], has_more}` |
| `fetch_playlist_songs` | `platform`* `input`*（歌单链接或 ID） | `{songs:[…]}` |
| `parse_link` | `input`*（链接/分享文案）`platform`（兜底） | `{platform, kind, data}` |
| `fetch_hot_keywords` | `platform`* | 数组 |
| `fetch_suggestions` | `platform`* `keyword`* | `{song,singer,album,mv}` |
| `get_lyric_by_id` | `platform`* `songId`*（integer） | 歌词对象 |

`platform` 枚举（搜索类）：`qqmusic` `kuwo` `kugou` `netease` `qishui`。`parse_link` 的 `platform` 额外支持 `bilibili`。
`page` ≥ 1，`limit` 1–100（超出报「无效的分页参数」）。

注意事项：

- **汽水只支持 `search_songs`**：专辑/歌手/歌单搜索在服务端返回「未知音乐操作」；`fetch_hot_keywords` 返回 `[]`，`fetch_suggestions` 返回全空结构；`page > 1` 返回空列表（游标无法由页码推导）。
- **汽水搜索结果只带占位档位**（通常 `128kaac`），真实档位要 `parse_link` 或 HTTP `songs/detail` 补。
- **B 站不参与搜索**，只能 `parse_link`（返回 `kind: "song"`）。
- **`get_lyric_by_id` 对酷狗/汽水实际无效**：这两个平台按 `songMid` 取词，而工具 schema 只有 `songId`（integer）。需要它们的歌词时走 HTTP `POST /api/music/lyrics`，传 `{"platform":"kugou","songMid":"<hash>"}`。网易云/QQ/酷我用 `songId` 正常。
- 搜索结果里的 `platform` 字段由服务端补齐，可直接用于建任务。

## 分享链接解析

```json
{"input": "https://music.163.com/song?id=1974443814"}
```

返回：

```json
{"platform":"netease","kind":"song",
 "data":{"song":{"platform":"netease","id":1974443814,"mid":"1974443814",
   "title":"我记得","artist":"赵雷","album":"署前街少年",
   "qualities":[{"quality":"128kmp3","filename":"1974443814_standard.mp3","size":…},
                {"quality":"flac","filename":"1974443814_lossless.flac","size":65211331}]}}}
```

- `kind` ∈ `song` / `playlist` / `album` / `unknown`（短链常见 `unknown`，`data` 里按平台返回的实际结构判断：有 `songs` 就是列表）。
- 识别是**离线按 host 精确匹配**：网易云 `music.163.com`/`163cn.tv`、QQ `*.y.qq.com`、酷狗 `kugou.com`、酷我 `kuwo.cn`、B 站 `bilibili.com`/`b23.tv`、汽水 `qishui.douyin.com`（或 `music.douyin.com/qishui/`）。识别不出来的链接加 `platform` 兜底。
- 单曲结果可直接喂 `create_download_task`；歌单/专辑结果先取 `data.songs`，再逐首建任务。

> ⚠️ **不要把纯数字 ID 当单曲传给 `parse_link`。**
> 实测 `{"input":"186016","platform":"netease"}` 返回的是 `kind: "playlist"` —— 网易云里 id 186016 恰好也是一张歌单（159 首），拿到的完全不是"晴天"。
> 原因是离线只能按 host 判断类型，识别不出的一律按歌单兜底解析（源码注释：短链/无法判断交给歌单解析）。
> 要按 ID 取单曲，把 ID 拼成标准链接：`https://music.163.com/song?id=186016`。汽水没有单曲页链接时，用 `songMid` + HTTP `songs/detail`。

## 任务（7）

| 工具 | 参数 | 返回 |
| --- | --- | --- |
| `list_tasks` | 无 | `TaskRecord[]` |
| `create_download_task` | `song`* `desiredQuality`* `duplicateAction` `stream` | `{outcome, task?}` |
| `pause_task` | `taskId`* | `{ok:true}` |
| `resume_task` | `taskId`* | `{ok:true}` |
| `retry_task` | `taskId`* | `{retried:bool}` |
| `remove_task` | `taskId`* `deleteFile`=false | `{ok:true}` |
| `remove_tasks` | `taskIds`* `deleteFile`=false | `{succeeded, failed, errors[]}` |

### `create_download_task` 的 `song` 结构

```jsonc
{
  "platform": "netease",        // 必填
  "id": 1974443814,             // 必填，整数（汽水恒为 0，靠 mid 识别）
  "mid": "1974443814",          // 必填，字符串
  "title": "我记得",             // 必填
  "artist": "赵雷",              // 必填
  "album": "署前街少年",          // 必填
  "coverUrl": "https://…",      // 可选
  "mediaMid": "…",              // 可选（QQ/酷狗用它区分同一首歌的不同介质）
  "qualities": [                // 必填，不能为空
    {"quality": "flac", "filename": "1974443814_lossless.flac", "size": 65211331}
  ]
}
```

`qualities` 必须来自 `parse_link` / 搜索 / `songs/detail` 的原样输出，**不要手工修改 `filename` 或 `size`**——降级和断点续传都依赖它们与 `quality` 绑定。

### `stream`（可选，直链投递）

```jsonc
{"url":"https://…","playAuth":"…","filename":"320.m4a","size":8414936,"expiresAt":0}
```

`url` 与 `filename` 必填；`playAuth` 只用于加密流；`expiresAt` 为毫秒时间戳，0 表示未知。带上 `stream` 后服务端**不再请求平台**，音质取 `filename` 主干，所以 `desiredQuality` 只是占位。这是汽水下载的默认路径（见 `qishui.md`）。

## 设置（3）

| 工具 | 参数 | 说明 |
| --- | --- | --- |
| `get_settings` | 无 | `{revision, settings:{…}}` |
| `patch_settings` | `changes`* `expected`* | 字段级合并，`expected` 必须是每个改动字段的原值；对不上返回「设置冲突: 字段名」 |
| `get_default_download_dir` | 无 | 字符串路径 |

改设置前先 `get_settings` 拿原值，把原值原样放进 `expected`。**字段在 `settings` 里没出现时，`expected` 传 `null`**（说明该字段还没落盘、走内置默认值；实测线上只存了 `artistSeparator` `downloadDir` `namingTemplate` `writeMetadata` 四项）。常用字段：`defaultQuality` `autoDowngrade` `qualityDowngradeOrder` `duplicateStrategy`(`ask`/`overwrite`/`rename`/`cancel`) `maxConcurrent`(1–32) `namingTemplate` `writeMetadata` `downloadLrc` `downloadCover`。

冲突返回 `409` + `{error, fields, snapshot}`；拿到后按新 `revision` 重新取原值再改，不要盲目重试。

## 账号（7）

| 工具 | 参数 | 说明 |
| --- | --- | --- |
| `list_accounts` | 无 | `{accounts:[{platform, platformName, loggedIn, account, supportsQrLogin, supportsCookieLogin, note}]}` |
| `get_login_status` | 无 | 仅 QQ 音乐 |
| `start_qr_login` | `platform`*（`qqmusic`/`netease`/`kugou`） | 返回 PNG 图片 + `{sessionId, imagePath, imageUrl, expiresInSeconds}` |
| `check_qr_login` | `platform`* `sessionId`* | `{status, done, saved, account}` |
| `save_cookie` | `platform`*（`netease`/`qishui`）`cookie`* | 网易云立即校验；汽水只做格式校验 |
| `login_with_credentials` | `platform`*（`qqmusic`/`kugou`）+ 各自字段 | QQ: `uin`+`authst`（可选 refreshToken/refreshKey/accessToken/openid）；酷狗: `userid`+`token`（可选 mid/dfid） |
| `logout_account` | `platform`*（四个账号平台） | 汽水等于清 Cookie |

扫码流程：`start_qr_login` 拿到 PNG → 把图片发给用户扫 → 每 2 秒 `check_qr_login`，`done: true` 停止。`confirmed` 时凭据已自动写盘，**不要再调 `save_cookie`**。汽水没有扫码，见 `qishui.md`。

## 曲库（4）

| 工具 | 参数 | 说明 |
| --- | --- | --- |
| `list_library` | `keyword` `offset`=0 `limit`（1–500） | `{directory, total, files:[{name, path, size, extension}]}` |
| `delete_library_files` | `paths`*（`list_library` 返回的路径） | `{deleted:number, failed:[{path,error}]}` |
| `rename_library_file` | `path`* `newName`* | 自动清洗非法字符，可省略扩展名沿用原扩展名 |
| `write_library_metadata` | `path`* + `title` `artist` `album` `lyrics` | 只改传入字段；空字符串表示清除 |

曲库作用于**服务端下载目录**，不是任务记录。改名与写标签**只有 MCP 有**，HTTP 无对应接口。

## 调用建议

- `search_*` 的 `limit` 用 10–20 即可，信息足够排序；把 `limit` 开到 100 只会浪费上下文。
- 工具返回是 `content[0].text` 里的 JSON 字符串，需要解析后再判断；`isError: true` 时文本是错误原因。
- 没有 `cancel_task` 工具：取消就是 `remove_task`（服务端 `remove_task` 内部走 `cancel_task`，会停下载并删记录）。
