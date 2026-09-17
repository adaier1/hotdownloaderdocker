# umc_kuwo

酷我解密相关。

## 酷我

### PC 平台

不需要额外配置密钥。

### 安卓平台

需要利用 `root` 权限提取 mmkv 数据库。

## 波点音乐

波点音乐（酷我 Lite，安卓/iOS）。

- 安卓包名 `cn.wenyu.bodian`

### 安卓

数据库路径 `/data/data/cn.wenyu.bodian/databases/list_downloaded.db`

密钥存储在 `download` 表中的 `json` 列。部分数据节选：

```json5
{
    "audioPath": "/sdcard/Android/data/cn.wenyu.bodian/files/BodianMusic/music/歌名-咯咯咯.mflac",
    "downInfo": {
        // ekey: string | null
        "ekey": "des_encrypt(device_id || ekey)"
    }
}
```

其中，当 `downInfo.ekey` 为 `null` 时表示该 `ekey` 不参与解密。

`ekey` 可以使用 `umc_kuwo::des::decode_ekey(ekey, umc_kuwo::SECRET_KEY)` 解密。
