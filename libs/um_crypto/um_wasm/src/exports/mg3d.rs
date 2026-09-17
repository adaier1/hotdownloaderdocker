use umc_mg3d::{guess_key, Decipher};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// Migu3D MG3D file decipher.
#[wasm_bindgen(js_name=Migu3D)]
pub struct JsMigu3D(Decipher);

#[wasm_bindgen(js_class=Migu3D)]
impl JsMigu3D {
    /// Create a new decipher and guess its key from first 0x100 bytes.
    #[wasm_bindgen(js_name=fromHeader)]
    pub fn from_header(header: &[u8]) -> Result<JsMigu3D, JsError> {
        let key = guess_key(header).ok_or_else(|| JsError::new("failed to guess key"))?;
        let decipher = Decipher::new_from_final_key(&key)?;
        Ok(JsMigu3D(decipher))
    }

    /// Create a new decipher from file_key
    #[wasm_bindgen(js_name=fromFileKey)]
    pub fn from_file_key(file_key: &str) -> Result<JsMigu3D, JsError> {
        let decipher = Decipher::new_from_file_key(file_key)?;
        Ok(JsMigu3D(decipher))
    }

    /// Decrypt encrypted buffer part.
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        self.0.decrypt(buffer, offset)
    }
}
