use bysqr::codec::{decode_payload, Header};

struct Fixture {
    name: &'static str,
    payload: &'static str,
    expected_sequence: &'static str,
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
