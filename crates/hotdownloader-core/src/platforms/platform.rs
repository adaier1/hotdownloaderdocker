use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 支持的音乐平台标识；具体 API 实现由运行时的 worker 提供。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    /// QQ 音乐
    #[serde(rename = "qqmusic")]
    QqMusic,
    /// 酷我音乐
    #[serde(rename = "kuwo")]
    Kuwo,
    /// 酷狗音乐
    #[serde(rename = "kugou")]
    Kugou,
    /// 网易云音乐
    #[serde(rename = "netease")]
    Netease,
    /// 哔哩哔哩（仅支持视频链接解析，不参与搜索）
    #[serde(rename = "bilibili")]
    Bilibili,
}

impl Platform {
    /// 与前端、持久化记录一致的平台标识。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QqMusic => "qqmusic",
            Self::Kuwo => "kuwo",
            Self::Kugou => "kugou",
            Self::Netease => "netease",
            Self::Bilibili => "bilibili",
        }
    }
}

impl FromStr for Platform {
    type Err = String;

    /// 从前端或持久化记录中的平台字符串解析。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "qqmusic" => Ok(Self::QqMusic),
            "kuwo" => Ok(Self::Kuwo),
            "kugou" => Ok(Self::Kugou),
            "netease" => Ok(Self::Netease),
            "bilibili" => Ok(Self::Bilibili),
            _ => Err(format!("不支持的平台: {value}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn platform_strings_keep_existing_error_contract() {
        assert_eq!("qqmusic".parse::<Platform>().unwrap(), Platform::QqMusic);
        assert_eq!("kuwo".parse::<Platform>().unwrap(), Platform::Kuwo);
        assert_eq!("kugou".parse::<Platform>().unwrap(), Platform::Kugou);
        assert_eq!("netease".parse::<Platform>().unwrap(), Platform::Netease);
        assert_eq!(
            "other".parse::<Platform>().unwrap_err(),
            "不支持的平台: other"
        );
    }
}
