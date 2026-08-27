use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use base64::Engine;
use bysqr::{
    invoice, invoice_items,
    invoice_items::InvoiceItemsList,
    pay::{try_deserialize_pay, Pay},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

static TEMP_PATH_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[derive(Deserialize)]
struct Fixture {
    source: String,
    payload: String,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/pay/valid-payment-order.json")).unwrap()
}

fn run_with_stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input).unwrap();
    drop(stdin);

    child.wait_with_output().unwrap()
}

fn items_list(invoice_id: &str, count: usize) -> InvoiceItemsList {
    let lines = (1..=count)
        .map(|index| {
            json!({
                "ItemName": format!("Batch item {index}"),
                "InvoicedQuantity": "1",
                "UnitPriceTaxExclusiveAmount": "10",
                "UnitPriceTaxAmount": "2",
                "ClassifiedTaxCategory": "0.2"
            })
        })
        .collect::<Vec<_>>();
    serde_json::from_value(json!({
        "InvoiceID": invoice_id,
        "InvoiceLines": { "InvoiceLine": lines }
    }))
    .unwrap()
}

fn inline_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "bysqr-cli-items-{label}-{}-{}",
        std::process::id(),
        TEMP_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn parse_string_array(output: &Output) -> Vec<String> {
    assert_success(output);
    serde_json::from_slice(&output.stdout).unwrap()
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
fn encodes_json_from_stdin_like_file_source() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pay/json/standing-order.json"
    );
    let file_output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(["encode", "--src", source, "--format", "svg"])
        .output()
        .unwrap();
    let stdin_output = run_with_stdin(
        &["encode", "--src", "-", "--format", "svg"],
        &std::fs::read(source).unwrap(),
    );

    assert!(
        file_output.status.success(),
        "{}",
        String::from_utf8_lossy(&file_output.stderr)
    );
    assert!(
        stdin_output.status.success(),
        "{}",
        String::from_utf8_lossy(&stdin_output.stderr)
    );
    let file_svg = xmltree::Element::parse(file_output.stdout.as_slice()).unwrap();
    let stdin_svg = xmltree::Element::parse(stdin_output.stdout.as_slice()).unwrap();
    assert_eq!(stdin_svg, file_svg);
    assert_eq!(stdin_svg.name, "svg");
}

