# Changelog

All notable changes to this project are documented in this file.

## Unreleased

## 0.3.1 - 2026-08-28

### Changed

- Crates.io Trusted Publishing now runs for every release without a repository
  feature flag.

## 0.3.0 - 2026-08-28

### Added

- First public Rust library and CLI package for crates.io.
- Version-gated release validation and automated crates.io Trusted Publishing
  for releases after the initial manual publication.

### Changed

- Renamed the Cargo CLI binary from `bysqrcli` to `bysqr`.
- Moved the internal theme gallery generator from an installed binary to a
  Cargo example.
- Disabled unused image-format features to reduce the default dependency graph.
- GitHub release assets are now collected before creating one atomic release.

### Compatibility

- This remains a pre-1.0 release. Public Rust and WebAssembly APIs may change
  in subsequent 0.x versions.

## 0.2.1 - 2026-08-27

### Changed

- macOS release binaries are signed with Developer ID and notarized by Apple.

## 0.2.0 - 2026-08-27

### Added

- Application-level `InvoiceItemsList` model with canonical JSON/XML,
  standalone JSON Schema, chunk encoding and strict reassembly.
- Native CLI stdin transport through `--src -`.
- Batch `encode-items` and `decode-items` CLI workflows with optional parent
  INVOICE validation.
- Versioned WASM bridge contract, structured JavaScript errors and browser
  boundary tests.

### Changed

- WASM release archives now include an explicit manifest, schemas,
  documentation and checksums.

## 0.1.0 - 2026-08-27

First feature-complete encoder/decoder preview release.

### Added

- PAY payment order, standing order and direct debit encoding and decoding.
- INVOICE encoding and decoding for Invoice, Proforma Invoice, Credit Note,
  Debit Note and Advance Invoice documents.
- INVOICE ITEMS encoding, decoding, chunking and strict multi-QR reassembly.
- Canonical JSON and XML input/output with JSON Schemas for all three families.
- Text payload decoding plus an optional raster QR image reader.
- Logo-manual PAY and INVOICE themes and fixed INVOICE ITEMS branding.
- SVG, PNG and JPEG rendering, CLI workflows and WebAssembly exports.
- Valid offline interoperability fixtures and end-to-end QR scan tests.

### Changed

- QR and raster rendering APIs now return typed errors instead of panicking on
  caller-controlled input.
- PAY and INVOICE expose matching default and themed SVG entry points.
- Advisory XSD length annotations are reported without being treated as hard
  field limits.

### Compatibility

- This is a pre-1.0 release. Public Rust and WebAssembly APIs may change in
  subsequent 0.x versions.
