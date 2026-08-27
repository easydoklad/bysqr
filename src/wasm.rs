//! Stable, string-and-byte-oriented WebAssembly adapter.
//!
//! This module deliberately keeps Rust domain models out of the WASM ABI.
//! Canonical documents cross the boundary as JSON/XML strings, encoded
//! documents as Base32hex strings, and raster images as byte arrays.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use crate::{
    diagnostic::AdvisoryDiagnostic,
    document::{self, Document},
    error::Error,
    invoice::{self, InvoiceModelError},
    invoice_items::{self, InvoiceLine},
    pay, qr,
};

const WASM_API_VERSION: u32 = 1;

type AdapterResult<T> = std::result::Result<T, Box<WasmError>>;

/// Stable structured error thrown by all WASM exports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct WasmError {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decoded: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
}

impl WasmError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
            position: None,
            actual: None,
            maximum: None,
            expected: None,
            format: None,
            decoded: None,
            count: None,
        }
    }

    fn invalid(field: impl Into<String>, message: impl Into<String>) -> Self {
        let field = field.into();
        let detail = message.into();
        let mut error = Self::new("INVALID_INPUT", format!("invalid {field}: {detail}"));
        error.field = Some(field);
        error
    }

    fn deserialize(format: impl Into<String>, message: impl Into<String>) -> Self {
        let format = format.into();
        let detail = message.into();
        let mut error = Self::new(
            "DESERIALIZE",
            format!("unable to deserialize {format}: {detail}"),
        );
        error.format = Some(format);
        error
    }

    fn serialize(format: impl Into<String>, message: impl Into<String>) -> Self {
        let format = format.into();
        let detail = message.into();
        let mut error = Self::new(
            "SERIALIZE",
            format!("unable to serialize {format}: {detail}"),
        );
        error.format = Some(format);
        error
    }
}

impl From<Error> for WasmError {
    fn from(error: Error) -> Self {
        let message = error.to_string();
        match error {
            Error::InvalidInput { field, .. } => {
                let mut error = Self::new("INVALID_INPUT", &message);
                error.field = Some(field.to_owned());
                error
            }
            Error::Unsupported(_) => Self::new("UNSUPPORTED", &message),
            Error::SequenceTooLong { actual, maximum } => {
                let mut error = Self::new("SEQUENCE_TOO_LONG", &message);
                error.actual = Some(actual as u64);
                error.maximum = Some(maximum as u64);
                error
            }
            Error::PayloadTooLong(actual) => {
                let mut error = Self::new("PAYLOAD_TOO_LONG", &message);
                error.actual = Some(actual as u64);
                error.maximum = Some(u16::MAX as u64);
                error
            }
            Error::InvalidPayload(_) => Self::new("INVALID_PAYLOAD", &message),
            Error::InvalidSequence {
                position, field, ..
            } => {
                let mut error = Self::new("INVALID_SEQUENCE", &message);
                error.position = Some(position);
                error.field = Some(field.to_owned());
                error
            }
            Error::Compression(_) => Self::new("COMPRESSION", &message),
            Error::ChecksumMismatch { expected, actual } => {
                let mut error = Self::new("CHECKSUM_MISMATCH", &message);
                error.expected = Some(expected as u64);
                error.actual = Some(actual as u64);
                error
            }
            Error::Deserialize { format, .. } => {
                let mut error = Self::new("DESERIALIZE", &message);
                error.format = Some(format.to_owned());
                error
            }
            Error::QrEncode(_) => Self::new("QR_ENCODE", &message),
            Error::SvgRender(_) => Self::new("SVG_RENDER", &message),
            Error::ImageEncode { format, .. } => {
                let mut error = Self::new("IMAGE_ENCODE", &message);
                error.format = Some(format.to_owned());
                error
            }
            Error::ImageDecode(_) => Self::new("IMAGE_DECODE", &message),
            Error::QrNotFound => Self::new("QR_NOT_FOUND", &message),
            Error::QrDecode(_) => Self::new("QR_DECODE", &message),
            Error::PayQrNotFound { decoded } => {
                let mut error = Self::new("PAY_QR_NOT_FOUND", &message);
                error.decoded = Some(decoded);
                error
            }
            Error::InvoiceItemsQrNotFound { decoded } => {
                let mut error = Self::new("INVOICE_ITEMS_QR_NOT_FOUND", &message);
                error.decoded = Some(decoded);
                error
            }
            Error::MultiplePayQrCodes(count) => {
                let mut error = Self::new("MULTIPLE_PAY_QR_CODES", &message);
                error.count = Some(count);
                error
            }
            Error::BySquareQrNotFound { decoded } => {
                let mut error = Self::new("BY_SQUARE_QR_NOT_FOUND", &message);
                error.decoded = Some(decoded);
                error
            }
            Error::MultipleBySquareQrCodes(count) => {
                let mut error = Self::new("MULTIPLE_BY_SQUARE_QR_CODES", &message);
                error.count = Some(count);
                error
            }
            Error::Utf8(_) => Self::new("UTF8", &message),
        }
    }
}

