use wasm_bindgen::JsError;

pub fn map_js_error<T>(error: T) -> JsError
where
    T: std::fmt::Debug + std::fmt::Display,
{
    JsError::new(error.to_string().as_str())
}
