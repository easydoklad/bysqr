# Valid offline Invoice interoperability fixtures

These source-neutral fixtures contain only an encoded Invoice payload and its
exact decoded tab-separated sequence. They do not depend on an Invoice XML or
JSON model, network access, QR image decoding, or production Invoice code.

The integration test passes each payload to `crate::codec::decode_payload`.
Successful decoding validates the Base32hex envelope, declared length, raw LZMA
stream, CRC32, and UTF-8 sequence. The test then asserts the exact `1/0/0/0`
header, all 45 TSV fields, and selected identifying fields.

- `valid-interoperability-offline-forsys-legacy.*` is the valid legacy Forsys
  interoperability vector. It contains the fractional VAT token `0.2` and
  retains its explicit zero-valued summary fields.
- `valid-interoperability-offline-official-current.*` is the valid current
  official PNG sample interoperability vector. Its deployed sequence contains
  the VAT token `20`. That discrepancy is retained solely as compatibility
  evidence; it does not define or alter canonical Invoice VAT behavior.

Each text fixture has one final line ending for repository readability. The
test removes only that line ending, preserving meaningful trailing tabs (the
official current sequence ends with an empty 45th field).
