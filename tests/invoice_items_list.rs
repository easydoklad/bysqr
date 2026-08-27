use bysqr::invoice_items::{InvoiceItemsList, InvoiceLines};
use serde_json::{json, Value};

const JSON_SCHEMA: &str = include_str!("../spec/invoice-items-list.schema.json");

fn valid_value() -> Value {
    json!({
        "InvoiceID": "INV-LIST",
        "InvoiceLines": {
            "InvoiceLine": [
                {
                    "ItemName": "First service",
                    "InvoicedQuantity": "1",
                    "UnitPriceTaxExclusiveAmount": "10",
                    "UnitPriceTaxAmount": "2",
                    "ClassifiedTaxCategory": "0.2"
                },
                {
                    "ItemEANCode": "8580000000002",
                    "InvoicedQuantity": "2.5",
                    "UnitPriceTaxExclusiveAmount": "4",
                    "UnitPriceTaxAmount": "0.8",
                    "ClassifiedTaxCategory": "0.2"
                }
            ]
        }
    })
}

fn valid_list() -> InvoiceItemsList {
    serde_json::from_value(valid_value()).expect("valid InvoiceItemsList")
}

#[test]
fn canonical_json_round_trip_preserves_order_and_has_only_aggregate_fields() {
    let document = valid_list();
    assert_eq!(
        document.invoice_lines.invoice_line[0].item_name.as_deref(),
        Some("First service")
    );
    assert_eq!(
        document.invoice_lines.invoice_line[1]
            .item_ean_code
            .as_deref(),
        Some("8580000000002")
    );

    let canonical = serde_json::to_value(&document).unwrap();
    let fields = canonical.as_object().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.contains_key("InvoiceID"));
    assert!(fields.contains_key("InvoiceLines"));
    assert!(!fields.contains_key("FirstInvoiceLineID"));
    assert_eq!(
        serde_json::from_value::<InvoiceItemsList>(canonical).unwrap(),
        document
    );
}

#[test]
fn json_rejects_first_invoice_line_id_and_empty_or_invalid_lines() {
    let mut with_first_line_id = valid_value();
    with_first_line_id["FirstInvoiceLineID"] = json!("1");
    assert!(serde_json::from_value::<InvoiceItemsList>(with_first_line_id).is_err());

    let mut empty = valid_value();
    empty["InvoiceLines"]["InvoiceLine"] = json!([]);
    let error = serde_json::from_value::<InvoiceItemsList>(empty).unwrap_err();
    assert!(error.to_string().contains("at least one InvoiceLine"));

    let mut invalid_line = valid_value();
    invalid_line["InvoiceLines"]["InvoiceLine"][0]["ItemEANCode"] = json!("8580000000001");
    let error = serde_json::from_value::<InvoiceItemsList>(invalid_line).unwrap_err();
    assert!(error
        .to_string()
        .contains("exactly one of ItemName and ItemEANCode"));

    let error = InvoiceItemsList::new("INV-LIST", InvoiceLines::new(Vec::new())).unwrap_err();
    assert_eq!(error.field(), "InvoiceLines");
}

#[test]
fn canonical_xml_round_trip_is_unbranded_and_rejects_other_roots() {
    let document = valid_list();
    let xml = document.to_xml_string().unwrap();
    assert!(xml.starts_with("<InvoiceItemsList>"));
    assert!(xml.ends_with("</InvoiceItemsList>"));
    assert!(xml.contains("<InvoiceLines><InvoiceLine>"));
    assert!(!xml.contains("FirstInvoiceLineID"));
    assert!(!xml.contains("xmlns"));
    assert!(!xml.contains("xsi:type"));
    assert_eq!(InvoiceItemsList::from_xml_str(&xml).unwrap(), document);

    let wrong_root = xml
        .replacen("<InvoiceItemsList>", "<InvoiceItems>", 1)
        .replacen("</InvoiceItemsList>", "</InvoiceItems>", 1);
    let error = InvoiceItemsList::from_xml_str(&wrong_root).unwrap_err();
    assert_eq!(error.field(), "InvoiceItemsList XML");
    assert!(error
        .to_string()
        .contains("root element must be InvoiceItemsList"));
}

#[test]
fn xml_rejects_block_only_fields_branding_and_empty_lines() {
    let xml = valid_list().to_xml_string().unwrap();

    let with_first_line_id = xml.replacen(
        "<InvoiceLines>",
        "<FirstInvoiceLineID>1</FirstInvoiceLineID><InvoiceLines>",
        1,
    );
    assert!(InvoiceItemsList::from_xml_str(&with_first_line_id).is_err());

    let branded = xml.replacen(
        "<InvoiceItemsList>",
        "<InvoiceItemsList xmlns=\"http://www.bysquare.com/bysquare\">",
        1,
    );
    assert!(InvoiceItemsList::from_xml_str(&branded).is_err());

    let typed = xml.replacen(
        "<InvoiceItemsList>",
        "<InvoiceItemsList xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"InvoiceItemsList\">",
        1,
    );
    assert!(InvoiceItemsList::from_xml_str(&typed).is_err());

    let empty = r#"<InvoiceItemsList><InvoiceID>INV-LIST</InvoiceID><InvoiceLines><InvoiceLine/></InvoiceLines></InvoiceItemsList>"#;
    assert!(InvoiceItemsList::from_xml_str(empty).is_err());
    let no_lines =
        r#"<InvoiceItemsList><InvoiceID>INV-LIST</InvoiceID><InvoiceLines/></InvoiceItemsList>"#;
    let error = InvoiceItemsList::from_xml_str(no_lines).unwrap_err();
    assert!(error.to_string().contains("at least one InvoiceLine"));
}

#[test]
fn standalone_schema_is_valid_and_matches_the_aggregate_contract() {
    let schema: Value = serde_json::from_str(JSON_SCHEMA).unwrap();
    jsonschema::draft202012::meta::validate(&schema).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();

    let valid = valid_value();
    assert!(validator.is_valid(&valid));

    let mut with_first_line_id = valid.clone();
    with_first_line_id["FirstInvoiceLineID"] = json!("1");
    assert!(!validator.is_valid(&with_first_line_id));

    let mut empty = valid.clone();
    empty["InvoiceLines"]["InvoiceLine"] = json!([]);
    assert!(!validator.is_valid(&empty));

    let mut invalid_line = valid;
    invalid_line["InvoiceLines"]["InvoiceLine"][0]["ItemEANCode"] = json!("8580000000001");
    assert!(!validator.is_valid(&invalid_line));
}