#[test]
fn reports_malformed_json_from_stdin() {
    let output = run_with_stdin(&["encode", "--src", "-", "--format", "svg"], b"{");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unable to deserialize JSON"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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
fn reports_invalid_raster_options_without_panicking() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pay/json/standing-order.json"
    );
    for (format, option, value, expected) in [
        ("png", "--size", "0", "invalid size"),
        ("png", "--size", "8193", "invalid size"),
        ("jpeg", "--quality", "0", "invalid quality"),
        ("jpeg", "--quality", "101", "invalid quality"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
            .args(["encode", "--src", source, "--format", format, option, value])
            .output()
            .unwrap();

        assert!(!output.status.success(), "{format} {option} {value}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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
    let svg = qr::create_pay_svg(&payload).unwrap();
    let png = qr::render_png(&svg, 1_024).unwrap();
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
    let svg = qr::create_invoice_svg(&payload).unwrap();
    let png = qr::render_png(&svg, 1_024).unwrap();
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
    let svg = qr::create_pay_svg(&payload).unwrap();
    let png = qr::render_png(&svg, 512).unwrap();
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
fn decodes_payload_from_stdin_like_inline_source() {
    let fixture = fixture();
    let inline_output = Command::new(env!("CARGO_BIN_EXE_bysqrcli"))
        .args(["decode", "--src", &fixture.payload, "--format", "json"])
        .output()
        .unwrap();
    let stdin_output = run_with_stdin(
        &["decode", "--src", "-", "--format", "json"],
        fixture.payload.as_bytes(),
    );

    assert!(
        inline_output.status.success(),
        "{}",
        String::from_utf8_lossy(&inline_output.stderr)
    );
    assert!(
        stdin_output.status.success(),
        "{}",
        String::from_utf8_lossy(&stdin_output.stderr)
    );
    assert_eq!(stdin_output.stdout, inline_output.stdout);
    let decoded: Pay = serde_json::from_slice(&stdin_output.stdout).unwrap();
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

#[test]
fn encode_items_accepts_json_xml_path_and_stdin_and_prints_svg_arrays() {
    let items = items_list("INV-BATCH", 5);
    let json = inline_json(&items);
    let xml = items.to_xml_string().unwrap();

    let inline_output = run(&["encode-items", "--src", &json, "--format", "svg"]);
    let stdin_output = run_with_stdin(
        &["encode-items", "--src", "-", "--format", "svg"],
        xml.as_bytes(),
    );

    let source_path = temp_path("source.json");
    std::fs::write(&source_path, &json).unwrap();
    let path_output = run(&[
        "encode-items",
        "--src",
        source_path.to_str().unwrap(),
        "--format",
        "svg",
    ]);
    std::fs::remove_file(source_path).unwrap();

    let inline_svgs = parse_string_array(&inline_output);
    let stdin_svgs = parse_string_array(&stdin_output);
    let path_svgs = parse_string_array(&path_output);
    assert_eq!(inline_svgs.len(), 2);
    assert_eq!(stdin_svgs.len(), inline_svgs.len());
    assert_eq!(path_svgs.len(), inline_svgs.len());
    for ((inline, stdin), path) in inline_svgs.iter().zip(&stdin_svgs).zip(&path_svgs) {
        assert_eq!(
            xmltree::Element::parse(stdin.as_bytes()).unwrap(),
            xmltree::Element::parse(inline.as_bytes()).unwrap()
        );
        assert_eq!(
            xmltree::Element::parse(path.as_bytes()).unwrap(),
            xmltree::Element::parse(inline.as_bytes()).unwrap()
        );
    }
    for svg in inline_svgs {
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("M104.382 0C106.433"));
        xmltree::Element::parse(svg.as_bytes()).unwrap();
    }
}

#[test]
fn encode_items_prints_png_and_jpeg_data_url_arrays_with_default_size() {
    let source = inline_json(&items_list("INV-RASTER", 1));
    for (format, prefix, signature) in [
        ("png", "data:image/png;base64,", b"\x89PNG".as_slice()),
        (
            "jpeg",
            "data:image/jpeg;base64,",
            b"\xff\xd8\xff".as_slice(),
        ),
        ("jpg", "data:image/jpeg;base64,", b"\xff\xd8\xff".as_slice()),
    ] {
        let output = run(&["encode-items", "--src", &source, "--format", format]);
        let entries = parse_string_array(&output);
        assert_eq!(entries.len(), 1, "{format}");
        let encoded = entries[0].strip_prefix(prefix).unwrap_or_else(|| {
            panic!(
                "{format} output did not use the expected data URL: {}",
                entries[0]
            )
        });
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert!(bytes.starts_with(signature), "{format}");
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!(image.width(), 512, "{format}");
        assert_eq!(image.height(), 600, "{format}");
    }
}

#[test]
fn encode_items_saves_deterministic_files_at_chunk_boundaries() {
    for (count, format, extension, signature) in [
        (1, "svg", "svg", b"<svg".as_slice()),
        (5, "png", "png", b"\x89PNG".as_slice()),
        (9, "jpeg", "jpeg", b"\xff\xd8\xff".as_slice()),
    ] {
        let source = inline_json(&items_list("INV-FILES", count));
        let directory = temp_path(&format!("{count}-{format}"));
        let output = run(&[
            "encode-items",
            "--src",
            &source,
            "--format",
            format,
            "--size",
            "96",
            "--save",
            directory.to_str().unwrap(),
        ]);
        assert_success(&output);
        assert!(output.stdout.is_empty());

        let expected_count = count.div_ceil(4);
        let mut names = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        names.sort();
        let expected_names = (1..=expected_count)
            .map(|index| format!("invoice-items-{index:03}.{extension}"))
            .collect::<Vec<_>>();
        assert_eq!(names, expected_names, "{count} lines");
        for name in names {
            assert!(
                std::fs::read(directory.join(name))
                    .unwrap()
                    .starts_with(signature),
                "{count} lines"
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn encode_items_preflights_collisions_and_overwrites_only_when_requested() {
    let source = inline_json(&items_list("INV-COLLISION", 5));
    let directory = temp_path("collision");
    std::fs::create_dir_all(&directory).unwrap();
    let first = directory.join("invoice-items-001.svg");
    let second = directory.join("invoice-items-002.svg");
    std::fs::write(&second, b"sentinel").unwrap();

    let output = run(&[
        "encode-items",
        "--src",
        &source,
        "--format",
        "svg",
        "--save",
        directory.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert!(
        !first.exists(),
        "wrote an earlier file before collision check"
    );
    assert_eq!(std::fs::read(&second).unwrap(), b"sentinel");

    let overwrite = run(&[
        "encode-items",
        "--src",
        &source,
        "--format",
        "svg",
        "--save",
        directory.to_str().unwrap(),
        "--overwrite",
    ]);
    assert_success(&overwrite);
    assert!(std::fs::read(&first).unwrap().starts_with(b"<svg"));
    assert!(std::fs::read(&second).unwrap().starts_with(b"<svg"));
    std::fs::remove_dir_all(&directory).unwrap();

    let file_destination = temp_path("not-a-directory");
    std::fs::write(&file_destination, b"sentinel").unwrap();
    let output = run(&[
        "encode-items",
        "--src",
        &source,
        "--format",
        "svg",
        "--save",
        file_destination.to_str().unwrap(),
        "--overwrite",
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("must be a directory"));
    assert_eq!(std::fs::read(&file_destination).unwrap(), b"sentinel");
    std::fs::remove_file(file_destination).unwrap();
}

#[test]
fn encode_items_manages_stale_same_format_outputs_as_one_batch() {
    let source = inline_json(&items_list("INV-STALE", 5));
    let directory = temp_path("stale");
    std::fs::create_dir_all(&directory).unwrap();
    let first = directory.join("invoice-items-001.svg");
    let second = directory.join("invoice-items-002.svg");
    let stale = directory.join("invoice-items-003.svg");
    let other_format = directory.join("invoice-items-003.png");
    let unrelated = directory.join("invoice-items-not-generated.svg");
    std::fs::write(&stale, b"stale svg").unwrap();
    std::fs::write(&other_format, b"unrelated png").unwrap();
    std::fs::write(&unrelated, b"unrelated svg").unwrap();

    let output = run(&[
        "encode-items",
        "--src",
        &source,
        "--format",
        "svg",
        "--save",
        directory.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert!(!first.exists());
    assert!(!second.exists());
    assert_eq!(std::fs::read(&stale).unwrap(), b"stale svg");
    assert_eq!(std::fs::read(&other_format).unwrap(), b"unrelated png");
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"unrelated svg");

    let overwrite = run(&[
        "encode-items",
        "--src",
        &source,
        "--format",
        "svg",
        "--save",
        directory.to_str().unwrap(),
        "--overwrite",
    ]);
    assert_success(&overwrite);
    assert!(std::fs::read(&first).unwrap().starts_with(b"<svg"));
    assert!(std::fs::read(&second).unwrap().starts_with(b"<svg"));
    assert!(!stale.exists());
    assert_eq!(std::fs::read(&other_format).unwrap(), b"unrelated png");
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"unrelated svg");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn decode_items_reassembles_out_of_order_payloads_to_json_and_xml() {
    let items = items_list("INV-DECODE", 9);
    let mut payloads = items.encode_chunks().unwrap();
    payloads.reverse();
    let source = inline_json(&payloads);

    let json_output = run(&["decode-items", "--src", &source, "--format", "json"]);
    assert_success(&json_output);
    let decoded: InvoiceItemsList = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(decoded, items);

    let xml_output = run_with_stdin(
        &["decode-items", "--src", "-", "--format", "xml"],
        source.as_bytes(),
    );
    assert_success(&xml_output);
    let xml = String::from_utf8(xml_output.stdout).unwrap();
    assert!(xml.starts_with("<InvoiceItemsList>"));
    assert!(!xml.contains("FirstInvoiceLineID"));
    assert_eq!(InvoiceItemsList::from_xml_str(xml.trim()).unwrap(), items);

    let path = temp_path("payloads.json");
    std::fs::write(&path, &source).unwrap();
    let path_output = run(&[
        "decode-items",
        "--src",
        path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    std::fs::remove_file(path).unwrap();
    assert_success(&path_output);
    assert_eq!(
        serde_json::from_slice::<InvoiceItemsList>(&path_output.stdout).unwrap(),
        items
    );
}

#[test]
fn decode_items_rejects_malformed_empty_mixed_and_gapped_inputs_without_output() {
    let first = items_list("INV-A", 5).encode_chunks().unwrap();
    let second = items_list("INV-B", 5).encode_chunks().unwrap();
    let mixed = inline_json(&vec![first[0].clone(), second[1].clone()]);

    let mut gap_payloads = items_list("INV-GAP", 9).encode_chunks().unwrap();
    gap_payloads.remove(1);
    let gap = inline_json(&gap_payloads);

    for (label, source, expected) in [
        ("malformed JSON", "{".to_owned(), "payload array"),
        ("empty array", "[]".to_owned(), "must not be empty"),
        ("non-text entry", "[1]".to_owned(), "payload array"),
        (
            "malformed payload",
            "[\"not-a-payload\"]".to_owned(),
            "payload",
        ),
        ("mixed invoices", mixed, "InvoiceID"),
        ("gapped blocks", gap, "FirstInvoiceLineID"),
    ] {
        let output = run(&["decode-items", "--src", &source]);
        assert!(!output.status.success(), "accepted {label}");
        assert!(
            output.stdout.is_empty(),
            "printed partial output for {label}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "unexpected {label} error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn batch_commands_validate_against_a_specific_parent_invoice() {
    let parent_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/invoice/valid-interoperability-offline-multiple-lines.json"
    );
    let parent = invoice::try_deserialize_invoice(include_str!(
        "fixtures/invoice/valid-interoperability-offline-multiple-lines.json"
    ))
    .unwrap();
    let parent_xml = parent.to_xml_string().unwrap();
    let matching = items_list("INV-MULTI-2026", 3);
    let matching_source = inline_json(&matching);

    let encode = run(&[
        "encode-items",
        "--src",
        &matching_source,
        "--format",
        "svg",
        "--invoice-src",
        parent_path,
    ]);
    assert_eq!(parse_string_array(&encode).len(), 1);

    let payload_source = inline_json(&matching.encode_chunks().unwrap());
    let decode = run(&[
        "decode-items",
        "--src",
        &payload_source,
        "--format",
        "json",
        "--invoice-src",
        &parent_xml,
    ]);
    assert_success(&decode);

    let wrong_id = inline_json(&items_list("OTHER", 3));
    let encode_mismatch = run(&[
        "encode-items",
        "--src",
        &wrong_id,
        "--format",
        "svg",
        "--invoice-src",
        parent_path,
    ]);
    assert!(!encode_mismatch.status.success());
    assert!(encode_mismatch.stdout.is_empty());
    assert!(String::from_utf8_lossy(&encode_mismatch.stderr).contains("InvoiceID"));

    let wrong_count = items_list("INV-MULTI-2026", 2).encode_chunks().unwrap();
    let wrong_count_source = inline_json(&wrong_count);
    let decode_mismatch = run(&[
        "decode-items",
        "--src",
        &wrong_count_source,
        "--invoice-src",
        parent_path,
    ]);
    assert!(!decode_mismatch.status.success());
    assert!(decode_mismatch.stdout.is_empty());
    assert!(String::from_utf8_lossy(&decode_mismatch.stderr).contains("InvoiceLines"));

    let pay_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pay/json/standing-order.json"
    );
    let wrong_parent_type = run(&[
        "encode-items",
        "--src",
        &matching_source,
        "--format",
        "svg",
        "--invoice-src",
        pay_path,
    ]);
    assert!(!wrong_parent_type.status.success());
    assert!(
        String::from_utf8_lossy(&wrong_parent_type.stderr).contains("deserialize parent Invoice")
    );

    for command in ["encode-items", "decode-items"] {
        let source = if command == "encode-items" {
            matching_source.as_str()
        } else {
            payload_source.as_str()
        };
        let mut args = vec![command, "--src", source, "--invoice-src", "-"];
        if command == "encode-items" {
            args.extend(["--format", "svg"]);
        }
        let output = run(&args);
        assert!(!output.status.success(), "{command}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("not supported"));
    }
}
