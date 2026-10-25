//! Rust 直连网易云；接口及协议参考 Binaryify/NeteaseCloudMusicApi。
//! 查询、登录、歌词及音源由桌面端与 Web 服务端共用。

pub mod album;
pub mod artist;
mod client;
pub mod cover;
mod crypto;
pub mod link;
pub mod login;
pub mod lyrics;
mod parser;
pub mod playlist;
pub mod search;
mod song;
pub mod suggest;

#[cfg(test)]
mod live_tests;
