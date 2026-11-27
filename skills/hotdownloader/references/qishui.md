# 汽水音乐（qishui）接入细节

主文档 `SKILL.md` 讲通用的搜索/解析/下载；这里只写汽水**不一样的地方**。

**遇到汽水问题先跑自检**，它会告诉你卡在哪一步：

```bash
python scripts/qishui_doctor.py
```

只想知道"怎么下"→ 直接看「下载默认路径：本机解析直链」；只想"能搜到"→ 看「要不要常驻一个签名服务」。

三句话概括：

1. **没有网页版**，`www.qishui.com` 只是下载落地页，分享 H5 也没有登录入口 —— 所以"浏览器登录后抠 Cookie"不成立，只能读本机桌面客户端的 Cookie 库；
2. **扫码已废弃**：上游 README 明写「扫码登录暂未调通」，`GetQRLoginSourceNames()` 也不含 soda，实测轮询几次就持续 `error_code=7`，不要尝试、也不要在应用里实现；
3. **Cookie ≠ 能用**：搜索、`hi_res`/`spatial` 档位、会员曲整曲都还要 `x-helios`/`x-medusa` 签名，而签名只能由客户端里的 `bdms.node` 生成。

## 能力对照（有 Cookie 和没签名的区别）

| 能力 | 无 Cookie 无签名 | 有 Cookie 无签名 | Cookie + 签名 |
| --- | --- | --- | --- |
| 分享链接解析（元数据/封面/歌词） | ✅ | ✅ | ✅ |
| 搜索 | ❌ 报「需要本机…提供签名」 | ❌ 仍报错 | ✅ 真实结果 |
| 免费曲目整曲 | ✅ 最高 320kaac | ✅ 最高 320kaac | ✅ 多出 `spatial` / `hi_res` |
| 会员曲目 | ⚠️ 60s | ⚠️ 60s | ⚠️ 仍 60s（除非账号是会员） |
| 可选档位数 | 3 档 AAC | 3 档 AAC | 4~5 档 |

两个容易误判的点：

- 缺签名时**上游**返回 `HTTP 200` + 空 body（不是 401/403）。但**服务端已经把这种空响应翻译成明确错误**给你：
  - 签名不可用 → `汽水搜索需要本机汽水客户端提供签名；未安装客户端时请改用分享链接解析`
  - 签名可用但上游仍空 → `汽水搜索暂不可用：接口返回空响应，请改用分享链接解析`

  所以看到这两条，别当成"没搜到这首歌"去换关键词，那是通道问题。
- 签名只改变「请求身份」，不改变曲目权限 —— 会员曲目 60s 是账号问题。

最小安装集：**汽水音乐客户端（已登录）**。读 Cookie 不需要 Node；Node 只服务于签名，且可用客户端自带 Electron 顶替。

## 接入流程：三步

> 先跑自检，它会把下面三步全验一遍：
> ```bash
> python scripts/qishui_doctor.py
> ```

### 1. 确认在用户本机执行

Cookie 只能从本机客户端读，签名只能由本机 `bdms.node` 生成。**如果 agent 跑在服务器/容器里，这套流程走不通**，改走「公网部署：直链投递」一节，并说明是降级方案。

### 2. 装客户端并取 Cookie

