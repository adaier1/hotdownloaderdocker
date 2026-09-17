use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// QRC Decrypt ("*.qrc" cache file)
#[wasm_bindgen(js_name=decryptQRCFile)]
pub fn js_decrypt_qrc(buffer: &mut [u8]) -> Result<Vec<u8>, JsError> {
    umc_qrc::decrypt_qrc_file(buffer).map_err(JsError::from)
}

/// QRC Decrypt (network response)
#[wasm_bindgen(js_name=decryptQRCNetwork)]
pub fn js_decrypt_qrc_network(buffer: &str) -> Result<Vec<u8>, JsError> {
    umc_qrc::decrypt_qrc_network(buffer).map_err(JsError::from)
}
