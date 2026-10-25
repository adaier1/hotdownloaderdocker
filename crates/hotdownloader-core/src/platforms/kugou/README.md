# 酷狗平台

共享 Rust 实现，桌面端和独立 Web 服务均直接访问酷狗接口。

- 支持歌曲、歌单、专辑、歌手搜索，热搜和搜索建议。
- 支持完整歌单/专辑歌曲、分页歌手歌曲与专辑，详情中的歌手和专辑关联跳转。
- 歌单导入使用 `collection_…` 标识，或包含 `global_collection_id` / `gid` 的酷狗链接；也可从歌单搜索结果直接打开。
- 音质根据响应中的 hash 和文件大小生成：128k MP3、320k MP3、FLAC、Hi-Res。文件名中保存选定音质的 hash、专辑 ID 和 album_audio_id，供播放、重试、降级和恢复任务使用。
- LRC 歌词使用歌曲 hash 获取；歌词查询接口的 `songMid` 参数用于传递该 hash。
- 设置页提供酷狗 App 扫码、手动导入标准版 `userid` / `token`、登录状态和退出登录。手动导入仅校验字段格式，凭据有效性以平台请求结果为准；令牌过期后请重新登录。
- 设备 MID / DFID 与账号凭据保存在独立的 `kugou-credentials.json` 中（桌面端应用数据目录、Web 服务的数据目录），不进入普通设置、浏览器持久化或下载任务记录。每次取得播放/下载地址时读取当前凭据，并将同一 MID 和 userid 用于签名；退出清除账号凭据并使待确认会话失效。
- 平台权限、地区或验证要求可能阻止播放/下载；此时返回平台错误，不将试听片段保存为完整音源。

协议参考 [MakcRe/KuGouMusicApi](https://github.com/MakcRe/KuGouMusicApi/tree/514f870db7e60ca37bc5535be5da36d32b571c6f) 的 `util/request.js`、`util/helper.js` 和 `module/` 对应接口。保留上游 [MIT 许可证](LICENSE)。

离线验证使用项目现有 `npm run test:core`。真实接口验证手动运行，默认测试不访问酷狗网络：

```powershell
cargo test --locked --manifest-path crates/hotdownloader-core/Cargo.toml live_queries -- --ignored --nocapture
cargo test --locked --manifest-path crates/hotdownloader-core/Cargo.toml live_audio_link -- --ignored --nocapture
cargo test --locked --manifest-path crates/hotdownloader-core/Cargo.toml live_qr_login -- --ignored --nocapture
```
