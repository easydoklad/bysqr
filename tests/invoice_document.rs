use bysqr::{codec, document, invoice, Document};

const MINIMAL_INVOICE: &str = include_str!("fixtures/invoice/schema/minimal-header-invoice.json");
const CURRENT_PAYLOAD: &str =
    include_str!("fixtures/invoice/valid-interoperability-offline-official-current.payload.txt");

fn minimal_invoice() -> invoice::Invoice {
    invoice::try_deserialize_invoice(MINIMAL_INVOICE).unwrap()
}

#[test]
fn classifies_and_round_trips_invoice_data_and_payloads() {
    let expected = minimal_invoice();
    let document = document::try_deserialize(MINIMAL_INVOICE).unwrap();
    assert_eq!(document, Document::Invoice(Box::new(expected.clone())));

    let payload = document.encode().unwrap();
    let envelope = codec::decode_payload(&payload).unwrap();
    assert_eq!(envelope.header.by_square_type, 1);
    assert_eq!(envelope.header.version, 0);
    assert_eq!(envelope.header.document_type, 0);
    assert_eq!(envelope.header.reserved, 0);
    assert_eq!(document::decode(&payload).unwrap(), document);
}

#[test]
fn generic_json_and_xml_preserve_the_invoice_type() {
    let document = Document::Invoice(Box::new(minimal_invoice()));

    let json = document.to_json_pretty().unwrap();
    assert!(json.contains(r#""DocumentType": "Invoice""#));
    assert_eq!(document::try_deserialize(&json).unwrap(), document);

    let xml = document.to_xml().unwrap();
    assert!(xml.contains("xsi:type=\"Invoice\""));
    assert_eq!(document::try_deserialize(&xml).unwrap(), document);
}

#[test]
fn generic_decoder_accepts_the_valid_current_invoice_fixture() {
    let Document::Invoice(invoice) = document::decode(CURRENT_PAYLOAD.trim()).unwrap() else {
        panic!("fixture was not classified as INVOICE by square");
    };

    assert_eq!(invoice.document_type, invoice::DocumentType::Invoice);
    assert_eq!(
        invoice.data.tax_category_summaries.tax_category_summary[0]
            .classified_tax_category
            .as_str(),
        "0.2"
    );
}
