# 汽水签名器与 App 端请求契约

来源：`go-music-dl` + 其依赖 `music-lib/soda`（`signature.go`、`localsigner.go`、`download.go`、`search.go`），以及本项目的实测结果（2026-11-22）。

## 1. 为什么需要签名

- 会员整曲、`hi_res`/`spatial` 档位、搜索都走 App 端接口，要求逐请求的 `x-helios` / `x-medusa`；
- 这两个头只能由官方原生组件 `bdms.node`（底层 mssdk / metasecml.dll）生成，没有可复刻的纯软件实现；
- 缺口令时服务端返回 **`HTTP 200` + 空 body**，不是 401/403——所以现象像"没有结果"而不是"被拒绝"。

## 2. 本机签名：怎么调 bdms.node

`bdms.node` 由汽水客户端提供（`.../resources/app.asar.unpacked/bdms.node`），Node 直接 `require` 即可，只暴露三个方法：

```js
const bdms = require('bdms.node');
bdms.init({ deviceId });                       // 每进程一次
const raw = bdms.generateHttpSignatureHeaders(url, headerLines);
// raw 形如 "X-Helios\r\n<h>\r\nX-Medusa\r\n<m>"
```

`headerLines` 是「名称 \r\n 值 \r\n 名称 \r\n 值」拼成的文本，**必须包含实际请求里的全部头（含 Cookie）**，否则签名与实际请求不匹配。

本项目把这段逻辑放在 `qishui/assets/qishui-bridge.cjs`：stdin 收 `{bdms, device_id, url, method, body, headers}`，stdout 回 `{headers:{X-Helios,X-Medusa}}`，由 `signer.rs` 用 20 秒超时的子进程调用。

设备身份（`device_id` / `iid` / `fp`）必须与请求参数、签名三者一致，首次生成后缓存到 `<配置目录>/HotDownloader/qishui-identity.json`。

## 3. 远程签名服务契约（Docker / 服务器）

实现见 `scripts/qishui-signer-server.mjs`：

**`GET /config`**

```json
{"device_id":"1795352913298","identity":{"device_id":"…","iid":"…","fp":"…"}}
```

**`POST /sign`**

```json
{"url":"https://api.qishui.com/luna/pc/track_v2?...","method":"POST",
 "body":"{\"track_id\":\"...\"}","ts_ms":1795352913000,
 "headers":{"User-Agent":"LunaPC/3.3.0(359450208)","Cookie":"..."}}

→ {"ok":true,"headers":{"X-Helios":"…","X-Medusa":"…"}}
→ {"ok":false,"error":"unauthorized"}
```

兼容 Meting 风格的扁平回包 `{"ok":true,"X-Helios":"…","X-Medusa":"…"}`；若回包带 `ms_token` / `a_bogus`，客户端需把它们追加到 URL 查询串（本项目 `signer.rs` 会做）。

鉴权：`Authorization: Bearer <QISHUI_SIGNER_TOKEN>`（未设 token 则不校验，不建议）。

## 4. App 端整曲取流（`POST /luna/pc/track_v2`）

查询参数（`device_platform=windows` 的一整套 App 参数，`device_id/iid/fp` 来自签名器）：

```
aid=386088&app_name=luna_pc&region=cn&geo_region=cn&os_region=cn&sim_region=
&device_id=…&cdid=&iid=…&version_name=3.3.0&version_code=30030000&channel=official
&build_mode=master&network_carrier=&ac=wifi&tz_name=Asia/Shanghai&resolution=
&device_platform=windows&device_type=Windows&os_version=Windows 11&fp=…
```

请求体：

```json
{"track_id":"7689447702921349121","media_type":"track",
 "queue_type":"favorite_track_playlist","scene_name":"library"}
```

请求头（都会参与签名）：

```
User-Agent: LunaPC/3.3.0(359450208)
Content-Type: application/json; charset=utf-8
X-SS-STUB: <请求体 MD5 的大写十六进制>
x-luna-background-type: foreground
x-luna-is-background-req: 0
x-luna-is-local-user: 1
Cookie: <整串>
```

响应结构与 web 端点一致（`track` + `track_player.video_model` / `url_player_info`），但变体是完整时长、且包含 `spatial` / `hi_res`。

## 5. App 端搜索（`GET /luna/pc/search/track`）

同样的 App 参数，额外带 `q` / `cursor` / `search_id`（UUID v4）/ `search_method=input` / `debug_params` / `from_search_id` / `search_scene`，版本用 `3.8.0` / `38000000`，UA 用 `LunaPC/3.8.0`。

响应与 web 搜索同构：`result_groups[].data[].entity.track`。

## 6. 实测结论（2026-11-22）

| 场景 | 结果 |
| --- | --- |
| 无签名 `POST track_v2` | `200` + 空 body |
| 签名后 `POST track_v2`（免费曲目） | 19.7KB，210s 完整变体：`spatial` 8.5MB、`hi_res` 320k 8.4MB 等 |
| 签名后 `POST track_v2`（会员曲目，非会员账号） | 仍 60s（权限问题，非签名问题） |
| 无签名搜索 | `200` + 空 body |
| 签名后搜索「周杰伦」 | 20 条，126KB |
| App 端 `quality=lossless` + `format=mp4` 变体 | 实为整支 MV（`ftypisom...mp41`，33MB），已在解析时过滤 |

## 7. 降级顺序

1. 免签详情（`GET pc/track_v2` → `seo_track`）——保证元数据/歌词/封面总能拿到；
2. 需要完整流且签名可用 → 补一发 App 端取流，把变体并到列表前面；
3. 签名不可用 / App 端失败 → 保持免签结果，日志记一条 warn，不阻塞任务。
