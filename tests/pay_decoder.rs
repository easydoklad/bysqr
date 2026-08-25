use bysqr::{
    codec::{encode_payload, Header},
    decoder, encoder,
    error::Error,
    models::try_deserialize_pay,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SequenceFixture {
    source: String,
    expected_sequence: String,
}

#[derive(Debug, Deserialize)]
struct PayloadFixture {
    source: String,
    payload: String,
    expected_sequence: String,
}

fn assert_sequence_round_trip(content: &str) {
    let fixture: SequenceFixture = serde_json::from_str(content).unwrap();
    let expected = try_deserialize_pay(&fixture.source).unwrap();

    let decoded_sequence = decoder::decode_sequence(&fixture.expected_sequence).unwrap();
    assert_eq!(decoded_sequence, expected);
    assert_eq!(
        encoder::encode_sequence(&decoded_sequence).unwrap(),
        fixture.expected_sequence
    );

    let payload = encoder::encode(&expected).unwrap();
    assert_eq!(decoder::decode(&payload).unwrap(), expected);
}

fn assert_payload_fixture(content: &str) {
    let fixture: PayloadFixture = serde_json::from_str(content).unwrap();
    let expected = try_deserialize_pay(&fixture.source).unwrap();
    let decoded = decoder::decode(&fixture.payload).unwrap();

    assert_eq!(decoded, expected);
    assert_eq!(
        encoder::encode_sequence(&decoded).unwrap(),
        fixture.expected_sequence
    );
}

#[test]
fn decodes_valid_payload_fixtures() {
    assert_payload_fixture(include_str!("fixtures/pay/valid-payment-order.json"));
    assert_payload_fixture(include_str!("fixtures/pay/valid-precision.json"));
}

#[test]
fn round_trips_every_xsd_sequence_fixture() {
    for fixture in [
        include_str!("fixtures/pay/xsd-bulk-payment-order.json"),
        include_str!("fixtures/pay/xsd-standing-order.json"),
        include_str!("fixtures/pay/xsd-direct-debit-sepa.json"),
        include_str!("fixtures/pay/xsd-direct-debit-other.json"),
    ] {
        assert_sequence_round_trip(fixture);
    }
}

#[test]
fn restores_bulk_beneficiaries_to_their_payments() {
    let fixture: SequenceFixture =
        serde_json::from_str(include_str!("fixtures/pay/xsd-bulk-payment-order.json")).unwrap();
    let pay = decoder::decode_sequence(&fixture.expected_sequence).unwrap();

    assert_eq!(
        pay.payments.payment[0].beneficiary_name.as_deref(),
        Some("Alice Example")
    );
    assert_eq!(
        pay.payments.payment[1].beneficiary_name.as_deref(),
        Some("Bob Example")
    );
}

#[test]
fn rejects_non_pay_headers_and_invalid_base32() {
    let payload = encode_payload(
        Header {
            by_square_type: 1,
            version: 0,
            document_type: 0,
            reserved: 0,
        },
        "\t1",
    )
    .unwrap();
    assert!(matches!(
        decoder::decode(&payload),
        Err(Error::InvalidPayload(_))
    ));
    assert!(matches!(
        decoder::decode("NOT-A-BASE32-PAYLOAD!"),
        Err(Error::InvalidPayload(_))
    ));
}

#[test]
fn rejects_missing_trailing_and_invalid_count_fields() {
    let fixture: PayloadFixture =
        serde_json::from_str(include_str!("fixtures/pay/valid-payment-order.json")).unwrap();
    let fields: Vec<_> = fixture.expected_sequence.split('\t').collect();

    let mut missing = fields.clone();
    missing.pop();
    assert!(matches!(
        decoder::decode_sequence(&missing.join("\t")),
        Err(Error::InvalidSequence { .. })
    ));

    let mut trailing = fields.clone();
    trailing.push("unexpected");
    assert!(matches!(
        decoder::decode_sequence(&trailing.join("\t")),
        Err(Error::InvalidSequence { .. })
    ));

    for (index, invalid) in [(1, "0"), (1, "NaN"), (11, "0"), (14, "2")] {
        let mut malformed = fields.clone();
        malformed[index] = invalid;
        assert!(
            decoder::decode_sequence(&malformed.join("\t")).is_err(),
            "field {index} accepted {invalid:?}"
        );
    }
}

#[test]
fn rejects_unknown_classifiers() {
    let fixture: SequenceFixture =
        serde_json::from_str(include_str!("fixtures/pay/xsd-standing-order.json")).unwrap();
    let fields: Vec<_> = fixture.expected_sequence.split('\t').collect();

    for (index, invalid) in [(2, "8"), (16, "4096"), (17, "x")] {
        let mut malformed = fields.clone();
        malformed[index] = invalid;
        assert!(matches!(
            decoder::decode_sequence(&malformed.join("\t")),
            Err(Error::InvalidSequence { .. }) | Err(Error::InvalidPayload(_))
        ));
    }
}

#[test]
fn serializes_decoded_pay_as_json_and_xml() {
    let fixture: SequenceFixture =
        serde_json::from_str(include_str!("fixtures/pay/xsd-direct-debit-sepa.json")).unwrap();
    let decoded = decoder::decode_sequence(&fixture.expected_sequence).unwrap();

    let json = serde_json::to_string_pretty(&decoded).unwrap();
    assert_eq!(try_deserialize_pay(&json).unwrap(), decoded);
    assert!(json.contains(r#""PaymentOptions": "paymentorder directdebit""#));

    let xml = quick_xml::se::to_string(&decoded).unwrap();
    assert_eq!(try_deserialize_pay(&xml).unwrap(), decoded);
    assert!(xml.contains("<DirectDebitScheme>SEPA</DirectDebitScheme>"));
}