1. **先问一句（仅 Windows）**：「你知道汽水音乐装在哪个目录吗？不确定的话我自动找。」
2. 不知道 / macOS / Linux → 直接搜，不要再问：
   - Windows：`%LOCALAPPDATA%\Programs\Soda Music\<版本>\resources\app.asar.unpacked\bdms.node`（`Packages\<版本>\` 下也有一份）；
   - macOS：`/Applications/SodaMusic.app/Contents/**/bdms.node`；
   - 也可显式指定 `QISHUI_CLIENT_DIR` 或 `QISHUI_BDMS_PATH`。
3. **找不到客户端就到此为止**：「本机没有安装汽水音乐客户端，无法获取 Cookie 和签名；请先安装并登录（`https://www.qishui.com/download`）。」
4. 读 Cookie 并落库（脚本自动探测三平台路径，Windows 下用共享标志绕过客户端独占锁，正常情况下**不用关客户端、不改任何文件**）：

   ```bash
   python scripts/qishui_login.py client --mcp
   # 只读不写：去掉 --mcp；走 REST：--write；指定库：--cookie-db <path>
   ```

   **例外：客户端运行时可能读不到。** 客户端会把最新登录态压在 WAL 里、库也可能被占用，此时脚本读到的是空值/旧值或缺 `sessionid`。脚本会自己探测进程，并明确提示：

   > 检测到汽水音乐客户端正在运行：SodaMusic.exe(PID …)。……请完全退出客户端：托盘图标右键 → 退出（任务管理器里确认没有 SodaMusic 进程），再重跑本条命令。

   照做即可（**托盘也要退，只关主窗口不算**）；读完 Cookie 再把客户端打开，不影响之后使用。agent 遇到这个提示**不要反复重试**，先让用户退客户端，再跑一次。

5. 判据：读到的键名必须含 `sessionid` / `sid_guard`；`qishui_login.py status` 应返回 `configured: true`。

该版本客户端的 `value` 列是明文；若某天变成 `v10` 加密，脚本会明确报错（需要 DPAPI + AES-GCM，未实现）。

### 3. 配签名器（本机自动，Docker 走远程）

> 只下载**不需要**配服务端签名器：下载走下面「本机解析直链」，签名在解析那一刻就完成了。这里配的是**服务端**签名，服务于 `search_songs`、`parse_link` 看多档详情。

- **本机**（服务直接跑在 Windows，不经 Docker）：进程内调 `bdms.node`。运行时优先 `QISHUI_NODE`，其次 PATH 的 `node`，最后客户端自带 Electron。装过客户端就自动可用，`mode: local`。
- **远程**（Docker/服务器）：`QISHUI_SIGNER_URL` 指向宿主机签名服务，`mode: remote`。详见下一节。

签名只依赖磁盘上的 `bdms.node`，**客户端开着也能签**；需要退出客户端的是上一步「读 Cookie」。推荐顺序：退客户端取 Cookie → 落库 → 再打开客户端 → 配/测签名器。

签名是**逐请求**的（绑定 url / method / body / headers / ts_ms），不能跨请求复用；能缓存的是设备身份（进程内）与详情结果（2 分钟）。

## 下载默认路径：本机解析直链，服务端只下载

**agent 下汽水，一律先本机出直链。** 服务端（Docker / 公网）没有 `bdms.node`，让它自己签名取流只能拿到低档；本机有 Cookie + `bdms.node`，能拿到 `hi_res` / `spatial` 并把 CDN 直链直接交给服务端。

| 场景 | 走哪条 | 要 8899 常驻吗 |
| --- | --- | --- |
| **agent 下汽水** | 本机 `qishui_stream.py` → `stream` 投递 | ❌ |
| Web UI 下汽水（3 档 AAC） | 服务端直接请求 | ❌ |
| 服务端搜汽水（MCP `search_songs`）/ 看 `hi_res` 档位 | 服务端签名（常驻或 remote） | ✅ |
| 其它平台的全部操作 | 服务端自签 | ❌ |

标准三步：

```bash
# 1) 本机解析（一次性，装客户端 + Node 即可，用完即退）
python scripts/qishui_stream.py <track_id>
# stderr：档位 hi_res · 8414936 字节 · 320.m4a
# stdout：{"url":"…","playAuth":"…","filename":"320.m4a","size":8414936,"expiresAt":0}
```

```jsonc
// 2) 直链原样作为 create_download_task 的 stream
{"song": { /* parse_link 结果 */ }, "desiredQuality": "320kaac",
 "stream": {"url": "…", "playAuth": "…", "filename": "320.m4a", "size": 8414936, "expiresAt": 0}}
// 3) list_tasks 看进度；失败且提示过期就回第 1 步重新解析
```

要点：

