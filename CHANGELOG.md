# Changelog

All notable changes to this project are documented in this file.

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
