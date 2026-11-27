# 排错

先判断**在哪一层出错**，再查对应表。改参数重试是最后手段，不是第一反应。

```
1 服务层    curl $BASE/healthz 通不通
2 通道层    MCP 连得上吗（list_accounts 有返回吗）
3 认证层    list_accounts 里各平台 loggedIn
4 查询层    搜索/解析能否拿到数据
5 任务层    create 的 outcome、任务的 status/errorMsg
6 文件层    磁盘、路径、占用
```

## 服务层

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| `curl $BASE/healthz` 连不上 | 容器没起 / 端口不对 / 地址推导错 | 确认容器状态；`$BASE` 只从 MCP 接入地址推导，兜底看 `.env` 的 `WEB_PORT` |
| 401 `访问凭据无效` | Basic 凭据错 / token 错 / MCP key 错 | `.env` 的 `AUTH_USERNAME/AUTH_PASSWORD`，或数据目录 `auth.json`；MCP 用 `?key=` |
| 页面能开但 API 401 | 浏览器有会话、curl 没带认证 | curl 加 `-u user:pass` |
| `/api/*` 全 404 | 路径拼错（如用了 DELETE 删任务） | 任务删除是 `POST /api/tasks/{id}/remove` |
| 改密码后所有客户端掉线 | 正常 | 更新 `.env`/`mcp.json` 里的凭据 |

## 通道层（MCP）

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| 工具列表里没有 hotdownloader | 新加的 MCP 不会热加载 | 在连接器管理点"信任"，重开会话 |
| MCP 工具报认证失败 | `?key=` 过期（被重置） | `GET /api/mcp` 取新 key，更新 `mcp.json` |
| MCP 超时但 HTTP 正常 | 网络/代理抖动 | 用 HTTP 同一操作**只重试一次**；写操作先 `list_tasks` 查是否已生效 |
| 明明有工具却调不通 | 用了旧文档里的工具名 | 以 `references/mcp-tools.md` 为准，工具名都是当前源码里的 |

## 查询层

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| 搜索返回空列表（全平台） | 关键词太怪（歌词整句/特殊符号） | 换成"歌手 + 歌名" |
| 只有汽水报错「汽水搜索需要本机汽水客户端提供签名…」 | 签名不可用（上游是 `200 + 空 body`，服务端已翻译成可执行错误） | `GET /api/qishui/signer` 看 `available`；装客户端并配签名，或改用分享链接解析（见 `qishui.md`） |
| 汽水报「汽水搜索暂不可用：接口返回空响应…」 | 签名可用但上游风控返回空 | 换分享链接解析，或稍后再试 |
| 汽水第 2 页空 | 游标无法由页码推导，服务端只给首屏 | 换关键词，不要翻页 |
| 汽水专辑/歌手/歌单搜索报「未知音乐操作」 | 该平台没有这些能力 | 用歌曲搜索，或换平台 |
| 网易云只有 128kmp3 | 未登录 / 该曲需会员 | `list_accounts` → `save_cookie` |
| `parse_link` 报「未能提取」或「暂不支持该链接」 | 不是受支持的链接，或链接被截断 | 让用户重新复制完整分享文案；或加 `platform` 兜底 |
| `parse_link` 返回 `kind: unknown` | 短链离线无法判断类型 | 看 `data` 里有没有 `songs`；再用 `fetch_playlist_songs` |
| `parse_link` 传纯 ID 却返回 `kind: playlist` + 一大堆歌 | 纯 ID 无法离线判断类型，服务端按歌单兜底解析（实测 id 186016 解析出另一张 159 首歌单） | 拼标准链接：`https://music.163.com/song?id=<id>`；汽水用 `songMid` + `songs/detail` |
| `page`/`limit` 报「无效的分页参数」 | `page` 为 0 或 `limit` 超 100 | 用 1–100 |
| 酷狗/汽水歌词为空 | `get_lyric_by_id` 只传了 `songId`，这两家要 `songMid` | 走 HTTP `POST /api/music/lyrics` 带 `songMid` |

