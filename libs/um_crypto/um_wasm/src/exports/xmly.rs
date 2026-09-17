use std::convert::TryInto;
use umc_xmly::android::{decrypt_android, FileType};
use umc_xmly::pc::Header as PCHeader;
use umc_xmly::XmlyError;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// Decrypt X2M Header
#[wasm_bindgen(js_name=decryptX2MHeader)]
pub fn js_decrypt_x2m_header(buffer: &mut [u8]) -> Result<(), JsError> {
    decrypt_android(FileType::X2M, buffer.try_into().map_err(JsError::from)?);
    Ok(())
}

/// Decrypt X3M Header
#[wasm_bindgen(js_name=decryptX3MHeader)]
pub fn js_decrypt_x3m_header(buffer: &mut [u8]) -> Result<(), JsError> {
    decrypt_android(FileType::X3M, buffer.try_into().map_err(JsError::from)?);
    Ok(())
}

/// Ximalaya PC Decipher.
#[wasm_bindgen(js_name=XmlyPC)]
pub struct JsXmlyPC(PCHeader);

#[wasm_bindgen(js_class=XmlyPC)]
impl JsXmlyPC {
    /// Get required bytes for the header, or throw error if not valid XM file.
    #[wasm_bindgen(js_name = "getHeaderSize")]
    pub fn get_header_size(buffer: &[u8]) -> Result<usize, JsError> {
        let required_len = match PCHeader::from_buffer(buffer) {
            Ok(hdr) => hdr.data_start_offset,
            Err(XmlyError::MetadataTooSmall(n)) => n,
            Err(err) => Err(JsError::from(err))?,
        };

        Ok(required_len)
    }

    /// Create a new XmlyPC decipher
    #[wasm_bindgen(constructor)]
    pub fn new(header: &[u8]) -> Result<JsXmlyPC, JsError> {
        let hdr = PCHeader::from_buffer(header)?;
        Ok(Self(hdr))
    }

    /// Get the first few bytes of the header.
    #[wasm_bindgen(getter, js_name=audioHeader)]
    pub fn get_audio_header(&self) -> Vec<u8> {
        self.0.copy_m4a_header()
    }

    /// Get the offset where the encrypted header is
    #[wasm_bindgen(getter, js_name=encryptedHeaderOffset)]
    pub fn get_encrypted_header_offset(&self) -> usize {
        self.0.data_start_offset
    }

    /// Get the size of encrypted header
    #[wasm_bindgen(getter, js_name=encryptedHeaderSize)]
    pub fn get_encrypted_header_len(&self) -> usize {
        self.0.encrypted_header_size
    }

    /// Decrypt encrypted header
    pub fn decrypt(&self, buffer: &mut [u8]) -> Result<usize, JsError> {
        let size = self.0.decrypt(buffer)?.len();
        Ok(size)
    }
}
