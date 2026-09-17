use umc_qmc::QMCv2Cipher;

/// 解密上下文，封装 umc_qmc 解密器
pub struct DecryptContext {
    cipher: Option<QMCv2Cipher>,
    pub enabled: bool,
}

/// 初始化解密上下文
/// - `key`：前端传递的 ekey 字符串
/// - `enabled`：用户设置中是否启用解密
pub fn init_decryption(key: &str, enabled: bool) -> DecryptContext {
    if !enabled || key.is_empty() {
        return DecryptContext {
            cipher: None,
            enabled: false,
        };
    }

    match QMCv2Cipher::new_from_ekey(key.as_bytes()) {
        Ok(cipher) => {
            log::info!("解密器初始化成功");
            DecryptContext {
                cipher: Some(cipher),
                enabled: true,
            }
        }
        Err(e) => {
            log::error!("解密器初始化失败: {}", e);
            DecryptContext {
                cipher: None,
                enabled: false,
            }
        }
    }
}

/// 流式解密数据块
/// - `ctx`：由 `init_decryption` 创建的上下文
/// - `data`：待解密的字节块，原地修改
/// - `_size`：数据长度（冗余参数，保留以兼容原接口）
/// - `offset`：该数据块在文件中的绝对偏移（字节）
pub fn decrypt_chunk(ctx: &DecryptContext, data: &mut [u8], _size: u64, offset: u64) {
    if !ctx.enabled {
        return;
    }
    if let Some(cipher) = &ctx.cipher {
        cipher.decrypt(data, offset as usize);
    }
}
