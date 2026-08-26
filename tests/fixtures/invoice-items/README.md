# Valid offline INVOICE ITEMS interoperability fixtures

These source-neutral fixtures contain fixed, independently valid INVOICE ITEMS
payloads and their canonical semantic JSON. They have no network or QR image
dependency at test time.

- `valid-interoperability-offline-mixed-lines.*` covers a named item, an EAN
  item, order and delivery-note references, multiple VAT rates, and a deposit
  represented by negative transported amounts.
- `valid-interoperability-offline-multi-qr-{1,9}.*` is one nine-line logical
  list transported in two deployed blocks. It verifies the shared `InvoiceID`,
  `FirstInvoiceLineID` ordering, decoding of an eight-line block, and lossless
  reassembly. The library's own convenience chunker deliberately follows the
  more conservative four-lines-per-QR recommendation.

Every payload is decoded through the full Base32hex, header, raw LZMA, CRC32,
UTF-8, and TSV pipeline. Encoder assertions are semantic because independent
LZMA implementations do not have to emit byte-identical compressed streams.
