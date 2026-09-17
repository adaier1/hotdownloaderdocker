use umc_joox::decrypt::JooxDecipher;
use umc_joox::header::Header;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// Joox file.
#[wasm_bindgen(js_name=JooxFile)]
pub struct JsJooxFile(Header);

#[wasm_bindgen(js_class = JooxFile)]
impl JsJooxFile {
    /// Initialize header. Header should be 0x0c bytes.
    pub fn parse(header: &[u8], uuid: String) -> Result<JsJooxFile, JsError> {
        Ok(JsJooxFile(
            Header::from_buffer(header, uuid.as_bytes()).map_err(JsError::from)?,
        ))
    }

    /// Get the buffer size to allocate for decrypt method.
    #[wasm_bindgen(getter, js_name = "bufferLength")]
    pub fn get_buffer_size(&self) -> usize {
        self.0.get_audio_block_size()
    }

    /// Decrypt a given block of buffer (see {@link bufferLength})
    /// Return the length of decrypted & unpadded data from the input buffer.
    #[wasm_bindgen(js_name = "decrypt")]
    pub fn decrypt(&self, buffer: &mut [u8]) -> Result<usize, JsError> {
        let decrypted = self.0.decrypt_audio_block(buffer).map_err(JsError::from)?;
        Ok(decrypted.len())
    }
}
