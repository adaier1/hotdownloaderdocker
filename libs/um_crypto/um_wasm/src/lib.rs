use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

mod errors;
mod utils;

pub mod exports;

/// Init panic hook
#[wasm_bindgen(js_name=initPanicHook)]
pub fn init_panic_hook() {
    set_panic_hook();
}