impl From<InvoiceModelError> for WasmError {
    fn from(error: InvoiceModelError) -> Self {
        Self::invalid(error.field(), error.message())
    }
}

impl From<Box<WasmError>> for WasmError {
    fn from(error: Box<WasmError>) -> Self {
        *error
    }
}

fn boxed_error(error: impl Into<WasmError>) -> Box<WasmError> {
    Box::new(error.into())
}

fn to_js_error(error: impl Into<WasmError>) -> JsValue {
    let error = error.into();
    serde_wasm_bindgen::to_value(&error)
        .expect("serializing the fixed-shape WASM error object cannot fail")
}

fn to_json<T: Serialize>(value: &T, format: &'static str) -> AdapterResult<String> {
    serde_json::to_string(value)
        .map_err(|error| boxed_error(WasmError::serialize(format, error.to_string())))
}

/// Version of the low-level WASM ABI exposed by this adapter.
#[wasm_bindgen]
pub fn wasm_api_version() -> u32 {
    WASM_API_VERSION
}

/// Rust crate version used to build the WASM package.
#[wasm_bindgen]
pub fn bysqr_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Classify and encode canonical PAY, INVOICE, or INVOICE ITEMS JSON/XML.
#[wasm_bindgen]
pub fn encode_document(source: &str) -> Result<String, JsValue> {
    encode_document_impl(source).map_err(to_js_error)
}

fn encode_document_impl(source: &str) -> AdapterResult<String> {
    document::try_deserialize(source)
        .map_err(boxed_error)?
        .encode()
        .map_err(boxed_error)
}

/// Parse and encode only canonical PAY JSON/XML.
#[wasm_bindgen]
pub fn encode_pay(source: &str) -> Result<String, JsValue> {
    encode_pay_impl(source).map_err(to_js_error)
}

fn encode_pay_impl(source: &str) -> AdapterResult<String> {
    let pay = pay::try_deserialize_pay(source).map_err(boxed_error)?;
    pay::encode(&pay).map_err(boxed_error)
}

/// Parse and encode only canonical INVOICE JSON/XML.
#[wasm_bindgen]
pub fn encode_invoice(source: &str) -> Result<String, JsValue> {
    encode_invoice_impl(source).map_err(to_js_error)
}

fn encode_invoice_impl(source: &str) -> AdapterResult<String> {
    let invoice = invoice::try_deserialize_invoice(source).map_err(boxed_error)?;
    invoice::encode(&invoice).map_err(boxed_error)
}

/// Parse and encode only canonical INVOICE ITEMS JSON/XML.
#[wasm_bindgen]
pub fn encode_invoice_items(source: &str) -> Result<String, JsValue> {
    encode_invoice_items_impl(source).map_err(to_js_error)
}

fn encode_invoice_items_impl(source: &str) -> AdapterResult<String> {
    let items = invoice_items::try_deserialize_invoice_items(source).map_err(boxed_error)?;
    invoice_items::encode(&items).map_err(boxed_error)
}

