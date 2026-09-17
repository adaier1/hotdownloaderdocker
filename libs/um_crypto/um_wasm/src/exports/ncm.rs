use umc_ncm::header::NCMFile;
use umc_ncm::NetEaseCryptoError;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// NCMFile
#[wasm_bindgen(js_name=NCMFile)]
pub struct JsNCMFile {
    ncm: Option<NCMFile>,
}

#[wasm_bindgen(js_class=NCMFile)]
impl JsNCMFile {
    /// Create a NCMFile instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<JsNCMFile, JsError> {
        Ok(JsNCMFile { ncm: None })
    }

    /// Open NCM file.
    /// If everything is ok, return `0`.
    /// If it needs more header bytes, return positive integer.
    /// If it was not a valid NCM file, return -1.
    ///
    /// # Arguments
    ///
    /// * `header`: Header bytes of NCM file.
    ///
    /// returns: Result<i32, JsError>
    ///
    /// If it needs more bytes, the new header size will be returned.
    /// If the header was large enough, it will return 0.
    pub fn open(&mut self, header: &[u8]) -> Result<i32, JsError> {
        match NCMFile::new(header) {
            Ok(ncm) => {
                self.ncm = Some(ncm);
                Ok(0)
            }
            Err(NetEaseCryptoError::HeaderTooSmall(n)) => Ok(n as i32),
            Err(NetEaseCryptoError::NotNCMFile) => Ok(-1),
            Err(err) => Err(JsError::new(err.to_string().as_str())),
        }
    }

    /// Decrypt buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer`: Buffer to decrypt.
    /// * `offset`: Offset (start from 0, of encrypted binary data)
    ///
    /// returns: Result<(), JsError>
    pub fn decrypt(&self, buffer: &mut [u8], offset: usize) -> Result<(), JsError> {
        if let Some(ncm) = &self.ncm {
            ncm.decrypt(buffer, offset);
            Ok(())
        } else {
            Err(JsError::new("NCMFile not initialized."))
        }
    }

    /// Get audio data offset.
    #[wasm_bindgen(getter, js_name=audioOffset)]
    pub fn get_audio_offset(&self) -> Result<usize, JsError> {
        if let Some(ncm) = &self.ncm {
            Ok(ncm.audio_data_offset)
        } else {
            Err(JsError::new("NCMFile not initialized."))
        }
    }
}
