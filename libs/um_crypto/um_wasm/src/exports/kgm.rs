use umc_kgm::{decrypt_db, header::Header, Decipher};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// KuGou KGM file header.
#[wasm_bindgen(js_name=KuGouHeader)]
pub struct JsKuGouHdr(Header);

#[wasm_bindgen(js_class=KuGouHeader)]
impl JsKuGouHdr {
    /// Parse the KuGou header (0x400 bytes recommended).
    #[wasm_bindgen(constructor)]
    pub fn new(header: &[u8]) -> Result<Self, JsError> {
        let header = Header::from_buffer(header).map_err(JsError::from)?;
        Ok(Self(header))
    }

    /// Get the audio hash (kgm v5).
    #[wasm_bindgen(getter, js_name = "audioHash")]
    pub fn get_audio_hash(&self) -> String {
        self.0.audio_hash.clone()
    }

    /// Get version
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn get_crypto_version(&self) -> u32 {
        self.0.crypto_version
    }

    /// Get offset to encrypted data
    #[wasm_bindgen(getter, js_name = "offsetToData")]
    pub fn get_offset_to_data(&self) -> u32 {
        self.0.offset_to_data as u32
    }
}

/// KuGou KGM file decipher.
#[wasm_bindgen(js_name=KuGou)]
pub struct JsKuGou(Decipher);

#[wasm_bindgen(js_class=KuGou)]
impl JsKuGou {
    /// Parse the KuGou header (0x400 bytes recommended).
    pub fn from_header(header: &[u8]) -> Result<Self, JsError> {
        let header = JsKuGouHdr::new(header)?;
        Self::from_header_v5(&header, None)
    }

    /// Parse the KuGou header (0x400 bytes recommended).
    #[wasm_bindgen(js_name=fromHeaderV5)]
    pub fn from_header_v5(header: &JsKuGouHdr, ekey: Option<String>) -> Result<Self, JsError> {
        let decipher = Decipher::new_v5(&header.0, ekey).map_err(JsError::from)?;
        Ok(Self(decipher))
    }

    /// Decrypt a buffer.
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        self.0.decrypt(buffer, offset)
    }

    /// Decrypt Kugou PC client db.
    #[wasm_bindgen(js_name=decryptDatabase)]
    pub fn decrypt_db(database: &mut [u8]) -> Result<(), JsError> {
        decrypt_db(database).map_err(JsError::from)
    }
}