## 任务层

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| outcome `needs_confirmation` | 目标文件已存在 | 问用户后带 `duplicateAction` 重调一次 |
| outcome `existing` | 已有同曲任务 | 看旧任务状态，不要当失败 |
| 建完立刻 `error`「所选音质不可用」 | `desiredQuality` 传了文件名，或该曲没这档 | 传标签；先 `parse_link` 看 `qualities` |
| 建完立刻 `error`「直链已过期」 | 汽水直链过期 | 回本机重新解析投递 |
| 建时 `waiting`，一会儿变 `error`「音质候选已耗尽，最后音质 X 获取链接失败: …」 | `qualities` 声明有档位，但实际取流被拒（未登录/无权限/下架） | 看冒号后的原因：账号无权限 → 登录；`result=104003` → 会员或下架；换平台是常见解法 |
| `error` 任务的 `quality` 比目标低很多 | 降级链走到了底，不是用户选了低档 | 汇报时写清"从 flac 降到 128kmp3 都取不到" |
| 「获取下载链接失败: <原因>」 | 未进入降级就失败 | 同上，按冒号后原因处理 |
| 「同名任务正在下载，请选择保留两份」 | `overwrite` 撞上正在写入的文件 | 改 `rename` 或等那条完成 |
| 一直 `waiting` | 并发位满 / 队列暂停 | 看 `maxConcurrent`；队列暂停只能界面恢复 |
| `processing` 很久 | 收尾在写标签/下载歌词封面 | 正常，别重试；超过几分钟看磁盘与占用 |
| `pause` 报「只有等待或下载中的任务可以暂停」 | 状态不对 | 先 `list_tasks` 看实际状态 |
| `resume` 报「下载队列已暂停，请先恢复队列」 | 队列级暂停 | 请用户在界面恢复队列 |
| `retry_task` 返回 `retried: false` | 状态不是 `error` | 不是错误，无需处理 |
| 「此歌曲已有未结束的下载任务」 | 同曲还有别的任务在跑 | 处理那条，不要重试本条 |
| 「重试次数已用尽」「已无后续可用品质」 | 该平台拿不到更好档位 | 换平台或接受当前档 |
| 「下载收尾失败」 | 文件被占用 / 磁盘满 / 标签写入失败 | 释放占用、清空间；这类不参与自动降级 |

## 文件层

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| 完成但曲库里找不到 | 文件被移动/改名，或不在默认下载目录 | 用任务 `filePath` 确认实际位置 |
| 文件名被截断/带序号 | 超长或重名策略 | `pathWarning` 有说明；必要时 `rename_library_file` |
| flac 只有 1–2MB | 试听片段或降级流 | 会员曲非会员无解；否则 `parse_link` 复查档位 |
| 删文件失败 | 被播放器占用 / 非音频扩展名 / 不在下载目录 | 关闭占用程序；用 `list_library` 返回的路径 |
| 改名报错 | 目标名已存在，或含非法字符 | 换名；非法字符会自动替换成下划线 |

## 汽水相关

全部在 `references/qishui.md`，先跑：

```bash
python scripts/qishui_doctor.py
```

它会依次检查 Cookie、客户端、`bdms.node`、Node、服务连通、签名器状态，直接告诉你卡在哪一步。

## 系统化排查（用户说"什么都下不了"）

```
1 healthz            → 服务活着吗
2 list_accounts      → 是不是全都没登录（Cookie 一起过期很常见）
3 搜一首确知的歌      → 搜索链路通不通
4 parse_link 同一首   → 解析链路通不通
5 建一个任务          → outcome 是什么
6 list_tasks         → errorMsg 原文
7 换平台重复 3–5      → 单平台问题还是全局问题
```

每一步只记事实，不要在同一层反复试。定位到具体层再改。
