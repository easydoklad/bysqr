#[cfg(feature = "wasm")]
use crate::pay::Pay;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

pub mod codec;
pub mod document;
pub mod error;
pub mod pay;
pub mod qr;
#[cfg(feature = "qr-reader")]
pub mod qr_reader;

pub use document::{decode, Document};

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_svg(source: &str) -> Result<String, JsValue> {
    let pay: Pay = pay::try_deserialize_pay(source).map_err(js_error)?;
    let encoded = pay::encode(&pay).map_err(js_error)?;
    let svg = qr::create_pay_svg(&encoded, qr::Theme::default());
    String::from_utf8(svg).map_err(js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_png(source: &str, size: u32) -> Result<String, JsValue> {
    let pay: Pay = pay::try_deserialize_pay(source).map_err(js_error)?;
    let encoded = pay::encode(&pay).map_err(js_error)?;
    let svg = qr::create_pay_svg(&encoded, qr::Theme::default());
    Ok(qr::to_base64_png(&svg, size))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_jpeg(source: &str, size: u32, quality: u8) -> Result<String, JsValue> {
    let pay: Pay = pay::try_deserialize_pay(source).map_err(js_error)?;
    let encoded = pay::encode(&pay).map_err(js_error)?;
    let svg = qr::create_pay_svg(&encoded, qr::Theme::default());
    Ok(qr::to_base64_jpeg(&svg, size, quality))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn decode_to_json(payload: &str) -> Result<String, JsValue> {
    let pay = pay::decode(payload.trim()).map_err(js_error)?;
    serde_json::to_string_pretty(&pay).map_err(js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn decode_to_xml(payload: &str) -> Result<String, JsValue> {
    let pay = pay::decode(payload.trim()).map_err(js_error)?;
    quick_xml::se::to_string(&pay).map_err(js_error)
}

#[cfg(all(feature = "wasm", feature = "qr-reader"))]
#[wasm_bindgen]
pub fn decode_image_to_json(image: &[u8]) -> Result<String, JsValue> {
    let pay = qr_reader::decode_pay_from_bytes(image).map_err(js_error)?;
    serde_json::to_string_pretty(&pay).map_err(js_error)
}

#[cfg(all(feature = "wasm", feature = "qr-reader"))]
#[wasm_bindgen]
pub fn decode_image_to_xml(image: &[u8]) -> Result<String, JsValue> {
    let pay = qr_reader::decode_pay_from_bytes(image).map_err(js_error)?;
    quick_xml::se::to_string(&pay).map_err(js_error)
}

#[cfg(feature = "wasm")]
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
