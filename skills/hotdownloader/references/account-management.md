# 账号管理

四个平台有登录态，方式各不相同。**先 `list_accounts` 看现状，再选方式**，不要按印象直接让用户扫码。

## 能力矩阵

| 平台 | 扫码 | Cookie | 凭据 | 不登录的后果 |
| --- | --- | --- | --- | --- |
| `qqmusic` | ✅ | ❌ | ✅ `uin` + `authst` | 会员曲/高音质拿不到，详情接口部分受限 |
| `netease` | ✅ | ✅ | ❌ | 只能 128kmp3，部分曲目搜不到 |
| `kugou` | ✅ | ❌ | ✅ `userid` + `token` | 高音质与部分曲库受限 |
| `qishui` | ❌ | ✅ | ❌ | **搜索直接不可用**（空响应），只能 3 档 AAC |

`list_accounts` 返回每行 `loggedIn` `account` `supportsQrLogin` `supportsCookieLogin` `note`，`note` 里常有失败原因（网络异常只影响那一行）。

汽水是特例：Cookie 要从**本机客户端**读，还需要签名才能搜索——完整流程在 `qishui.md`，别用通用登录流程套。

## 扫码登录（QQ / 网易云 / 酷狗）

```
start_qr_login {platform}
  → 返回 PNG 图片内容 + {sessionId, imagePath, imageUrl, expiresInSeconds}
把图片发给用户（直接展示 PNG，不要只给 URL —— imageUrl 需要认证头才能取）
  → 每 2 秒 check_qr_login {platform, sessionId}
      status: waiting（待扫）→ scanned（已扫待确认）→ confirmed | expired | canceled | error
      done: true 时停止
```

要点：

- `confirmed` 时凭据**已自动写盘**，`saved: true`，不要再调 `save_cookie` 或 `login_with_credentials`。
- 有效期：QQ 180s、网易云 180s、酷狗 300s；`expired` 要重新 `start_qr_login`。
- 轮询间隔 2 秒，`done` 为 true 立刻停。别 500ms 一次把接口刷爆。
- 用户说"扫了没反应"→ 先确认扫的是**当前这次**的码（旧码会覆盖 `qr-image/{platform}.png`）。

## Cookie 登录（网易云 / 汽水）

```jsonc
save_cookie {"platform":"netease","cookie":"MUSIC_U=…; __csrf=…"}
```

- 网易云：写入后**立即调校验接口**，返回 `loggedIn` 与 `account`，失败会直接报错。
- 汽水：只做格式校验后落盘，`loggedIn` 仅代表"已配置"，不代表能用（搜索还要签名）。
- Cookie 取值：浏览器 F12 → Network → 任一请求的 `Cookie` 头整行复制。汽水可从本机客户端读，见 `qishui.md`。
- **不要在对话里回显完整 Cookie**；`/api/qishui/cookie` 只返回 `{configured, length, keys}` 是有意的。

## 凭据登录（QQ / 酷狗）

```jsonc
// QQ
{"platform":"qqmusic","uin":"10001","authst":"<token>",
 "refreshToken":"…","refreshKey":"…","accessToken":"…","openid":"…"}   // 后四个可选

// 酷狗
{"platform":"kugou","userid":"12345","token":"<token>","mid":"…","dfid":"…"}  // mid/dfid 可选
```

返回只含账号标识，**不回显令牌**。QQ 的 `authst` 有效期有限，过期后会突然搜不到会员曲——此时重新扫码最省事。

## 登出

```jsonc
logout_account {"platform":"qqmusic|netease|kugou|qishui"}
```

汽水的登出等于清除 Cookie（`DELETE /api/qishui/cookie` 等价）。登出后 `list_accounts` 的 `loggedIn` 应为 false，复核一次。

## 判断"是登录问题吗"

```
搜索/下载异常
├─ list_accounts 显示未登录 → 是。先登录
├─ 已登录但仍只有低音质
│   ├─ 该曲是会员曲 → 非会员无解，说明清楚
│   └─ 不是会员曲 → 可能是凭据过期（QQ 常见）→ 重新扫码
├─ 汽水已配置 Cookie 但搜索空
│   └─ 不是登录问题，是签名问题 → qishui.md / GET /api/qishui/signer
└─ 网易云只有 128kmp3
    └─ 未登录或 Cookie 过期 → save_cookie 重写
```

## 安全边界

- 不把 Cookie/token 写进任务、日志、文件名、报告；
- 不在回复里贴出完整凭据（最多给前 6 位 + 长度）；
- 扫码图只发给当前用户，不发群；
- `~/.workbuddy/mcp.json` 里的 `?key=` 等同管理员权限，不要外传。
