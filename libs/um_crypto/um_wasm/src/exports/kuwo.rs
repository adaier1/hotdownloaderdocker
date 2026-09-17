use crate::errors::map_js_error;
use crate::exports::qmc::JsQMC2;
use umc_kuwo::kwm_v1::CipherV1;
use umc_kuwo::{Decipher, Header};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// Kuwo KWM file header.
#[wasm_bindgen(js_name=KuwoHeader)]
pub struct JsKuwoHeader(Header);

#[wasm_bindgen(js_class = KuwoHeader)]
impl JsKuwoHeader {
    /// Parse the KuWo header (0x400 bytes)
    pub fn parse(header: &[u8]) -> Result<JsKuwoHeader, JsError> {
        let hdr = Header::from_bytes(header).map_err(map_js_error)?;
        Ok(JsKuwoHeader(hdr))
    }

    /// Get quality id (used for Android Kuwo APP),
    ///   that can be then used to extract ekey from mmkv db.
    #[wasm_bindgen(getter, js_name=qualityId)]
    pub fn quality_id(&self) -> u32 {
        self.0.get_quality_id()
    }

    /// Get resource id
    #[wasm_bindgen(getter, js_name=resourceId)]
    pub fn resource_id(&self) -> u32 {
        self.0.resource_id
    }
}

/// Create a decipher instance for "BoDian Music".
#[wasm_bindgen(js_name=kuwoBodianCipherFactory)]
pub fn js_kuwo_bodian_cipher_factory(ekey: &str) -> Result<JsQMC2, JsError> {
    let ekey = umc_kuwo::des::decode_ekey(ekey, &umc_kuwo::SECRET_KEY).map_err(map_js_error)?;
    JsQMC2::new(ekey.as_str())
}

/// Kuwo KWM v1
#[wasm_bindgen(js_name=KWMDecipherV1)]
pub struct JsDecipherV1(CipherV1);

#[wasm_bindgen(js_class=KWMDecipherV1)]
impl JsDecipherV1 {
    /// Create a decipher instance for "Kuwo KWM v1".
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        self.0.decrypt(buffer, offset)
    }
}

/// Create a decipher instance for "Kuwo KWM v2".
#[wasm_bindgen(js_name=kuwoV2CipherFactory)]
pub fn js_kuwo_v2_cipher_factory(ekey: &str) -> Result<JsQMC2, JsError> {
    JsQMC2::new(ekey)
}

/// Common V1/V2 wrapper interface, derived from `KuwoHeader.makeCipher`
#[wasm_bindgen(js_name=KWMDecipher)]
pub struct JsDecipher(Decipher);

#[wasm_bindgen(js_class=KWMDecipher)]
impl JsDecipher {
    /// Create an instance of cipher (decipher) for decryption
    #[wasm_bindgen(constructor)]
    pub fn make_decipher(
        header: &JsKuwoHeader,
        ekey: Option<String>,
    ) -> Result<JsDecipher, JsError> {
        let cipher = Decipher::new(&header.0, ekey).map_err(map_js_error)?;
        Ok(JsDecipher(cipher))
    }

    /// Decrypt buffer at given offset.
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        self.0.decrypt(buffer, offset)
    }
}
