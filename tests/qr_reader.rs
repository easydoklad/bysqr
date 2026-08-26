#![cfg(feature = "qr-reader")]

use bysqr::{
    error::Error,
    invoice,
    pay::{self, try_deserialize_pay},
    qr::{self, Theme},
    qr_reader, Document,
};

fn fixture_pay() -> bysqr::pay::Pay {
    try_deserialize_pay(include_str!("fixtures/pay/json/direct-debit-sepa.json")).unwrap()
}

fn fixture_qr() -> (bysqr::pay::Pay, String, Vec<u8>) {
    let pay = fixture_pay();
    let payload = pay::encode(&pay).unwrap();
    let svg = qr::create_pay_svg(&payload, Theme::default());
    let png = qr::render_png(&svg, 1_024);
    (pay, payload, png)
}

fn fixture_invoice() -> invoice::Invoice {
    invoice::try_deserialize_invoice(include_str!(
        "fixtures/invoice/schema/minimal-header-invoice.json"
    ))
    .unwrap()
}

#[test]
fn extracts_payload_and_decodes_generated_pay_png() {
    let (expected, payload, png) = fixture_qr();

    assert_eq!(
        qr_reader::extract_payloads_from_bytes(&png).unwrap(),
        vec![payload]
    );
    assert_eq!(qr_reader::decode_pay_from_bytes(&png).unwrap(), expected);
}

#[test]
fn decodes_generated_pay_jpeg() {
    let expected = fixture_pay();
    let payload = pay::encode(&expected).unwrap();
    let svg = qr::create_pay_svg(&payload, Theme::default());
    let jpeg = qr::render_jpeg(&svg, 1_024, 95);

    assert_eq!(qr_reader::decode_pay_from_bytes(&jpeg).unwrap(), expected);
}

#[test]
fn extracts_payload_and_decodes_generated_invoice_png() {
    let expected = fixture_invoice();
    let payload = invoice::encode(&expected).unwrap();
    let svg = qr::create_invoice_svg(&payload);
    let png = qr::render_png(&svg, 1_024);

    assert_eq!(
        qr_reader::extract_payloads_from_bytes(&png).unwrap(),
        vec![payload]
    );
    assert_eq!(
        qr_reader::decode_document_from_bytes(&png).unwrap(),
        Document::Invoice(Box::new(expected))
    );
}

#[test]
fn distinguishes_image_qr_and_pay_errors() {
    assert!(matches!(
        qr_reader::decode_pay_from_bytes(b"not an image"),
        Err(Error::ImageDecode(_))
    ));

    let blank = image::DynamicImage::new_luma8(256, 256);
    assert!(matches!(
        qr_reader::decode_pay(&blank),
        Err(Error::QrNotFound)
    ));

    let svg = qr::create_pay_svg("HELLO", Theme::default());
    let png = qr::render_png(&svg, 1_024);
    assert!(matches!(
        qr_reader::decode_pay_from_bytes(&png),
        Err(Error::InvalidPayload(_))
    ));
}