#[derive(Serialize)]
#[serde(tag = "type", content = "value")]
enum TaggedDocument<'a> {
    #[serde(rename = "pay")]
    Pay(&'a pay::Pay),
    #[serde(rename = "invoice")]
    Invoice(&'a invoice::Invoice),
    #[serde(rename = "invoiceItems")]
    InvoiceItems(&'a invoice_items::InvoiceItems),
}

/// Decode a payload into a tagged canonical JSON document.
#[wasm_bindgen]
pub fn decode_document(payload: &str) -> Result<String, JsValue> {
    decode_document_impl(payload).map_err(to_js_error)
}

fn decode_document_impl(payload: &str) -> AdapterResult<String> {
    let document = document::decode(payload.trim()).map_err(boxed_error)?;
    let tagged = match &document {
        Document::Pay(pay) => TaggedDocument::Pay(pay),
        Document::Invoice(invoice) => TaggedDocument::Invoice(invoice),
        Document::InvoiceItems(items) => TaggedDocument::InvoiceItems(items),
    };
    to_json(&tagged, "tagged document JSON")
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRenderOptions {
    layout: Option<String>,
    position: Option<String>,
    color: Option<String>,
}

fn parse_render_options(options_json: &str) -> AdapterResult<qr::LogoTheme> {
    let options: RawRenderOptions = serde_json::from_str(options_json).map_err(|error| {
        boxed_error(WasmError::deserialize(
            "render options JSON",
            error.to_string(),
        ))
    })?;
    let default = qr::LogoTheme::default();
    let layout = match options.layout.as_deref() {
        None | Some("print") => qr::LogoLayout::Print,
        Some("electronic") => qr::LogoLayout::Electronic,
        Some(_) => {
            return Err(boxed_error(WasmError::invalid(
                "layout",
                "must be print or electronic",
            )))
        }
    };
    let position = match options.position.as_deref() {
        None | Some("bottom") => qr::LogoPosition::Bottom,
        Some("top") => qr::LogoPosition::Top,
        Some("left") => qr::LogoPosition::Left,
        Some("right") => qr::LogoPosition::Right,
        Some(_) => {
            return Err(boxed_error(WasmError::invalid(
                "position",
                "must be bottom, top, left, or right",
            )))
        }
    };
    let color = match options.color.as_deref() {
        None => default.color,
        Some("light") => qr::LogoColor::Light,
        Some("dark") => qr::LogoColor::Dark,
        Some("gray") => qr::LogoColor::Gray,
        Some("black") => qr::LogoColor::Black,
        Some(_) => {
            return Err(boxed_error(WasmError::invalid(
                "color",
                "must be light, dark, gray, or black",
            )))
        }
    };
    Ok(qr::LogoTheme::new(layout, position, color))
}

fn render_payload_svg(payload: &str, options_json: &str) -> AdapterResult<Vec<u8>> {
    let payload = payload.trim();
    let document = document::decode(payload).map_err(boxed_error)?;
    let theme = parse_render_options(options_json)?;
    match document {
        Document::Pay(_) => qr::create_pay_svg_with_theme(payload, theme),
        Document::Invoice(_) => qr::create_invoice_svg_with_theme(payload, theme),
        Document::InvoiceItems(_) if theme == qr::LogoTheme::default() => {
            qr::create_invoice_items_svg(payload)
        }
        Document::InvoiceItems(_) => {
            return Err(boxed_error(WasmError::invalid(
                "options",
                "INVOICE ITEMS supports only the default theme",
            )))
        }
    }
    .map_err(boxed_error)
}

/// Validate and render an encoded payload as SVG.
#[wasm_bindgen]
pub fn render_svg(payload: &str, options_json: &str) -> Result<String, JsValue> {
    render_svg_impl(payload, options_json).map_err(to_js_error)
}

fn render_svg_impl(payload: &str, options_json: &str) -> AdapterResult<String> {
    String::from_utf8(render_payload_svg(payload, options_json)?)
        .map_err(Error::from)
        .map_err(boxed_error)
}

/// Validate and render an encoded payload as raw PNG bytes.
#[wasm_bindgen]
pub fn render_png(payload: &str, width: u32, options_json: &str) -> Result<Vec<u8>, JsValue> {
    render_png_impl(payload, width, options_json).map_err(to_js_error)
}

fn render_png_impl(payload: &str, width: u32, options_json: &str) -> AdapterResult<Vec<u8>> {
    let svg = render_payload_svg(payload, options_json)?;
    qr::render_png(&svg, width).map_err(boxed_error)
}

/// Validate and render an encoded payload as raw JPEG bytes.
#[wasm_bindgen]
pub fn render_jpeg(
    payload: &str,
    width: u32,
    quality: u8,
    options_json: &str,
) -> Result<Vec<u8>, JsValue> {
    render_jpeg_impl(payload, width, quality, options_json).map_err(to_js_error)
}

fn render_jpeg_impl(
    payload: &str,
    width: u32,
    quality: u8,
    options_json: &str,
) -> AdapterResult<Vec<u8>> {
    let svg = render_payload_svg(payload, options_json)?;
    qr::render_jpeg(&svg, width, quality).map_err(boxed_error)
}

/// Chunk canonical `InvoiceLine[]` JSON and encode each INVOICE ITEMS block.
#[wasm_bindgen]
pub fn encode_invoice_items_chunks(invoice_id: &str, lines_json: &str) -> Result<String, JsValue> {
    encode_invoice_items_chunks_impl(invoice_id, lines_json).map_err(to_js_error)
}

fn encode_invoice_items_chunks_impl(invoice_id: &str, lines_json: &str) -> AdapterResult<String> {
    let lines: Vec<InvoiceLine> = serde_json::from_str(lines_json).map_err(|error| {
        boxed_error(WasmError::deserialize(
            "InvoiceLine[] JSON",
            error.to_string(),
        ))
    })?;
    let payloads = invoice_items::encode_chunks(invoice_id, lines).map_err(boxed_error)?;
    to_json(&payloads, "payload array JSON")
}

/// Decode and reassemble a JSON array of INVOICE ITEMS payload strings.
#[wasm_bindgen]
pub fn decode_invoice_items_chunks(payloads_json: &str) -> Result<String, JsValue> {
    decode_invoice_items_chunks_impl(payloads_json).map_err(to_js_error)
}

fn decode_invoice_items_chunks_impl(payloads_json: &str) -> AdapterResult<String> {
    let payloads: Vec<String> = serde_json::from_str(payloads_json).map_err(|error| {
        boxed_error(WasmError::deserialize(
            "payload array JSON",
            error.to_string(),
        ))
    })?;
    let reassembled = invoice_items::decode_chunks(&payloads).map_err(boxed_error)?;
    to_json(&reassembled, "reassembled INVOICE ITEMS JSON")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmDiagnostic<'a> {
    field_path: &'a str,
    actual_character_count: usize,
    recommended_maximum: usize,
}

impl<'a> From<&'a AdvisoryDiagnostic> for WasmDiagnostic<'a> {
    fn from(diagnostic: &'a AdvisoryDiagnostic) -> Self {
        Self {
            field_path: &diagnostic.field_path,
            actual_character_count: diagnostic.actual_character_count,
            recommended_maximum: diagnostic.recommended_maximum,
        }
    }
}

/// Return non-rejecting advisory maximum-length diagnostics for canonical source.
#[wasm_bindgen]
pub fn document_diagnostics(source: &str) -> Result<String, JsValue> {
    document_diagnostics_impl(source).map_err(to_js_error)
}

fn document_diagnostics_impl(source: &str) -> AdapterResult<String> {
    let document = document::try_deserialize(source).map_err(boxed_error)?;
    let diagnostics = match &document {
        Document::Pay(pay) => pay.advisory_diagnostics(),
        Document::Invoice(invoice) => invoice.advisory_diagnostics(),
        Document::InvoiceItems(items) => items.advisory_diagnostics(),
    };
    let diagnostics = diagnostics
        .iter()
        .map(WasmDiagnostic::from)
        .collect::<Vec<_>>();
    to_json(&diagnostics, "document diagnostics JSON")
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    const PAY: &str = include_str!("../tests/fixtures/pay/json/direct-debit-sepa.json");
    const INVOICE: &str =
        include_str!("../tests/fixtures/invoice/valid-interoperability-offline-single-line.json");
    const ITEMS: &str = include_str!(
        "../tests/fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json"
    );

    #[test]
    fn metadata_is_stable() {
        assert_eq!(wasm_api_version(), 1);
        assert_eq!(bysqr_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn family_encoders_and_tagged_decoder_use_canonical_data() {
        let pay_payload = encode_pay_impl(PAY).unwrap();
        assert_eq!(pay_payload, encode_document_impl(PAY).unwrap());
        let pay: Value =
            serde_json::from_str(&decode_document_impl(&pay_payload).unwrap()).unwrap();
        assert_eq!(pay["type"], "pay");
        assert!(pay["value"]["Payments"]["Payment"].is_array());

        let invoice_payload = encode_invoice_impl(INVOICE).unwrap();
        let invoice: Value =
            serde_json::from_str(&decode_document_impl(&invoice_payload).unwrap()).unwrap();
        assert_eq!(invoice["type"], "invoice");
        assert!(invoice["value"].get("DocumentType").is_some());

        let items_payload = encode_invoice_items_impl(ITEMS).unwrap();
        let items: Value =
            serde_json::from_str(&decode_document_impl(&items_payload).unwrap()).unwrap();
        assert_eq!(items["type"], "invoiceItems");
        assert!(items["value"]["InvoiceLines"]["InvoiceLine"].is_array());

        let wrong_family = encode_pay_impl(INVOICE).unwrap_err();
        assert_eq!(wrong_family.code, "DESERIALIZE");
    }

    #[test]
    fn themed_payload_rendering_returns_svg_and_raw_rasters() {
        let payload = encode_pay_impl(PAY).unwrap();
        let svg = render_svg_impl(
            &payload,
            r#"{"layout":"electronic","position":"left","color":"light"}"#,
        )
        .unwrap();
        assert!(svg.contains("viewBox=\"0 0 600 512\""));
        assert!(svg.contains(qr::LogoColor::Light.pay_hex()));

        let png = render_png_impl(&payload, 256, "{}").unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

        let jpeg = render_jpeg_impl(&payload, 256, 85, "{}").unwrap();
        assert_eq!(&jpeg[..2], b"\xff\xd8");
        assert_eq!(&jpeg[jpeg.len() - 2..], b"\xff\xd9");

        let items = encode_invoice_items_impl(ITEMS).unwrap();
        let error = render_svg_impl(&items, r#"{"color":"black"}"#).unwrap_err();
        assert_eq!(error.code, "INVALID_INPUT");
        assert_eq!(error.field.as_deref(), Some("options"));
    }

    #[test]
    fn invoice_item_chunks_round_trip_canonical_lines() {
        let source: Value = serde_json::from_str(ITEMS).unwrap();
        let line = source["InvoiceLines"]["InvoiceLine"][0].clone();
        let lines = vec![line; 5];
        let lines_json = serde_json::to_string(&lines).unwrap();
        let encoded = encode_invoice_items_chunks_impl("INV-CHUNKS", &lines_json).unwrap();
        let payloads: Vec<String> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(payloads.len(), 2);

        let decoded: Value = serde_json::from_str(
            &decode_invoice_items_chunks_impl(&serde_json::to_string(&payloads).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded["InvoiceID"], "INV-CHUNKS");
        assert_eq!(decoded["InvoiceLines"]["InvoiceLine"], json!(lines));
    }

    #[test]
    fn diagnostics_use_stable_camel_case_fields() {
        let mut source: Value = serde_json::from_str(PAY).unwrap();
        source["Payments"]["Payment"][0]["PaymentNote"] = json!("x".repeat(141));
        let diagnostics: Value =
            serde_json::from_str(&document_diagnostics_impl(&source.to_string()).unwrap()).unwrap();
        let note = diagnostics
            .as_array()
            .unwrap()
            .iter()
            .find(|diagnostic| {
                diagnostic["fieldPath"]
                    .as_str()
                    .unwrap()
                    .ends_with("PaymentNote")
            })
            .unwrap();
        assert_eq!(note["actualCharacterCount"], 141);
        assert_eq!(note["recommendedMaximum"], 140);
    }

    #[test]
    fn core_and_model_errors_have_stable_codes_and_fields() {
        let invalid = WasmError::from(Error::InvalidSequence {
            position: 7,
            field: "Amount",
            message: "bad decimal".to_owned(),
        });
        assert_eq!(invalid.code, "INVALID_SEQUENCE");
        assert_eq!(invalid.field.as_deref(), Some("Amount"));
        assert_eq!(invalid.position, Some(7));

        let checksum = WasmError::from(Error::ChecksumMismatch {
            expected: 10,
            actual: 20,
        });
        let value = serde_json::to_value(checksum).unwrap();
        assert_eq!(value["code"], "CHECKSUM_MISMATCH");
        assert_eq!(value["expected"], 10);
        assert_eq!(value["actual"], 20);

        let model = encode_invoice_impl("{}").unwrap_err();
        assert_eq!(model.code, "INVALID_INPUT");
        assert_eq!(model.field.as_deref(), Some("Invoice JSON"));
    }
}
