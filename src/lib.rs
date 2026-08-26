#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

pub mod codec;
pub mod document;
pub mod error;
pub mod invoice;
pub mod invoice_items;
pub mod pay;
pub mod qr;
#[cfg(feature = "qr-reader")]
pub mod qr_reader;

pub use document::{decode, try_deserialize, Document};

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_svg(source: &str) -> Result<String, JsValue> {
    let svg = encode_source_to_svg(source)?;
    String::from_utf8(svg).map_err(js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_png(source: &str, size: u32) -> Result<String, JsValue> {
    let svg = encode_source_to_svg(source)?;
    Ok(qr::to_base64_png(&svg, size))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn encode_to_jpeg(source: &str, size: u32, quality: u8) -> Result<String, JsValue> {
    let svg = encode_source_to_svg(source)?;
    Ok(qr::to_base64_jpeg(&svg, size, quality))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn decode_to_json(payload: &str) -> Result<String, JsValue> {
    decode(payload.trim())
        .map_err(js_error)?
        .to_json_pretty()
        .map_err(js_error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn decode_to_xml(payload: &str) -> Result<String, JsValue> {
    decode(payload.trim())
        .map_err(js_error)?
        .to_xml()
        .map_err(js_error)
}

#[cfg(all(feature = "wasm", feature = "qr-reader"))]
#[wasm_bindgen]
pub fn decode_image_to_json(image: &[u8]) -> Result<String, JsValue> {
    qr_reader::decode_document_from_bytes(image)
        .map_err(js_error)?
        .to_json_pretty()
        .map_err(js_error)
}

#[cfg(all(feature = "wasm", feature = "qr-reader"))]
#[wasm_bindgen]
pub fn decode_image_to_xml(image: &[u8]) -> Result<String, JsValue> {
    qr_reader::decode_document_from_bytes(image)
        .map_err(js_error)?
        .to_xml()
        .map_err(js_error)
}

#[cfg(feature = "wasm")]
fn encode_source_to_svg(source: &str) -> Result<Vec<u8>, JsValue> {
    let document = try_deserialize(source).map_err(js_error)?;
    let encoded = document.encode().map_err(js_error)?;
    Ok(match document {
        Document::Pay(_) => qr::create_pay_svg(&encoded, qr::Theme::default()),
        Document::Invoice(_) => qr::create_invoice_svg(&encoded),
        Document::InvoiceItems(_) => qr::create_invoice_items_svg(&encoded),
    })
}

#[cfg(feature = "wasm")]
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
