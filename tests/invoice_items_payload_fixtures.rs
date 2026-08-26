use bysqr::{
    codec,
    invoice_items::{self, reassemble_invoice_lines, try_deserialize_invoice_items},
    Document,
};

const MIXED_JSON: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json");
const MIXED_PAYLOAD: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-mixed-lines.payload.txt");
const MULTI_1_JSON: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-multi-qr-1.json");
const MULTI_1_PAYLOAD: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-multi-qr-1.payload.txt");
const MULTI_9_JSON: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-multi-qr-9.json");
const MULTI_9_PAYLOAD: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-multi-qr-9.payload.txt");

fn payload(value: &str) -> &str {
    value.strip_suffix('\n').unwrap_or(value)
}

#[test]
fn mixed_line_payload_matches_canonical_semantics_and_header() {
    let expected = try_deserialize_invoice_items(MIXED_JSON).unwrap();
    let decoded = invoice_items::decode(payload(MIXED_PAYLOAD)).unwrap();
    assert_eq!(decoded, expected);

    let envelope = codec::decode_payload(payload(MIXED_PAYLOAD)).unwrap();
    assert_eq!(envelope.header.by_square_type, 2);
    assert_eq!(envelope.header.version, 0);
    assert_eq!(envelope.header.document_type, 0);
    assert_eq!(envelope.header.reserved, 0);
    assert_eq!(envelope.sequence.split('\t').count(), 3 + 12 * 3);

    assert_eq!(
        invoice_items::decode(&invoice_items::encode(&decoded).unwrap()).unwrap(),
        decoded
    );
    assert!(matches!(
        bysqr::decode(payload(MIXED_PAYLOAD)).unwrap(),
        Document::InvoiceItems(_)
    ));
}

#[test]
fn deployed_multi_qr_blocks_reassemble_in_sequence_order() {
    let first_expected = try_deserialize_invoice_items(MULTI_1_JSON).unwrap();
    let last_expected = try_deserialize_invoice_items(MULTI_9_JSON).unwrap();
    let first = invoice_items::decode(payload(MULTI_1_PAYLOAD)).unwrap();
    let last = invoice_items::decode(payload(MULTI_9_PAYLOAD)).unwrap();
    assert_eq!(first, first_expected);
    assert_eq!(last, last_expected);
    assert_eq!(first.invoice_lines.invoice_line.len(), 8);

    let reassembled = reassemble_invoice_lines([last, first]).unwrap();
    assert_eq!(reassembled.invoice_id, "INV-ITEMS-NINE");
    assert_eq!(reassembled.invoice_lines.len(), 9);
    assert_eq!(
        reassembled.invoice_lines[0].item_name.as_deref(),
        Some("I1")
    );
    assert_eq!(
        reassembled.invoice_lines[8].item_ean_code.as_deref(),
        Some("8580000000009")
    );
}

#[test]
fn reassembled_items_validate_against_their_parent_invoice() {
    let parent = bysqr::invoice::try_deserialize_invoice(include_str!(
        "fixtures/invoice/valid-interoperability-offline-multiple-lines.json"
    ))
    .unwrap();
    let items = invoice_items::decode(payload(MIXED_PAYLOAD)).unwrap();
    let mut reassembled = reassemble_invoice_lines([items]).unwrap();
    reassembled.validate_against_invoice(&parent).unwrap();

    reassembled.invoice_lines.pop();
    assert_eq!(
        reassembled
            .validate_against_invoice(&parent)
            .unwrap_err()
            .field(),
        "InvoiceLines"
    );
}

#[test]
fn canonical_json_and_xml_use_generic_document_dispatch() {
    let json = bysqr::try_deserialize(MIXED_JSON).unwrap();
    let xml = json.to_xml().unwrap();
    assert!(xml.starts_with("<InvoiceItems"));
    assert!(xml.contains("xsi:type=\"InvoiceItems\""));
    assert!(matches!(
        bysqr::try_deserialize(&xml).unwrap(),
        Document::InvoiceItems(_)
    ));
}