- **Cookie 来源**：`--cookie` > `QISHUI_COOKIE` > `data/qishui-credentials.json` > 本机客户端库（`--cookie-from-client` 强制走最后一条）。客户端在跑且读不到会提示先退客户端。
- **设备身份**：默认读 `%APPDATA%\HotDownloader\qishui-identity.json`；没有就按毫秒生成并落盘，和应用侧一致，不需要先跑应用。
- **风格**：`--prefer` 默认 `hi_res`（实测 `spatial` 在无 Cookie 的服务器上会被降级）；`--filename` 可覆盖输出名。
- **带 `stream` 时服务端不再校验音质、也不再请求平台**：任务音质取 `stream.filename` 的主干，所以 `desiredQuality` 只是占位。
- **过期**：`expiresAt` 到点后建任务会直接是 `error`（提示重新投递）；解析完尽快投。
- 只有本机没有客户端 / `bdms.node` 时，才退回让服务端自签（低档、无搜索）。

脚本用 `node -e` 调一次 `bdms.node` 就退出，不监听端口、不占内存、不需要保活。换句话说：**装客户端 + Node 仍然要，但"一直开着终端"可以不要**。

## 要不要常驻一个签名服务？分场景

常驻只服务于**服务端侧的搜索与详情**（`search_songs`、想看到多档 `qualities`）。纯下载不需要它——那部分已被上面的直链投递覆盖。

```bash
# 需要常驻时才跑（宿主机，不是容器）
QISHUI_SIGNER_TOKEN=<自定义token> node scripts/qishui-signer-server.mjs
```

代价是宿主机上那个 node 终端不能关：一关容器就搜不到（不报错，只是静默退化）。所以推荐：**默认不常驻，只有要 agent 在服务端搜汽水时才开**。

## 签名服务跑在哪：宿主机，不在容器里

（只有选择常驻方案时才需要看这一节。）

`host.docker.internal` 是 Docker 给「宿主机」的固定域名，指容器外面那台电脑：

| 组件 | 跑在哪 | 原因 |
| --- | --- | --- |
| hotdownloader（Web UI + API） | 容器 | 单镜像部署 |
| 签名服务 `qishui-signer-server.mjs` | **宿主机** | 要 `require` `bdms.node`（PE 原生插件），Linux 容器加载不了 |
| 汽水客户端 | **宿主机** | 提供 Cookie 库与 `bdms.node` |

「该进程需保持在线」= **宿主机上那个 node 终端不能关**；它跟容器生命周期无关，一关容器就搜不到（不报错，只是静默退化）。

```bash
# 宿主机（Git Bash）
QISHUI_SIGNER_TOKEN=<自定义token> node scripts/qishui-signer-server.mjs
# 监听 0.0.0.0:8899；宿主机自测 http://127.0.0.1:8899/config，容器内用 host.docker.internal:8899
```

容器侧（`docker-compose.yml` 已透传 + `host.docker.internal:host-gateway`）：

```
QISHUI_SIGNER_URL=http://host.docker.internal:8899/sign
QISHUI_SIGNER_TOKEN=<自定义token>
```

验证：`curl -u <账号>:<密码> $BASE/api/qishui/signer` 应返回 `{"available":true,"mode":"remote"}`；账号页汽水卡片的「签名器」一行也会显示对应状态。`$BASE` 由 MCP 接入地址去掉 `/mcp?key=…` 得到（见 SKILL.md「第 0 步：确认服务可用」），不要写死端口，跟着 MCP 接入地址走。

## 直链投递的收益与代价

收益：服务端零签名、零 Cookie——公网机器被入侵也拿不到你的账号。代价：服务端失去「降级重取」能力，失败只能请本机重新解析投递。

1. **优先 `hi_res` / `highest`，别选 `spatial`**：实测 `spatial` 的地址在无 Cookie 的服务器上会被降级成低档（字节数对不上）；`hi_res` 时服务端拿到的 8,414,936 与本机一致。
2. **`expiresAt` 过期**：建任务时直接标错误并提示重新解析；排队久了要重投。
3. **服务器可完全不配 Cookie**：CDN 直链不需要登录态。
4. **搜索仍走服务端**：`search_songs` 没有 `bdms.node` 时只能免签兜底；搜不出来就先本机解析分享链接拿 `mid`，再按上面的方式投直链。

