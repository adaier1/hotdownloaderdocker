use rand::Rng;

/// GUID 可用字符集，用于生成随机 guid
/// https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/utils.rs#L16
const GUID_CHARSET: &[u8] = b"ABCDEF1234567890";

/// 生成一个 32 位随机 GUID，由大写字母和数字组成。
/// 为 API 请求的 comm 字段提供随机 guid，替换原有硬编码值，避免固定 guid 可能导致的限制或风控。
/// 使用 rand 0.9 的 `rng()` 与 `random_range` 生成 32 个随机字符。
/// https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/utils.rs#L28
pub(crate) fn get_guid() -> String {
    let mut rng = rand::rng();
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..GUID_CHARSET.len());
            GUID_CHARSET[idx] as char
        })
        .collect()
}
