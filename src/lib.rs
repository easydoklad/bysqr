#[cfg(feature = "wasm")]
use crate::models::Pay;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

pub mod codec;
pub mod encoder;
pub mod error;
pub mod models;
pub mod qr;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_svg(source: &str) -> Result<String, JsValue> {
    let pay: Pay = models::try_deserialize_pay(source).map_err(js_error)?;
    let encoded = encoder::encode(&pay).map_err(js_error)?;
    let svg = qr::create_pay_svg(&encoded, qr::Theme::default());
    String::from_utf8(svg).map_err(js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_png(source: &str, size: u32) -> Result<String, JsValue> {
    let pay: Pay = models::try_deserialize_pay(source).map_err(js_error)?;
    let encoded = encoder::encode(&pay).map_err(js_error)?;
    let svg = qr::create_pay_svg(&encoded, qr::Theme::default());
    Ok(qr::to_base64_png(&svg, size))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_jpeg(source: &str, size: u32, quality: u8) -> Result<String, JsValue> {
    let pay: Pay = models::try_deserialize_pay(source).map_err(js_error)?;
    let encoded = encoder::encode(&pay).map_err(js_error)?;
    let svg = qr::create_pay_svg(&encoded, qr::Theme::default());
    Ok(qr::to_base64_jpeg(&svg, size, quality))
}

#[cfg(feature = "wasm")]
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
