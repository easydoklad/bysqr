# PAY by square conformance fixtures

The `valid-*.json` files are known-good PAY by square conformance vectors. They
use synthetic data only.

Each fixture stores a source document, a valid Base32hex payload, and the
expected uncompressed tab-delimited sequence. Tests are fully offline and do not
depend on PNG rendering. LZMA encoders can produce different valid byte streams,
so conformance is asserted after decoding the envelope, CRC32, and sequence
rather than by comparing compressed payload strings.

The `xsd-*.json` files cover combinations and boundaries derived from
`spec/bysquare.xsd`. Their expected sequences follow the ordering rules from the
by-square specification, including the special bulk-payment beneficiary
ordering.
