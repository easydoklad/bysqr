use bysqr::{codec::decode_payload, encoder, error::Error, models::try_deserialize_pay};
use serde::Deserialize;
use serde_json::json;

const DEMO_IBAN: &str = "SK7700000000000000000000";

#[derive(Debug, Deserialize)]
struct SequenceFixture {
    name: String,
    schema: String,
    source: String,
    expected_sequence: String,
}

fn minimal_payment(overrides: serde_json::Value) -> serde_json::Value {
    let mut payment = json!({
        "PaymentOptions": "paymentorder",
        "Amount": "1",
        "CurrencyCode": "EUR",
        "BankAccounts": {
            "BankAccount": [{ "IBAN": DEMO_IBAN }]
        }
    });

    let object = payment.as_object_mut().unwrap();
    for (key, value) in overrides.as_object().unwrap() {
        object.insert(key.clone(), value.clone());
    }
    payment
}

fn pay_with(payments: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "Payments": { "Payment": payments } })
}

#[test]
fn xsd_derived_bulk_fixture_uses_beneficiary_tail_ordering() {
    let fixture: SequenceFixture =
        serde_json::from_str(include_str!("fixtures/pay/xsd-bulk-payment-order.json")).unwrap();
    assert_eq!(fixture.schema, "spec/bysquare.xsd");

    let pay = try_deserialize_pay(&fixture.source).unwrap();
    let sequence = encoder::encode_sequence(&pay).unwrap();
    assert_eq!(sequence, fixture.expected_sequence, "{}", fixture.name);

    let payload = encoder::encode(&pay).unwrap();
    assert_eq!(decode_payload(&payload).unwrap().sequence, sequence);
}

#[test]
fn tabs_in_values_are_replaced_with_spaces() {
    let source = pay_with(vec![minimal_payment(json!({
        "PaymentNote": "left\tright",
        "BeneficiaryName": "Fixture\tRecipient"
    }))]);
    let pay = try_deserialize_pay(&source.to_string()).unwrap();
    let sequence = encoder::encode_sequence(&pay).unwrap();

    assert!(sequence.contains("left right"));
    assert!(sequence.contains("Fixture Recipient"));
    assert!(!sequence.contains("left\tright"));
}

#[test]
fn payload_length_uses_both_little_endian_bytes() {
    let source = pay_with(vec![minimal_payment(json!({
        "PaymentNote": "Ž".repeat(140),
        "BeneficiaryName": "Č".repeat(140),
        "BeneficiaryAddressLine1": "Ľ".repeat(70),
        "BeneficiaryAddressLine2": "Š".repeat(70)
    }))]);
    let pay = try_deserialize_pay(&source.to_string()).unwrap();
    let sequence = encoder::encode_sequence(&pay).unwrap();
    assert!(sequence.len() + 4 > u8::MAX as usize);
    assert!(sequence.chars().count() <= encoder::MAX_SEQUENCE_CHARACTERS);

    let payload = encoder::encode(&pay).unwrap();
    assert_eq!(decode_payload(&payload).unwrap().sequence, sequence);
}

#[test]
fn rejects_values_outside_xsd_constraints() {
    let cases = [
        (
            "currency",
            pay_with(vec![minimal_payment(json!({ "CurrencyCode": "EURX" }))]),
        ),
        (
            "zero amount",
            pay_with(vec![minimal_payment(json!({ "Amount": "0.00" }))]),
        ),
        (
            "empty accounts",
            pay_with(vec![minimal_payment(json!({
                "BankAccounts": { "BankAccount": [] }
            }))]),
        ),
        (
            "reference choice",
            pay_with(vec![minimal_payment(json!({
                "VariableSymbol": "1",
                "OriginatorsReferenceInformation": "RF00"
            }))]),
        ),
    ];

    for (name, source) in cases {
        let pay = try_deserialize_pay(&source.to_string()).unwrap();
        assert!(
            matches!(encoder::encode(&pay), Err(Error::InvalidInput { .. })),
            "{name} was accepted"
        );
    }
}

#[test]
fn rejects_empty_and_oversized_payment_collections() {
    let empty = try_deserialize_pay(&pay_with(vec![]).to_string()).unwrap();
    assert!(matches!(
        encoder::encode(&empty),
        Err(Error::InvalidInput {
            field: "Payments",
            ..
        })
    ));

    let payments = (0..4)
        .map(|_| minimal_payment(json!({ "PaymentNote": "x".repeat(140) })))
        .collect();
    let oversized = try_deserialize_pay(&pay_with(payments).to_string()).unwrap();
    assert!(matches!(
        encoder::encode(&oversized),
        Err(Error::SequenceTooLong { .. })
    ));
}

#[test]
fn reports_unimplemented_payment_extensions() {
    let source = pay_with(vec![minimal_payment(json!({
        "PaymentOptions": "paymentorder standingorder",
        "StandingOrderExt": {
            "Day": 1,
            "Periodicity": "monthly"
        }
    }))]);
    let pay = try_deserialize_pay(&source.to_string()).unwrap();

    assert!(matches!(encoder::encode(&pay), Err(Error::Unsupported(_))));
}
