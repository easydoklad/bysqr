use std::process::Command;

use bysqr::{
    invoice, invoice_items,
    pay::{try_deserialize_pay, Pay},
};
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
fn encodes_canonical_invoice_json_with_invoice_branding() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/invoice/schema/minimal-header-invoice.json"
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
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("#F5871F"));
}

#[test]
fn applies_logo_theme_options_to_invoice() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/invoice/schema/minimal-header-invoice.json"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args([
            "encode",
            "--src",
            source,
            "--format",
            "svg",
            "--logo-layout",
            "electronic",
            "--logo-position",
            "left",
            "--logo-color",
            "gray",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(svg.contains("viewBox=\"0 0 600 512\""));
    assert!(svg.contains("#5F6062"));
    assert!(!svg.contains("stroke-width=\"8\""));
}

#[test]
fn applies_logo_theme_options_to_pay_with_the_pay_palette() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pay/json/standing-order.json"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args([
            "encode",
            "--src",
            source,
            "--format",
            "svg",
            "--logo-layout",
            "electronic",
            "--logo-position",
            "right",
            "--logo-color",
            "light",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(svg.contains("viewBox=\"0 0 600 512\""));
    assert!(svg.contains("#A1C7E9"));
    assert!(!svg.contains("stroke-width=\"8\""));
}

#[test]
fn rejects_logo_theme_options_for_invoice_items() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args([
            "encode",
            "--src",
            source,
            "--format",
            "svg",
            "--logo-color",
            "gray",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("only apply to PAY and INVOICE documents"));
}

#[test]
fn encodes_canonical_invoice_items_json_with_items_branding() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json"
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
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("M104.382 0C106.433"));
}

#[cfg(feature = "qr-reader")]
#[test]
fn decodes_generated_qr_image_file() {
    use bysqr::{pay, qr};

    let fixture = fixture();
    let pay = try_deserialize_pay(&fixture.source).unwrap();
    let payload = pay::encode(&pay).unwrap();
    let svg = qr::create_pay_svg(&payload, qr::LogoTheme::default());
    let png = qr::render_png(&svg, 1_024);
    let source = std::env::temp_dir().join(format!("bysqr-cli-pay-{}.png", std::process::id()));
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

#[cfg(feature = "qr-reader")]
#[test]
fn decodes_generated_invoice_qr_image_file() {
    use bysqr::qr;

    let invoice = invoice::try_deserialize_invoice(include_str!(
        "fixtures/invoice/schema/minimal-header-invoice.json"
    ))
    .unwrap();
    let payload = invoice::encode(&invoice).unwrap();
    let svg = qr::create_invoice_svg(&payload);
    let png = qr::render_png(&svg, 1_024);
    let source = std::env::temp_dir().join(format!("bysqr-cli-invoice-{}.png", std::process::id()));
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
    let decoded =
        invoice::try_deserialize_invoice(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(decoded, invoice);
}

#[cfg(not(feature = "qr-reader"))]
#[test]
fn reports_disabled_qr_image_reader() {
    use bysqr::{pay, qr};

    let fixture = fixture();
    let pay = try_deserialize_pay(&fixture.source).unwrap();
    let payload = pay::encode(&pay).unwrap();
    let svg = qr::create_pay_svg(&payload, qr::LogoTheme::default());
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

#[test]
fn decodes_invoice_payload_to_json_and_xml() {
    let payload = include_str!(
        "fixtures/invoice/valid-interoperability-offline-official-current.payload.txt"
    )
    .trim();

    for format in ["json", "xml"] {
        let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
            .args(["decode", "--src", payload, "--format", format])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let decoded =
            invoice::try_deserialize_invoice(&String::from_utf8(output.stdout).unwrap()).unwrap();
        assert_eq!(decoded.document_type, invoice::DocumentType::Invoice);
        assert_eq!(
            decoded.data.tax_category_summaries.tax_category_summary[0]
                .classified_tax_category
                .as_str(),
            "0.2"
        );
    }
}

#[test]
fn decodes_invoice_items_payload_to_json_and_xml() {
    let payload = include_str!(
        "fixtures/invoice-items/valid-interoperability-offline-mixed-lines.payload.txt"
    )
    .trim();

    for format in ["json", "xml"] {
        let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
            .args(["decode", "--src", payload, "--format", format])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let decoded = invoice_items::try_deserialize_invoice_items(
            &String::from_utf8(output.stdout).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.invoice_id, "INV-MULTI-2026");
        assert_eq!(decoded.invoice_lines.invoice_line.len(), 3);
    }
}
