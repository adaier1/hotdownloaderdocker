# 网易云音乐

通过 Rust 直接调用网易云接口，不需要部署 Node.js API 服务。接口路径、参数和 WEAPI/EAPI 请求格式参考 [Binaryify/NeteaseCloudMusicApi](https://github.com/Binaryify/NeteaseCloudMusicApi/tree/185031ddcefad34e294df6933418e44cc70ec31f) 的 `module/`、`util/crypto.js` 和 `util/request.js`；网络、AES、RSA 和响应解析均由本项目实现。

## 已接入功能

| 功能 | 实现 |
| --- | --- |
| 搜索 | 歌曲、歌单、专辑、歌手，支持分页；热搜和分类搜索建议 |
| 关联浏览 | 歌手关联、专辑关联、专辑歌曲、歌手歌曲及专辑分页 |
| 歌单 | ID、网页/移动端完整链接、163cn.tv/163cn.com 分享短链接；按完整 trackIds 分批读取，保持原顺序 |
| 我的歌单 | 当前登录用户创建的歌单，包含自己的喜欢歌单；自动读取全部列表分页 |
| 账号 | 二维码生成与轮询、导入并验证 Cookie、刷新登录、退出登录 |
| 音频 | 游客匿名会话、MP3/FLAC 试听与下载、断点续传、现有音质降级和任务恢复机制 |
| 音质 | 按歌曲详情实际返回的大小展示 standard/exhigh/lossless/hires/jyeffect/sky/jymaster，映射到应用品质标签 |
| 歌词及标签 | LRC、YRC 转增强 LRC、纯音乐识别、歌词文件、封面及元数据；封面支持原图和 500 像素 |

前端平台标识为 `netease`。桌面端 IPC 和 Web 音乐路由均使用此核心；设置页提供网易云账号入口，歌单页提供“我的网易云音乐歌单”。歌单自动监控目前仍使用项目既有的 QQ 音乐专用实现。

音源权限由网易云和当前账号决定。返回试听片段、空音源、与请求不符的音质或格式时明确报错；不会将试听片段或较低音质保存成目标品质。用户可启用现有自动降级设置，或登录有歌曲权限的账号。

## 凭据

运行时数据目录中的 `netease-credentials.json` 独立保存 Cookie，与其他平台隔离；Cookie 不进入普通设置、任务、历史或登录 API 的响应。下载/播放每次解析链接都会读取最新凭据。游客 MUSIC_A 会话仅保存在进程内存。二维码会话绑定凭据文件和代次，退出、刷新二维码或新的登录操作会使旧会话失效。

## 基本验证

默认测试覆盖加密及响应解码、歌曲关联与品质解析、歌单链接校验、试听与音质校验、逐字歌词转换，以及退出后的凭据和会话失效：

```sh
cargo test --locked --manifest-path crates/hotdownloader-core/Cargo.toml netease
```

可手动运行两项联网冒烟测试，分别检查公开查询/完整歌单和游客音源/二维码；默认测试不会访问网易云。音源检查只请求音频响应头后丢弃响应，不下载整首歌曲。

```sh
cargo test --locked --manifest-path crates/hotdownloader-core/Cargo.toml live_public_queries -- --ignored --nocapture
cargo test --locked --manifest-path crates/hotdownloader-core/Cargo.toml live_source_and_qr -- --ignored --nocapture
```

账号确认扫码、Cookie 导入、会员音质和私有歌单需要实际用户账号验证；自动测试不会使用或读取开发者的真实凭据。上游接口变化、地区和账号权限可能影响联网测试结果。
