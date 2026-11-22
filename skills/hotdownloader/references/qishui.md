# 汽水音乐（qishui）接入细节

主文档 `SKILL.md` 讲通用的搜索/解析/下载；这里只写汽水**不一样的地方**。

三句话概括：

1. **没有网页版**，`www.qishui.com` 只是下载落地页，分享 H5 也没有登录入口 —— 所以"浏览器登录后抠 Cookie"不成立，只能读本机桌面客户端的 Cookie 库；
2. **扫码已废弃**：上游 README 明写「扫码登录暂未调通」，`GetQRLoginSourceNames()` 也不含 soda，实测轮询几次就持续 `error_code=7`，不要尝试、也不要在应用里实现；
3. **Cookie ≠ 能用**：搜索、`hi_res`/`spatial` 档位、会员曲整曲都还要 `x-helios`/`x-medusa` 签名，而签名只能由客户端里的 `bdms.node` 生成。

## 能力对照（有 Cookie 和没签名的区别）

| 能力 | 无 Cookie 无签名 | 有 Cookie 无签名 | Cookie + 签名 |
| --- | --- | --- | --- |
| 分享链接解析（元数据/封面/歌词） | ✅ | ✅ | ✅ |
| 搜索 | ❌ 空响应 | ❌ **仍然空响应** | ✅ 真实结果 |
| 免费曲目整曲 | ✅ 最高 320kaac | ✅ 最高 320kaac | ✅ 多出 `spatial` / `hi_res` |
| 会员曲目 | ⚠️ 60s | ⚠️ 60s | ⚠️ 仍 60s（除非账号是会员） |
| 可选档位数 | 3 档 AAC | 3 档 AAC | 4~5 档 |

两个容易误判的点：

- 缺签名的响应是 **`HTTP 200` + 空 body**（不是 401/403），看起来像"没搜到"而不是"被拒绝"；
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
4. 读 Cookie 并落库（脚本自动探测三平台路径，Windows 下用共享标志绕过客户端独占锁，**不用关客户端、不改任何文件**）：

   ```bash
   python scripts/qishui_login.py client --mcp
   # 只读不写：去掉 --mcp；走 REST：--write；指定库：--cookie-db <path>
   ```

5. 判据：读到的键名必须含 `sessionid` / `sid_guard`；`qishui_login.py status` 应返回 `configured: true`。

该版本客户端的 `value` 列是明文；若某天变成 `v10` 加密，脚本会明确报错（需要 DPAPI + AES-GCM，未实现）。

### 3. 配签名器（本机自动，Docker 走远程）

- **本机**（服务直接跑在 Windows，不经 Docker）：进程内调 `bdms.node`。运行时优先 `QISHUI_NODE`，其次 PATH 的 `node`，最后客户端自带 Electron。装过客户端就自动可用，`mode: local`。
- **远程**（Docker/服务器）：`QISHUI_SIGNER_URL` 指向宿主机签名服务，`mode: remote`。详见下一节。

签名是**逐请求**的（绑定 url / method / body / headers / ts_ms），不能跨请求复用；能缓存的是设备身份（进程内）与详情结果（2 分钟）。

## 要不要常驻一个签名服务？分场景

| 场景 | 需要 8899 常驻进程吗 |
| --- | --- |
| Web UI 解析分享链接 + 下载（3 档 AAC） | ❌ 不需要 |
| Web UI 想看到 `spatial` / `hi_res` 档位 | ✅ 需要（服务端自己取详情才有多档） |
| agent 在服务端搜汽水（MCP `search_songs`） | ✅ 需要 |
| agent 下汽水（本机解析 → 直链投递） | ❌ 不需要 |
| 其它平台的全部操作 | ❌ 不需要 |

**推荐做法：不常驻。** 解析放在本机、一次性完成，把直链投给服务器：

```bash
python scripts/qishui_stream.py <track_id>
# 输出 {"url":"…","playAuth":"…","filename":"320.m4a","size":8414936,"expiresAt":0}
# 直接作为 create_download_task 的 stream 参数
```

脚本用 `node -e` 调一次 `bdms.node` 就退出，不监听端口、不占内存、不需要保活。换句话说：**装客户端 + Node 仍然要，但"一直开着终端"可以不要**。

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

验证：`curl -u <账号>:<密码> http://127.0.0.1:8791/api/qishui/signer` 应返回 `{"available":true,"mode":"remote"}`；账号页汽水卡片的「签名器」一行也会显示对应状态。

## 公网部署：直链投递（服务端零签名、零 Cookie）

公网服务器上没有 `bdms.node`，也可能不该持有你的账号。让**能签名的机器先把音源解析出来**，再把直链随任务投给服务器：

```jsonc
// create_download_task 的 stream 参数
{"song": { /* parse_link 结果 */ }, "desiredQuality": "320kaac",
 "stream": {"url": "https://v5-xxx.douyinvod.com/…", "playAuth": "",
            "filename": "320.m4a", "size": 8414936, "expiresAt": 0}}
```

1. **优先 `hi_res` / `highest`，别选 `spatial`**：实测 `spatial` 的地址在无 Cookie 的服务器上会被降级成低档（字节数对不上）；选 `hi_res` 时服务端拿到的 8,414,936 与本机一致。
2. **`expiresAt` 过期**：建任务时直接标错误并提示重新解析；排队久了要重投。
3. **服务器可完全不配 Cookie**：CDN 直链与免签解析都不需要登录态，公网机器被入侵也拿不到你的账号。
4. **代价**：服务器失去"降级重取"能力，失败只能请本机重投。

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
| `client` 报缺 `sessionid` | 客户端未登录 | 先在客户端里登录 |
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
