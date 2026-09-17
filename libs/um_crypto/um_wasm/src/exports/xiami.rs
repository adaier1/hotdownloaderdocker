use umc_xiami::XiamiFile;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// Xiami XM file decipher.
#[wasm_bindgen(js_name=Xiami)]
pub struct JsXiami(XiamiFile);

#[wasm_bindgen(js_class = Xiami)]
impl JsXiami {
    /// Parse the Xiami header (0x400 bytes)
    pub fn from_header(header: &[u8]) -> Result<JsXiami, JsError> {
        let hdr = XiamiFile::from_header(header)?;
        Ok(JsXiami(hdr))
    }

    /// Decrypt encrypted buffer part.
    pub fn decrypt(&self, buffer: &mut [u8]) {
        self.0.decrypt(buffer)
    }

    /// After header (0x10 bytes), the number of bytes should be copied without decryption.
    #[wasm_bindgen(getter, js_name=copyPlainLength)]
    pub fn get_copy_plain_length(&self) -> usize {
        self.0.copy_len
    }
}
