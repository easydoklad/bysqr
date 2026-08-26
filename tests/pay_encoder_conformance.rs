use bysqr::{
    codec::decode_payload,
    error::Error,
    pay::{decoder, encoder, try_deserialize_pay},
};
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

fn assert_sequence_fixture(content: &str) {
    let fixture: SequenceFixture = serde_json::from_str(content).unwrap();
    assert_eq!(fixture.schema, "spec/bysquare.xsd");

    let pay = try_deserialize_pay(&fixture.source).unwrap();
    let sequence = encoder::encode_sequence(&pay).unwrap();
    assert_eq!(sequence, fixture.expected_sequence, "{}", fixture.name);

    let payload = encoder::encode(&pay).unwrap();
    assert_eq!(decode_payload(&payload).unwrap().sequence, sequence);
}

#[test]
fn xsd_derived_bulk_fixture_uses_beneficiary_tail_ordering() {
    assert_sequence_fixture(include_str!("fixtures/pay/xsd-bulk-payment-order.json"));
}

#[test]
fn xsd_derived_standing_order_fixture() {
    assert_sequence_fixture(include_str!("fixtures/pay/xsd-standing-order.json"));
}

#[test]
fn xsd_derived_sepa_direct_debit_fixture() {
    assert_sequence_fixture(include_str!("fixtures/pay/xsd-direct-debit-sepa.json"));
}

#[test]
fn xsd_derived_other_direct_debit_fixture() {
    assert_sequence_fixture(include_str!("fixtures/pay/xsd-direct-debit-other.json"));
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

    let sequence =
        encoder::encode_sequence_with_limit(&oversized, encoder::SequenceLimit::Unbounded).unwrap();
    assert!(sequence.chars().count() > encoder::MAX_SEQUENCE_CHARACTERS);

    let payload =
        encoder::encode_with_limit(&oversized, encoder::SequenceLimit::Unbounded).unwrap();
    assert_eq!(decoder::decode(&payload).unwrap(), oversized);
}

#[test]
fn unbounded_mode_still_enforces_the_protocol_size_limit() {
    let payments = (0..2_000).map(|_| minimal_payment(json!({}))).collect();
    let pay = try_deserialize_pay(&pay_with(payments).to_string()).unwrap();

    assert!(matches!(
        encoder::encode_with_limit(&pay, encoder::SequenceLimit::Unbounded),
        Err(Error::PayloadTooLong(_))
    ));
}

#[test]
fn validates_standing_order_option_and_extension_pair() {
    let missing_extension = pay_with(vec![minimal_payment(json!({
        "PaymentOptions": "standingorder"
    }))]);
    let pay = try_deserialize_pay(&missing_extension.to_string()).unwrap();
    assert!(matches!(
        encoder::encode(&pay),
        Err(Error::InvalidInput {
            field: "StandingOrderExt",
            ..
        })
    ));

    let missing_option = pay_with(vec![minimal_payment(json!({
        "StandingOrderExt": { "Periodicity": "Daily" }
    }))]);
    let pay = try_deserialize_pay(&missing_option.to_string()).unwrap();
    assert!(matches!(
        encoder::encode(&pay),
        Err(Error::InvalidInput {
            field: "PaymentOptions",
            ..
        })
    ));
}

#[test]
fn validates_standing_order_day_and_month_rules() {
    let cases = [
        (
            "daily day",
            json!({ "Day": 1, "Periodicity": "Daily" }),
            "Day",
        ),
        (
            "weekly day",
            json!({ "Day": 8, "Periodicity": "Weekly" }),
            "Day",
        ),
        (
            "monthly day",
            json!({ "Day": 32, "Periodicity": "Monthly" }),
            "Day",
        ),
        (
            "quarterly months",
            json!({ "Month": "January April", "Periodicity": "Quarterly" }),
            "Month",
        ),
        (
            "invalid last date",
            json!({ "Periodicity": "Daily", "LastDate": "2026-02-30" }),
            "LastDate",
        ),
    ];

    for (name, extension, expected_field) in cases {
        let source = pay_with(vec![minimal_payment(json!({
            "PaymentOptions": "standingorder",
            "StandingOrderExt": extension
        }))]);
        let pay = try_deserialize_pay(&source.to_string()).unwrap();
        assert!(
            matches!(
                encoder::encode(&pay),
                Err(Error::InvalidInput { field, .. }) if field == expected_field
            ),
            "{name} was accepted"
        );
    }
}

