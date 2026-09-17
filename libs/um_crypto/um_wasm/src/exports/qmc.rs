use crate::errors::map_js_error;
use umc_qmc::footer::{Data as FooterData, FooterParseError};
use umc_qmc::QMCv2Cipher;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// QMC1 (qmcflac) decipher, decrypt buffer at given offset.
#[wasm_bindgen(js_name=decryptQMC1)]
pub fn js_decrypt_qmc1(buffer: &mut [u8], offset: usize) {
    umc_qmc::v1::decrypt(buffer, offset)
}

/// QMC2 (mgg/mflac) cipher
#[wasm_bindgen(js_name=QMC2)]
pub struct JsQMC2(QMCv2Cipher);

#[wasm_bindgen(js_class=QMC2)]
impl JsQMC2 {
    /// Create a new QMC2 (mgg/mflac) cipher instance.
    #[wasm_bindgen(constructor)]
    pub fn new(ekey: &str) -> Result<JsQMC2, JsError> {
        let cipher = QMCv2Cipher::new_from_ekey(ekey).map_err(map_js_error)?;
        Ok(JsQMC2(cipher))
    }

    /// Decrypt buffer at given offset.
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        self.0.decrypt(buffer, offset)
    }
}

/// QMC Footer.
#[wasm_bindgen(js_name=QMCFooter)]
pub struct JsQMCFooter(umc_qmc::footer::Metadata);

#[wasm_bindgen(js_class=QMCFooter)]
impl JsQMCFooter {
    /// Parse QMC Footer from byte slice.
    ///   Recommended to slice the last 1024 bytes of the file.
    pub fn parse(footer: &[u8]) -> Result<Option<JsQMCFooter>, JsError> {
        match umc_qmc::footer::from_byte_slice(footer) {
            Ok(Some(metadata)) => Ok(Some(JsQMCFooter(metadata))),
            Ok(None) => Ok(None),
            Err(FooterParseError::PCv1EKeyTooLarge(_)) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// Get eKey (if embedded)
    #[wasm_bindgen(getter)]
    pub fn ekey(&self) -> Option<String> {
        self.0.ekey.clone()
    }

    /// Get size of footer
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize {
        self.0.size
    }

    /// Get media name (MusicEx)
    #[wasm_bindgen(getter, js_name=mediaName)]
    pub fn get_media_name(&self) -> Option<String> {
        match &self.0.data {
            FooterData::PCv2MusicEx(metadata) => Some(metadata.media_filename.clone()),
            _ => None,
        }
    }
}
