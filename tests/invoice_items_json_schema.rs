use bysqr::invoice_items::{try_deserialize_invoice_items, JSON_SCHEMA};
use serde_json::{json, Value};

const MINIMAL: &str =
    include_str!("fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json");

fn schema() -> Value {
    serde_json::from_str(JSON_SCHEMA).expect("InvoiceItems JSON Schema must be valid JSON")
}

fn validator() -> jsonschema::Validator {
    jsonschema::draft202012::new(&schema()).expect("InvoiceItems JSON Schema must compile")
}

fn minimal() -> Value {
    serde_json::from_str(MINIMAL).expect("InvoiceItems fixture must be valid JSON")
}

#[test]
fn schema_is_valid_draft_2020_12_and_accepts_fixture() {
    let schema = schema();
    jsonschema::draft202012::meta::validate(&schema).unwrap();
    assert!(jsonschema::draft202012::new(&schema)
        .unwrap()
        .is_valid(&minimal()));
}

#[test]
fn schema_enforces_required_fields_item_choice_period_pair_and_vat_range() {
    let validator = validator();
    let valid = minimal();
    let mut cases = Vec::new();

    let mut missing_id = valid.clone();
    missing_id.as_object_mut().unwrap().remove("InvoiceID");
    cases.push(("missing InvoiceID", missing_id));

    let mut empty_lines = valid.clone();
    empty_lines["InvoiceLines"]["InvoiceLine"] = json!([]);
    cases.push(("empty InvoiceLines", empty_lines));

    let mut both_items = valid.clone();
    both_items["InvoiceLines"]["InvoiceLine"][0]["ItemEANCode"] = json!("8580000000000");
    cases.push(("both item identifiers", both_items));

    let mut incomplete_period = valid.clone();
    incomplete_period["InvoiceLines"]["InvoiceLine"][0]["PeriodFromDate"] = json!("2026-08-01");
    cases.push(("incomplete period", incomplete_period));

    let mut percentage_points = valid.clone();
    percentage_points["InvoiceLines"]["InvoiceLine"][0]["ClassifiedTaxCategory"] = json!("23");
    cases.push(("VAT outside canonical range", percentage_points));

    let mut numeric_decimal = valid;
    numeric_decimal["InvoiceLines"]["InvoiceLine"][0]["InvoicedQuantity"] = json!(2);
    cases.push(("numeric canonical decimal", numeric_decimal));

    for (name, document) in cases {
        assert!(!validator.is_valid(&document), "{name} was accepted");
    }
}

#[test]
fn max_lengths_are_advisory_and_computed_fields_are_read_only() {
    fn contains_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(object) => {
                object.contains_key(key) || object.values().any(|value| contains_key(value, key))
            }
            Value::Array(array) => array.iter().any(|value| contains_key(value, key)),
            _ => false,
        }
    }

    let schema = schema();
    assert!(!contains_key(&schema, "maxLength"));
    for field in [
        "UnitPriceTaxInclusiveAmount",
        "LineTaxExclusiveAmount",
        "LineTaxInclusiveAmount",
        "LineTaxAmount",
    ] {
        assert_eq!(
            schema["$defs"]["InvoiceLine"]["properties"][field]["readOnly"],
            json!(true)
        );
    }

    let mut over_advisory = minimal();
    over_advisory["InvoiceID"] = json!("Ž".repeat(11));
    over_advisory["InvoiceLines"]["InvoiceLine"][0]["ItemName"] = json!("é".repeat(31));
    assert!(validator().is_valid(&over_advisory));

    let document = try_deserialize_invoice_items(&over_advisory.to_string()).unwrap();
    assert_eq!(document.advisory_diagnostics().len(), 2);
}

#[test]
fn rust_model_accepts_numeric_input_and_emits_canonical_strings() {
    let mut source = minimal();
    source["InvoiceLines"]["InvoiceLine"][0]["InvoicedQuantity"] = json!(2.5);
    source["InvoiceLines"]["InvoiceLine"][0]["UnitPriceTaxAmount"] = json!(20.0);
    let document = try_deserialize_invoice_items(&source.to_string()).unwrap();
    let canonical = serde_json::to_value(document).unwrap();
    assert_eq!(
        canonical["InvoiceLines"]["InvoiceLine"][0]["InvoicedQuantity"],
        json!("2.5")
    );
    assert_eq!(
        canonical["InvoiceLines"]["InvoiceLine"][0]["UnitPriceTaxAmount"],
        json!("20")
    );
    assert!(validator().is_valid(&canonical));
}
