//! Optional QR image reader for PAY by square.
//!
//! Enable the `qr-reader` Cargo feature to extract text payloads from raster
//! images. Consumers that already have a QR scanner can keep using
//! [`crate::decoder::decode`] directly without enabling this module.

use image::DynamicImage;
use rqrr::PreparedImage;

use crate::{
    decoder,
    error::{Error, Result},
    models::Pay,
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

fn decode_pay_payloads(payloads: Vec<String>) -> Result<Pay> {
    if payloads.len() == 1 {
        return decoder::decode(payloads[0].trim());
    }

    let decoded_count = payloads.len();
    let mut pay_documents = payloads
        .iter()
        .filter_map(|payload| decoder::decode(payload.trim()).ok());
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
    use super::decode_pay_payloads;
    use crate::{encoder, error::Error, models::try_deserialize_pay};

    fn valid_payload() -> String {
        let pay = try_deserialize_pay(include_str!(
            "../tests/fixtures/pay/json/bulk-payment-order.json"
        ))
        .unwrap();
        encoder::encode(&pay).unwrap()
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
}
