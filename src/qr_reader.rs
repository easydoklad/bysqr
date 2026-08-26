//! Optional QR image reader for supported by-square document families.
//!
//! Enable the `qr-reader` Cargo feature to extract text payloads from raster
//! images. Consumers that already have a QR scanner can keep using
//! [`crate::decode`] directly without enabling this module. The PAY-specific
//! functions remain available for consumers that only accept payments.

use image::DynamicImage;
use rqrr::PreparedImage;

use crate::{
    document,
    error::{Error, Result},
    pay::{self, Pay},
    Document,
};

/// Extract every decodable QR text payload from a raster image.
pub fn extract_payloads(image: &DynamicImage) -> Result<Vec<String>> {
    let mut prepared = PreparedImage::prepare(image.to_luma8());
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        return Err(Error::QrNotFound);
    }

    let detected = grids.len();
    let mut payloads = Vec::new();
    let mut errors = Vec::new();
    for grid in grids {
        match grid.decode() {
            Ok((_, payload)) => payloads.push(payload),
            Err(error) => errors.push(error.to_string()),
        }
    }

    if payloads.is_empty() {
        return Err(Error::QrDecode(format!(
            "detected {detected} candidate grids; {}",
            errors.join("; ")
        )));
    }

    Ok(payloads)
}

/// Decode raster image bytes and extract every QR text payload they contain.
pub fn extract_payloads_from_bytes(bytes: &[u8]) -> Result<Vec<String>> {
    let image =
        image::load_from_memory(bytes).map_err(|error| Error::ImageDecode(error.to_string()))?;
    extract_payloads(&image)
}

/// Find and decode exactly one valid PAY by square QR code in a raster image.
///
/// Other QR codes in the same image are ignored. More than one valid PAY code
/// is treated as ambiguous instead of selecting one silently.
pub fn decode_pay(image: &DynamicImage) -> Result<Pay> {
    decode_pay_payloads(extract_payloads(image)?)
}

/// Decode raster image bytes and reconstruct exactly one PAY document.
pub fn decode_pay_from_bytes(bytes: &[u8]) -> Result<Pay> {
    let image =
        image::load_from_memory(bytes).map_err(|error| Error::ImageDecode(error.to_string()))?;
    decode_pay(&image)
}

/// Find and decode exactly one valid by-square document in a raster image.
///
/// Other QR codes in the same image are ignored. More than one valid
/// by-square code is treated as ambiguous instead of selecting one silently.
pub fn decode_document(image: &DynamicImage) -> Result<Document> {
    decode_document_payloads(extract_payloads(image)?)
}

/// Decode raster image bytes and reconstruct exactly one by-square document.
pub fn decode_document_from_bytes(bytes: &[u8]) -> Result<Document> {
    let image =
        image::load_from_memory(bytes).map_err(|error| Error::ImageDecode(error.to_string()))?;
    decode_document(&image)
}

fn decode_document_payloads(payloads: Vec<String>) -> Result<Document> {
    let decoded_count = payloads.len();
    let mut documents = payloads
        .iter()
        .filter_map(|payload| document::decode(payload.trim()).ok());
    let first = documents.next();
    let second = documents.next();

    match (first, second) {
        (Some(document), None) => Ok(document),
        (Some(_), Some(_)) => {
            let count = 2 + documents.count();
            Err(Error::MultipleBySquareQrCodes(count))
        }
        (None, _) => Err(Error::BySquareQrNotFound {
            decoded: decoded_count,
        }),
    }
}

fn decode_pay_payloads(payloads: Vec<String>) -> Result<Pay> {
    if payloads.len() == 1 {
        return pay::decode(payloads[0].trim());
    }

    let decoded_count = payloads.len();
    let mut pay_documents = payloads
        .iter()
        .filter_map(|payload| pay::decode(payload.trim()).ok());
    let first = pay_documents.next();
    let second = pay_documents.next();

    match (first, second) {
        (Some(pay), None) => Ok(pay),
        (Some(_), Some(_)) => {
            let count = 2 + pay_documents.count();
            Err(Error::MultiplePayQrCodes(count))
        }
        (None, _) => Err(Error::PayQrNotFound {
            decoded: decoded_count,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_document_payloads, decode_pay_payloads};
    use crate::{error::Error, pay};

    fn valid_payload() -> String {
        let pay = pay::try_deserialize_pay(include_str!(
            "../tests/fixtures/pay/json/bulk-payment-order.json"
        ))
        .unwrap();
        pay::encode(&pay).unwrap()
    }

    #[test]
    fn selects_the_only_pay_payload_from_multiple_codes() {
        assert!(decode_pay_payloads(vec!["HELLO".to_owned(), valid_payload()]).is_ok());
    }

    #[test]
    fn rejects_zero_or_multiple_pay_payloads() {
        assert!(matches!(
            decode_pay_payloads(vec!["HELLO".to_owned(), "WORLD".to_owned()]),
            Err(Error::PayQrNotFound { decoded: 2 })
        ));

        let payload = valid_payload();
        assert!(matches!(
            decode_pay_payloads(vec![payload.clone(), payload]),
            Err(Error::MultiplePayQrCodes(2))
        ));
    }

    #[test]
    fn selects_or_rejects_generic_by_square_payloads() {
        let payload = valid_payload();
        assert!(matches!(
            decode_document_payloads(vec!["HELLO".to_owned(), payload.clone()]),
            Ok(crate::Document::Pay(_))
        ));
        assert!(matches!(
            decode_document_payloads(vec!["HELLO".to_owned()]),
            Err(Error::BySquareQrNotFound { decoded: 1 })
        ));
        assert!(matches!(
            decode_document_payloads(vec![payload.clone(), payload]),
            Err(Error::MultipleBySquareQrCodes(2))
        ));
    }
}
