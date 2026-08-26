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

#[cfg(feature = "qr-reader")]
#[test]
fn decodes_generated_qr_image_file() {
    use bysqr::{encoder, qr};

    let fixture = fixture();
    let pay = try_deserialize_pay(&fixture.source).unwrap();
    let payload = encoder::encode(&pay).unwrap();
    let svg = qr::create_pay_svg(&payload, qr::Theme::default());
    let png = qr::render_png(&svg, 1_024);
    let source = std::env::temp_dir().join(format!("bysqr-cli-{}.png", std::process::id()));
    std::fs::write(&source, png).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args([
            "decode",
            "--src",
            source.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    std::fs::remove_file(source).unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded: Pay = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(decoded, pay);
}

#[cfg(not(feature = "qr-reader"))]
#[test]
fn reports_disabled_qr_image_reader() {
    use bysqr::{encoder, qr};

    let fixture = fixture();
    let pay = try_deserialize_pay(&fixture.source).unwrap();
    let payload = encoder::encode(&pay).unwrap();
    let svg = qr::create_pay_svg(&payload, qr::Theme::default());
    let png = qr::render_png(&svg, 512);
    let source = std::env::temp_dir().join(format!("bysqr-cli-{}.png", std::process::id()));
    std::fs::write(&source, png).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(["decode", "--src", source.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(source).unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("qr-reader feature"));
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
