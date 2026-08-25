use bysqr::{
    codec::{decode_payload, Header},
    encoder,
    models::try_deserialize_pay,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    source: String,
    payload: String,
    expected_sequence: String,
}

fn assert_fixture(content: &str) {
    let fixture: Fixture = serde_json::from_str(content).unwrap();

    let reference = decode_payload(&fixture.payload)
        .unwrap_or_else(|error| panic!("{}: fixture payload failed: {error}", fixture.name));
    assert_eq!(reference.header, Header::PAY, "{}", fixture.name);
    assert_eq!(
        reference.sequence, fixture.expected_sequence,
        "{}",
        fixture.name
    );

    let pay = try_deserialize_pay(&fixture.source)
        .unwrap_or_else(|error| panic!("{}: source failed: {error}", fixture.name));
    let generated = encoder::encode(&pay)
        .unwrap_or_else(|error| panic!("{}: encoder failed: {error}", fixture.name));
    let decoded = decode_payload(&generated)
        .unwrap_or_else(|error| panic!("{}: generated payload failed: {error}", fixture.name));

    assert_eq!(decoded.header, reference.header, "{}", fixture.name);
    assert_eq!(decoded.sequence, reference.sequence, "{}", fixture.name);
}

#[test]
fn valid_basic_payment_order() {
    assert_fixture(include_str!("fixtures/pay/valid-payment-order.json"));
}

#[test]
fn valid_invoice_id_and_decimal_precision() {
    assert_fixture(include_str!("fixtures/pay/valid-precision.json"));
}