#[test]
fn accepts_weekly_day_and_month_selection() {
    let source = pay_with(vec![minimal_payment(json!({
        "PaymentOptions": "standingorder paymentorder",
        "StandingOrderExt": {
            "Day": 7,
            "Month": "January December",
            "Periodicity": "Weekly"
        }
    }))]);
    let pay = try_deserialize_pay(&source.to_string()).unwrap();
    let sequence = encoder::encode_sequence(&pay).unwrap();
    let fields: Vec<_> = sequence.split('\t').collect();

    assert_eq!(fields[2], "3");
    assert_eq!(&fields[14..20], &["1", "7", "2049", "w", "", "0"]);
}

#[test]
fn validates_direct_debit_option_and_extension_pair() {
    let missing_extension = pay_with(vec![minimal_payment(json!({
        "PaymentOptions": "directdebit"
    }))]);
    let pay = try_deserialize_pay(&missing_extension.to_string()).unwrap();
    assert!(matches!(
        encoder::encode(&pay),
        Err(Error::InvalidInput {
            field: "DirectDebitExt",
            ..
        })
    ));

    let missing_option = pay_with(vec![minimal_payment(json!({
        "DirectDebitExt": {
            "DirectDebitScheme": "other",
            "DirectDebitType": "one-off"
        }
    }))]);
    let pay = try_deserialize_pay(&missing_option.to_string()).unwrap();
    assert!(matches!(
        encoder::encode(&pay),
        Err(Error::InvalidInput {
            field: "PaymentOptions",
            ..
        })
    ));
}

#[test]
fn validates_direct_debit_reference_choice() {
    let cases = [
        (
            "mixed other references",
            json!({
                "DirectDebitScheme": "other",
                "DirectDebitType": "one-off",
                "VariableSymbol": "1",
                "OriginatorsReferenceInformation": "RF00"
            }),
        ),
        (
            "missing SEPA identifiers",
            json!({
                "DirectDebitScheme": "SEPA",
                "DirectDebitType": "recurrent"
            }),
        ),
        (
            "other with SEPA identifiers",
            json!({
                "DirectDebitScheme": "other",
                "DirectDebitType": "recurrent",
                "MandateID": "MANDATE-1",
                "CreditorID": "CREDITOR-1"
            }),
        ),
        (
            "zero maximum",
            json!({
                "DirectDebitScheme": "other",
                "DirectDebitType": "recurrent",
                "MaxAmount": "0"
            }),
        ),
        (
            "invalid validity date",
            json!({
                "DirectDebitScheme": "other",
                "DirectDebitType": "recurrent",
                "ValidTillDate": "not-a-date"
            }),
        ),
    ];

    for (name, extension) in cases {
        let source = pay_with(vec![minimal_payment(json!({
            "PaymentOptions": "directdebit",
            "DirectDebitExt": extension
        }))]);
        let pay = try_deserialize_pay(&source.to_string()).unwrap();
        assert!(
            matches!(encoder::encode(&pay), Err(Error::InvalidInput { .. })),
            "{name} was accepted"
        );
    }
}

#[test]
fn combines_standing_order_and_direct_debit_extensions() {
    let source = pay_with(vec![minimal_payment(json!({
        "PaymentOptions": "paymentorder standingorder directdebit",
        "StandingOrderExt": {
            "Periodicity": "Daily"
        },
        "DirectDebitExt": {
            "DirectDebitScheme": "other",
            "DirectDebitType": "recurrent",
            "OriginatorsReferenceInformation": "RF18539007547034"
        }
    }))]);
    let pay = try_deserialize_pay(&source.to_string()).unwrap();
    let sequence = encoder::encode_sequence(&pay).unwrap();
    let fields: Vec<_> = sequence.split('\t').collect();

    assert_eq!(fields[2], "7");
    assert_eq!(&fields[14..20], &["1", "", "", "d", "", "1"]);
    assert_eq!(
        &fields[20..30],
        &["0", "1", "", "", "RF18539007547034", "", "", "", "", ""]
    );

    let payload = encoder::encode(&pay).unwrap();
    assert_eq!(decoder::decode(&payload).unwrap(), pay);
}
