use um_audio::{AudioError, AudioType};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

/// Detected audio result
#[wasm_bindgen(js_name=AudioTypeResult)]
pub struct AudioTypeResult {
    /// When this field is not zero, it means we need to feed this amount of bytes to the detector.
    #[wasm_bindgen(js_name=needMore)]
    pub need_more: usize,

    /// Audio extension, without "."
    /// When is unknown, it will return "bin".
    #[wasm_bindgen(getter_with_clone,js_name=audioType)]
    pub audio_type: String,
}

/// Detect audio type for given file header.
/// Recommended a buffer of 1024 bytes.
#[wasm_bindgen(js_name=detectAudioType)]
pub fn detect_audio_type(buffer: &[u8]) -> Result<AudioTypeResult, JsError> {
    let (need_more, audio_type) = match um_audio::detect_audio_type(buffer) {
        Ok(t) => (0, t),
        Err(AudioError::NeedMoreHeader(n)) => (n, AudioType::Unknown),
        // Err(err) => Err(JsError::new(err.into()))?,
    };
    Ok(AudioTypeResult {
        need_more,
        audio_type: audio_type.to_string(),
    })
}
