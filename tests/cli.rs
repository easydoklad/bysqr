use std::process::Command;

use bysqr::models::{try_deserialize_pay, Pay};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    source: String,
    payload: String,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/pay/valid-payment-order.json")).unwrap()
}

#[test]
fn encodes_canonical_json_file() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pay/json/standing-order.json"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(["encode", "--src", source, "--format", "svg"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .starts_with("<svg"));
}

#[test]
fn decodes_payload_to_json() {
    let fixture = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(["decode", "--src", &fixture.payload, "--format", "json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded: Pay = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(decoded, try_deserialize_pay(&fixture.source).unwrap());
}

#[test]
fn decodes_payload_to_xml() {
    let fixture = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(["decode", "--src", &fixture.payload, "--format", "xml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded = try_deserialize_pay(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(decoded, try_deserialize_pay(&fixture.source).unwrap());
}
