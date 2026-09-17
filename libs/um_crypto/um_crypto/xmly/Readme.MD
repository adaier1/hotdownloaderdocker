# 喜马拉雅文件格式

## 安卓端

前 `0x400` 字节混淆，打乱顺序 + xor 固定密钥。

## PC 端

Electron 调用 node 的 `crypto` 层加密，完事后扔原生层(旧版)/Wasm(新版)再处理一遍。

其它人的逆向过程：

- [喜马拉雅 xm 文件解密逆向分析 [Electron]][xm_zh] | [存档][xm_zh_archive]
- [Reversing WebAssembly with pure guesswork - Ximalaya xm encryption (英文, 内容更多)][xm_en] | [存档][xm_en_archive]

[xm_zh]: https://www.aynakeya.com/articles/ctf/xi-ma-la-ya-xm-wen-jian-jie-mi-ni-xiang-fen-xi/

[xm_zh_archive]: https://web.archive.org/web/2023/https://www.aynakeya.com/articles/ctf/xi-ma-la-ya-xm-wen-jian-jie-mi-ni-xiang-fen-xi/

[xm_en]: https://www.aynakeya.com/articles/ctf/reverse-with-puring-guessing-ximalaya/

[xm_en_archive]: https://web.archive.org/web/2023/https://www.aynakeya.com/articles/ctf/reverse-with-puring-guessing-ximalaya/
