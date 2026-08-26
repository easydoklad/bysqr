#![cfg(feature = "qr-reader")]

use bysqr::{
    error::Error,
    invoice, invoice_items,
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

fn fixture_invoice_items() -> invoice_items::InvoiceItems {
    invoice_items::try_deserialize_invoice_items(include_str!(
        "fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json"
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
fn extracts_payload_and_decodes_generated_invoice_items_png() {
    let expected = fixture_invoice_items();
    let payload = invoice_items::encode(&expected).unwrap();
    let svg = qr::create_invoice_items_svg(&payload);
    let png = qr::render_png(&svg, 1_024);

    assert_eq!(
        qr_reader::extract_payloads_from_bytes(&png).unwrap(),
        vec![payload]
    );
    assert_eq!(
        qr_reader::decode_document_from_bytes(&png).unwrap(),
        Document::InvoiceItems(Box::new(expected))
    );
}

#[test]
fn scans_and_reassembles_two_invoice_items_codes_from_one_image() {
    let payloads = [
        include_str!(
            "fixtures/invoice-items/valid-interoperability-offline-multi-qr-9.payload.txt"
        )
        .trim(),
        include_str!(
            "fixtures/invoice-items/valid-interoperability-offline-multi-qr-1.payload.txt"
        )
        .trim(),
    ];
    let rendered = payloads.map(|payload| {
        let svg = qr::create_invoice_items_svg(payload);
        image::load_from_memory(&qr::render_png(&svg, 768))
            .unwrap()
            .to_rgba8()
    });
    let mut canvas = image::RgbaImage::new(1_536, 900);
    image::imageops::overlay(&mut canvas, &rendered[0], 0, 0);
    image::imageops::overlay(&mut canvas, &rendered[1], 768, 0);

    let merged = qr_reader::decode_invoice_items(&image::DynamicImage::ImageRgba8(canvas)).unwrap();
    assert_eq!(merged.invoice_id, "INV-ITEMS-NINE");
    assert_eq!(merged.invoice_lines.len(), 9);
    assert_eq!(merged.invoice_lines[0].item_name.as_deref(), Some("I1"));
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
