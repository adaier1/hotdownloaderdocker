use std::convert::TryInto;
use umc_qtfm::{nonce, secret, Decipher};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// QingTingFM QTA file decipher.
#[wasm_bindgen(js_name=QingTingFM)]
pub struct JsQingTingFM(Decipher);

#[wasm_bindgen(js_class = QingTingFM)]
impl JsQingTingFM {
    #[wasm_bindgen(js_name=getDeviceKey)]
    pub fn get_device_key(
        product: &str,
        device: &str,
        manufacturer: &str,
        brand: &str,
        board: &str,
        model: &str,
    ) -> Vec<u8> {
        secret::make_device_secret(product, device, manufacturer, brand, board, model).to_vec()
    }

    #[wasm_bindgen(js_name=getFileIV)]
    pub fn get_file_iv(file_name: &str) -> Result<Vec<u8>, JsError> {
        let iv = nonce::make_decipher_iv(file_name)?;
        Ok(iv.to_vec())
    }

    #[wasm_bindgen(constructor)]
    pub fn new(device_key: &[u8], file_iv: &[u8]) -> Result<JsQingTingFM, JsError> {
        let decipher = Decipher::new(device_key.try_into()?, file_iv.try_into()?);
        Ok(JsQingTingFM(decipher))
    }

    /// Decrypt encrypted buffer part.
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) {
        self.0.decrypt(buffer, offset)
    }
}