## 代码位置（维护用）

| 关注点 | 位置 |
| --- | --- |
| Cookie 存储 | `qishui/credentials.rs`（`qishui-credentials.json`，`Status` 只暴露 configured/length/keys） |
| 本机签名器 | `qishui/signer.rs` + `qishui/assets/qishui-bridge.cjs`；设备身份 `<配置目录>/HotDownloader/qishui-identity.json` |
| App 端接口 | `qishui/app.rs`：`fetch_pc_track` / `fetch_pc_search`；`signed_ready()` = Cookie 非空 且 签名可用 |
| 详情与取流 | `qishui/parser.rs::fetch_track` → `TrackBundle{payload, players, signed}`，2 分钟缓存 |
| 搜索链路 | `qishui/search.rs`：签名 PC → 免签 web → Android，三级兜底 |
| 状态接口 | `GET /api/qishui/cookie`、`GET /api/qishui/signer` |
| 前端 | `qishuiApi.ts`、`QishuiCookieSetting.vue`（Cookie + 签名器两段）、`AccountView.vue` 状态条 |
| 下载前补档位 | `useDownloadActions.ts::withQishuiQualities` → `POST /api/music/songs/detail`（失败静默回退） |

## 排错（汽水专属）

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| `client` 报"未找到 Cookie 库" | 没装 / 没登录过 / 非默认路径 | 装并登录客户端，或 `--cookie-db` 指定 |
| `client` 报"客户端正在运行 / 库是空的 / 缺 `sessionid`" | 客户端没退干净，Cookie 库被占或登录态没落盘 | 托盘右键完全退出客户端（任务管理器确认无 SodaMusic），再重跑一次 |
| `client` 报"解析 Cookie 库失败 / file is not a database" | 读到的库不完整（客户端正在写，或只拷了主库没带 WAL） | 退出客户端后重跑；别只拷 `Cookies` 文件 |
| `qishui_stream.py` 报 HTTP 4xx / 没返回 `url_player_info` | 用的 Cookie 已过期，或设备身份与签名不匹配 | `--cookie-from-client` 重试；仍失败删 `qishui-identity.json` 后重跑 |
| `qishui_stream.py` 报"未找到 bdms.node / node" | 没装客户端或没装 Node | 装客户端（或 `QISHUI_BDMS_PATH`）；`QISHUI_NODE` 指到自带 Electron |
| `client` 报缺 `sessionid`（已退客户端） | 客户端里确实没登录 | 打开客户端登录，再退出客户端后重读 |
| Cookie 存了但搜索为空 | 缺签名 | `GET /api/qishui/signer` 看 `available`，起签名服务 |
| 签名器 `available:false` | 没 `bdms.node` / 没 JS 运行时 / 远程 URL 没配 | 装客户端或 Node；或配 `QISHUI_SIGNER_URL` |
| 报"空响应（签名头或设备身份未被接受）" | 设备身份与签名器不一致 | 删 `qishui-identity.json` 后重启进程 |
| 下载提示"该音质不可用" | 请求档位不在当前变体里（常见于无签名） | 先确认签名，再看解析结果的 `qualities` |

## 安全红线

- Cookie 只落运行时数据目录，不进设置快照、任务记录、日志或 API 响应；临时副本 0600，落库后删除。
- 终端/对话里不打印完整 Cookie；`.workbuddy/`、`data/`、`.env` 已 gitignore，别提交截图。
- 远程签名服务能看到完整请求头（含 Cookie）：只在内网/本机跑，必须设 token，不要暴露公网。

## 参考

- 一次性解析脚本：`scripts/qishui_stream.py`（直链投递用，无常驻进程）
- 签名与 App 端请求契约：`references/signer.md`
- 上游实现：`go-music-dl`（`core/soda_client.go`）与 `music-lib/soda`（`signature.go`、`localsigner.go`、`search.go`、`download.go`）
