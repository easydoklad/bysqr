use bysqr::{
    codec::{decode_payload, Header},
    invoice, Document,
};

struct Fixture {
    name: &'static str,
    payload: &'static str,
    expected_sequence: &'static str,
}

struct SemanticFixture {
    name: &'static str,
    payload: &'static str,
    expected_json: &'static str,
    expected_field_count: usize,
}

const INVOICE_HEADER: Header = Header {
    by_square_type: 1,
    version: 0,
    document_type: 0,
    reserved: 0,
};

const FORSYS_LEGACY: Fixture = Fixture {
    name: "valid offline Forsys legacy Invoice interoperability vector",
    payload: include_str!(
        "fixtures/invoice/valid-interoperability-offline-forsys-legacy.payload.txt"
    ),
    expected_sequence: include_str!(
        "fixtures/invoice/valid-interoperability-offline-forsys-legacy.sequence.tsv"
    ),
};

const OFFICIAL_CURRENT: Fixture = Fixture {
    name: "valid offline official current Invoice interoperability vector",
    payload: include_str!(
        "fixtures/invoice/valid-interoperability-offline-official-current.payload.txt"
    ),
    expected_sequence: include_str!(
        "fixtures/invoice/valid-interoperability-offline-official-current.sequence.tsv"
    ),
};

const MULTIPLE_LINES: SemanticFixture = SemanticFixture {
    name: "valid offline multiple-line Invoice interoperability vector",
    payload: include_str!(
        "fixtures/invoice/valid-interoperability-offline-multiple-lines.payload.txt"
    ),
    expected_json: include_str!(
        "fixtures/invoice/valid-interoperability-offline-multiple-lines.json"
    ),
    expected_field_count: 50,
};

const SINGLE_LINE: SemanticFixture = SemanticFixture {
    name: "valid offline single-line Invoice interoperability vector",
    payload: include_str!(
        "fixtures/invoice/valid-interoperability-offline-single-line.payload.txt"
    ),
    expected_json: include_str!("fixtures/invoice/valid-interoperability-offline-single-line.json"),
    expected_field_count: 45,
};

fn without_terminal_line_ending(value: &str) -> &str {
    let value = value.strip_suffix('\n').unwrap_or(value);
    value.strip_suffix('\r').unwrap_or(value)
}

fn assert_valid_interoperability_fixture(fixture: &Fixture) -> Vec<String> {
    let payload = without_terminal_line_ending(fixture.payload);
    let expected_sequence = without_terminal_line_ending(fixture.expected_sequence);
    let decoded = decode_payload(payload).unwrap_or_else(|error| {
        panic!(
            "{} failed Base32hex/LZMA/CRC decoding: {error}",
            fixture.name
        )
    });

    assert_eq!(decoded.header, INVOICE_HEADER, "{}", fixture.name);
    assert_eq!(decoded.sequence, expected_sequence, "{}", fixture.name);

    let fields = decoded
        .sequence
        .split('\t')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 45, "{}", fixture.name);
    fields
}

fn assert_valid_semantic_fixture(fixture: &SemanticFixture) -> Vec<String> {
    let payload = without_terminal_line_ending(fixture.payload);
    let schema: serde_json::Value = serde_json::from_str(invoice::JSON_SCHEMA).unwrap();
    let expected_json: serde_json::Value = serde_json::from_str(fixture.expected_json).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let schema_errors = validator
        .iter_errors(&expected_json)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(
        schema_errors.is_empty(),
        "{} violates the Invoice JSON Schema: {schema_errors:#?}",
        fixture.name
    );

    let expected = invoice::try_deserialize_invoice(fixture.expected_json)
        .unwrap_or_else(|error| panic!("{} has invalid expected JSON: {error}", fixture.name));
    let decoded = invoice::decode(payload)
        .unwrap_or_else(|error| panic!("{} failed Invoice decoding: {error}", fixture.name));

    assert_eq!(decoded, expected, "{}", fixture.name);
    assert_eq!(
        bysqr::decode(payload).unwrap(),
        Document::Invoice(Box::new(expected.clone())),
        "{}",
        fixture.name
    );

    let envelope = decode_payload(payload).unwrap();
    assert_eq!(envelope.header, INVOICE_HEADER, "{}", fixture.name);
    let fields = envelope.sequence.split('\t').collect::<Vec<_>>();
    assert_eq!(
        fields.len(),
        fixture.expected_field_count,
        "{}",
        fixture.name
    );

    let encoded = invoice::encode(&expected).unwrap();
    assert_eq!(
        invoice::decode(&encoded).unwrap(),
        expected,
        "{}",
        fixture.name
    );
    fields.into_iter().map(str::to_owned).collect()
}

#[test]
fn valid_offline_forsys_legacy_invoice_interoperability_payload() {
    let fields = assert_valid_interoperability_fixture(&FORSYS_LEGACY);

    assert_eq!(fields[0], "201300001");
    assert_eq!(fields[1], "20130227");
    assert_eq!(fields[37], "0.2");
    assert_eq!(&fields[40..44], ["0", "0", "0", "0"]);
}

#[test]
fn valid_offline_official_current_invoice_interoperability_payload() {
    let fields = assert_valid_interoperability_fixture(&OFFICIAL_CURRENT);

    assert_eq!(fields[0], "INV-2025-0001");
    assert_eq!(fields[1], "20250115");
    // Compatibility evidence from this deployed sample; not canonical VAT behavior.
    assert_eq!(fields[37], "20");
}

#[test]
fn valid_offline_multiple_line_invoice_interoperability_payload() {
    let fields = assert_valid_semantic_fixture(&MULTIPLE_LINES);

    assert_eq!(fields[0], "INV-MULTI-2026");
    assert_eq!(fields[27], "3");
    assert_eq!(fields[36], "2");
    assert_eq!(fields[37], "0.1");
    assert_eq!(fields[42], "0.2");
    assert_eq!(fields[49], "32");
}

#[test]
fn valid_offline_single_line_invoice_interoperability_payload() {
    let fields = assert_valid_semantic_fixture(&SINGLE_LINE);

    assert_eq!(fields[0], "INV-SINGLE-2026");
    assert_eq!(fields[32], "8580000000001");
    assert_eq!(fields[35], "2.5");
    assert_eq!(fields[36], "1");
    assert_eq!(fields[44], "4");
}
