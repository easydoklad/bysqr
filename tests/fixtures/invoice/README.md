# Valid offline Invoice interoperability fixtures

These source-neutral fixtures contain valid encoded Invoice payloads and either
their exact decoded tab-separated sequence or their canonical semantic JSON.
They have no network or QR image dependency at test time.

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

- `valid-interoperability-offline-multiple-lines.*` covers three invoice lines,
  two VAT summaries, a claimed deposit, optional party/contact fields, rounding
  and the `mutualOffset` payment classifier.
- `valid-interoperability-offline-single-line.*` covers the compact embedded
  single-line representation with an EAN, decimal quantity, 23% VAT, negative
  rounding and the `cashOnDelivery` payment classifier.

The semantic fixtures compare the decoded model with canonical JSON, verify the
exact header and field count, and then exercise a semantic encoder/decoder
round trip. Their payload strings remain fixed interoperability inputs.
