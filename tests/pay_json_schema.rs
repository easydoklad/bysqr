use bysqr::{
    diagnostic::AdvisoryDiagnostic,
    pay::{decoder, encoder, try_deserialize_pay, JSON_SCHEMA},
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct SequenceFixture {
    source: String,
    expected_sequence: String,
}

fn schema() -> Value {
    serde_json::from_str(JSON_SCHEMA).expect("PAY JSON Schema must be valid JSON")
}

fn fixture_pairs() -> [(&'static str, &'static str); 4] {
    [
        (
            include_str!("fixtures/pay/json/bulk-payment-order.json"),
            include_str!("fixtures/pay/xsd-bulk-payment-order.json"),
        ),
        (
            include_str!("fixtures/pay/json/standing-order.json"),
            include_str!("fixtures/pay/xsd-standing-order.json"),
        ),
        (
            include_str!("fixtures/pay/json/direct-debit-sepa.json"),
            include_str!("fixtures/pay/xsd-direct-debit-sepa.json"),
        ),
        (
            include_str!("fixtures/pay/json/direct-debit-other.json"),
            include_str!("fixtures/pay/xsd-direct-debit-other.json"),
        ),
    ]
}

#[test]
fn pay_schema_is_valid_draft_2020_12() {
    let schema = schema();
    jsonschema::draft202012::meta::validate(&schema).unwrap();
    jsonschema::draft202012::new(&schema).unwrap();
}

#[test]
fn canonical_json_fixtures_match_xsd_fixtures_and_round_trip() {
    let schema = schema();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let documented_example_source = include_str!("../example/payment.json");
    let documented_example: Value = serde_json::from_str(documented_example_source).unwrap();
    assert!(validator.is_valid(&documented_example));
    encoder::encode(&try_deserialize_pay(documented_example_source).unwrap()).unwrap();

    for (json_source, xsd_fixture) in fixture_pairs() {
        let document: Value = serde_json::from_str(json_source).unwrap();
        let errors: Vec<_> = validator
            .iter_errors(&document)
            .map(|error| error.to_string())
            .collect();
        assert!(errors.is_empty(), "invalid JSON fixture: {errors:#?}");

        let fixture: SequenceFixture = serde_json::from_str(xsd_fixture).unwrap();
        let from_json = try_deserialize_pay(json_source).unwrap();
        let from_xml = try_deserialize_pay(&fixture.source).unwrap();
        assert_eq!(from_json, from_xml);
        assert_eq!(
            encoder::encode_sequence(&from_json).unwrap(),
            fixture.expected_sequence
        );

        let payload = encoder::encode(&from_json).unwrap();
        let decoded = decoder::decode(&payload).unwrap();
        assert_eq!(decoded, from_json);

        let canonical_output = serde_json::to_value(decoded).unwrap();
        assert!(validator.is_valid(&canonical_output));
    }
}

#[test]
fn schema_rejects_structurally_and_semantically_invalid_documents() {
    let schema = schema();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let valid: Value =
        serde_json::from_str(include_str!("fixtures/pay/json/standing-order.json")).unwrap();

    let mut cases = Vec::new();

    let mut missing_payments = valid.clone();
    missing_payments.as_object_mut().unwrap().remove("Payments");
    cases.push(("missing Payments", missing_payments));

    let mut unknown_property = valid.clone();
    unknown_property["Unknown"] = json!(true);
    cases.push(("unknown property", unknown_property));

    let mut empty_payments = valid.clone();
    empty_payments["Payments"]["Payment"] = json!([]);
    cases.push(("empty payment list", empty_payments));

    let mut invalid_currency = valid.clone();
    invalid_currency["Payments"]["Payment"][0]["CurrencyCode"] = json!("eur");
    cases.push(("invalid currency", invalid_currency));

    let mut zero_amount = valid.clone();
    zero_amount["Payments"]["Payment"][0]["Amount"] = json!("0");
    cases.push(("zero amount", zero_amount));

    let mut numeric_amount = valid.clone();
    numeric_amount["Payments"]["Payment"][0]["Amount"] = json!(42.5);
    cases.push(("non-canonical numeric amount", numeric_amount));

    let mut mixed_references = valid.clone();
    mixed_references["Payments"]["Payment"][0]["OriginatorsReferenceInformation"] =
        json!("RF18539007547034");
    cases.push(("mixed payment references", mixed_references));

    let mut missing_standing_extension = valid.clone();
    missing_standing_extension["Payments"]["Payment"][0]
        .as_object_mut()
        .unwrap()
        .remove("StandingOrderExt");
    cases.push((
        "missing standing-order extension",
        missing_standing_extension,
    ));

    let mut daily_with_day = valid.clone();
    daily_with_day["Payments"]["Payment"][0]["StandingOrderExt"]["Periodicity"] = json!("Daily");
    cases.push(("daily order with day and months", daily_with_day));

    let mut duplicate_month = valid.clone();
    duplicate_month["Payments"]["Payment"][0]["StandingOrderExt"]["Month"] =
        json!("January January");
    cases.push(("duplicate month", duplicate_month));

    let mut missing_direct_debit = valid;
    missing_direct_debit["Payments"]["Payment"][0]["PaymentOptions"] =
        json!("paymentorder directdebit");
    missing_direct_debit["Payments"]["Payment"][0]
        .as_object_mut()
        .unwrap()
        .remove("StandingOrderExt");
    cases.push(("missing direct-debit extension", missing_direct_debit));

    for (name, document) in cases {
        assert!(!validator.is_valid(&document), "{name} was accepted");
    }
}

#[test]
fn schema_rejects_invalid_direct_debit_reference_variants() {
    let schema = schema();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let sepa: Value =
        serde_json::from_str(include_str!("fixtures/pay/json/direct-debit-sepa.json")).unwrap();
    let other: Value =
        serde_json::from_str(include_str!("fixtures/pay/json/direct-debit-other.json")).unwrap();

    let mut sepa_without_mandate = sepa;
    sepa_without_mandate["Payments"]["Payment"][0]["DirectDebitExt"]
        .as_object_mut()
        .unwrap()
        .remove("MandateID");

    let mut other_with_mandate = other.clone();
    other_with_mandate["Payments"]["Payment"][0]["DirectDebitExt"]["MandateID"] =
        json!("MANDATE-1");

    let mut other_with_mixed_references = other;
    other_with_mixed_references["Payments"]["Payment"][0]["DirectDebitExt"]
        ["OriginatorsReferenceInformation"] = json!("RF18539007547034");

    for (name, document) in [
        ("SEPA without mandate", sepa_without_mandate),
        ("other with mandate", other_with_mandate),
        ("other with mixed references", other_with_mixed_references),
    ] {
        assert!(!validator.is_valid(&document), "{name} was accepted");
    }
}

#[test]
fn bsqr_max_lengths_are_advisory_in_schema_model_and_encoder() {
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
    let validator = jsonschema::draft202012::new(&schema).unwrap();

    let source = json!({
        "InvoiceID": "12345678901",
        "Payments": {
            "Payment": [{
                "PaymentOptions": "paymentorder",
                "Amount": "1234567890123456",
                "CurrencyCode": "EUR",
                "PaymentNote": "x".repeat(141),
                "BankAccounts": {
                    "BankAccount": [{ "IBAN": "SK7700000000000000000000" }]
                }
            }]
        }
    });
    assert!(validator.is_valid(&source));

    let pay = try_deserialize_pay(&source.to_string()).unwrap();
    assert_eq!(
        pay.advisory_diagnostics(),
        [
            AdvisoryDiagnostic {
                field_path: "InvoiceID".to_owned(),
                actual_character_count: 11,
                recommended_maximum: 10,
            },
            AdvisoryDiagnostic {
                field_path: "Payments.Payment[0].Amount".to_owned(),
                actual_character_count: 16,
                recommended_maximum: 15,
            },
            AdvisoryDiagnostic {
                field_path: "Payments.Payment[0].PaymentNote".to_owned(),
                actual_character_count: 141,
                recommended_maximum: 140,
            },
        ]
    );
    encoder::encode(&pay).unwrap();
}
