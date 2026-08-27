# bysqr

Open source PAY by square, INVOICE by square and INVOICE ITEMS by square
encoder/decoder written in Rust.

## Notice

Version 0.1 provides complete PAY, INVOICE and INVOICE ITEMS encoder/decoder
workflows. The project remains pre-1.0: public APIs may still change as more
independent interoperability feedback becomes available.

The project provides encoder and decoder implementations without relying on
external services and is designed to compile for multiple native targets and
WebAssembly.

## Installation

You can download the `bysqr` CLI application from the
[Releases](https://github.com/easydoklad/bysqr/releases) page.
There are precompiled binaries for macOS, Linux and Windows, for x86 and ARM architectures. 
You can find there also a wasm build if you are interested.

All binaries are compiled in two versions - the full version and headless version. The headless version does not have
GUI and is meant to be run only from command line or shipped with your application. Hence, the headless version does not have
any GUI related features, such as QR code preview. Its size is however much smaller than the full version.

## Usage

You can use the `bysqr` binary to encode and decode PAY by square, INVOICE by
square and individual INVOICE ITEMS by square blocks. PAY payment orders,
standing orders and direct debits are supported, together with Invoice,
Proforma Invoice, Credit Note, Debit Note and Advance Invoice documents.

### Encoding to QR code

To encode a PAY, INVOICE or one INVOICE ITEMS block to a QR code, run `encode`
with the source document:

```shell
bysqr encode --src payment.xml --save ~/Desktop/qr.svg
bysqr encode --src invoice.json --save ~/Desktop/invoice.svg
bysqr encode --src invoice-items.json --save ~/Desktop/items.svg

bysqr encode --src '<?xml version="1.0"?><Pay type="Pay">...</Pay>' --save ~/Desktop/qr.svg
```

Provided source (`--src`) may be a canonical PAY, INVOICE or INVOICE ITEMS
XML/JSON document. You can pass either a file path or the document itself. The
document root, `DocumentType`, or Items marker fields select the correct
encoder and QR branding automatically.

#### JSON input

The canonical JSON structure mirrors `spec/bysquare.xsd`: element names remain
in PascalCase and XML collections remain explicit objects, such as
`Payments.Payment` and `BankAccounts.BankAccount`.

```json
{
  "Payments": {
    "Payment": [
      {
        "PaymentOptions": "paymentorder",
        "Amount": "12.34",
        "CurrencyCode": "EUR",
        "BankAccounts": {
          "BankAccount": [
            { "IBAN": "SK7700000000000000000000" }
          ]
        }
      }
    ]
  }
}
```

```shell
bysqr encode --src payment.json --save ~/Desktop/qr.svg
```

The [PAY JSON Schema](spec/pay-by-square.schema.json) defines this format using
JSON Schema Draft 2020-12, including descriptions and constraints derived from
the PAY part of the XML schema. Canonical amounts are strings so exact decimal
precision is preserved. Numeric JSON amounts are accepted as an input
convenience, but schema-conformant documents use strings.

The [INVOICE JSON Schema](spec/invoice-by-square.schema.json) follows the same
rules. It requires `DocumentType`, whose value is one of `Invoice`,
`ProformaInvoice`, `CreditNote`, `DebitNote` or `AdvanceInvoice`. XML carries
the same value in the `Invoice` root's `xsi:type` attribute. Exact decimal
values, including VAT rates, are represented as strings; the canonical VAT
range is `0` through `1`, so 20% is written as `"0.2"`.

The [INVOICE ITEMS JSON Schema](spec/invoice-items-by-square.schema.json)
describes the separate type-2 blocks used by multi-line invoices. Each block
contains the parent `InvoiceID`, its `FirstInvoiceLineID`, and an explicit
`InvoiceLines.InvoiceLine` array. A line uses exactly one of `ItemName` and
`ItemEANCode`; an optional billing period must contain both dates.

Computed XSD properties are optional read-only values in the JSON Schema and
are not transported in the QR sequence. Use `Invoice::calculate_totals` and
`invoice_items::InvoiceLine::calculate` when computed values are needed.
`bsqr:maxLength` annotations are exposed as advisory diagnostics and never
cause silent truncation; hard XSD constraints and the applicable QR transport
limits are enforced.

To save generated QR code as image, use `--save` option with path where to save the image. Type of the file is
determined by the output file extension. We support generating `svg`, `png` and `jpeg` images.

#### PAY and INVOICE visual themes

PAY and INVOICE QR output support every composition documented by the by-square
logo manual: print or electronic layout, branding at the bottom, top, left or
right, and light, dark, gray or black color variations. Light and dark map to
the approved family palette: blue for PAY and orange for INVOICE. Both families
default to the dark print layout with bottom branding.

```shell
bysqr encode --src invoice.json --save invoice.svg \
  --logo-layout electronic \
  --logo-position left \
  --logo-color gray
```

These are constrained presets, rather than arbitrary styling controls. The QR
matrix remains black on white, and custom colors or altered logo proportions
are intentionally not presented as logo-manual-compliant output. INVOICE ITEMS
intentionally has no theme options and uses the generator-compatible black
composition.

#### QR code preview

You may also preview generated code instead of saving, by passing a `--preview` option. This will open a window
where the QR code is displayed.

```shell
bysqr encode --src payment.xml --preview
```

This feature is not available in headless version.

#### Output to stdout

If you want to output content of the image directly to the standard output, you may use `--format` option instead of `--save`.
This will print content of the image to the stdout in requested format. If you specify an `svg` format, the XML of the SVG will be printed out.
Other formats such as `png` and `jpeg` are printed out as base64 encoded strings.

```shell
bysqr encode --src payment.xml --format svg # output: <svg xmlns="http://www.w3.org/2000/svg">...</svg>
bysqr encode --src payment.xml --format png # output: data:image/png;base64,...
bysqr encode --src payment.xml --format jpeg # output: data:image/jpeg;base64,...
```

#### Image size

When you request `png` or `jpeg` format, you may use the `--size` option to control the size of the output image. The size
option controls the width of the generated image. Height of the image is automatically calculated, since QR code with required logo outline
is a rectangle. The `svg` format ignores the size setting.

```shell
# This will create a png image with 1024px width
bysqr encode --src payment.xml --format png --size 1024
```

#### Image quality

When saving to a `jpeg` format, you may configure image encoder quality using `--quality` option. It must be a number from **1** to **100**.
The default quality is set to **90**.

```shell
bysqr encode --src payment.xml --format jpeg --quality 95
```

### Decoding a payload

The decoder accepts the Base32hex content carried by the QR code, classifies its
header and prints a PAY, INVOICE or INVOICE ITEMS document as canonical JSON or
XML. JSON output conforms to the corresponding schema in `spec/`. The decoder
evaluates the text payload directly; optional raster image scanning is
described below.

```shell
bysqr decode --src '000620000...' --format json
bysqr decode --src payload.txt --format xml
```

### Decoding a QR image

Raster image reading is optional so applications that already use their own QR
scanner do not need to compile another one. Enable it with the `qr-reader`
feature:

```shell
cargo build --release --features qr-reader
bysqr decode --src payment.png --format json
bysqr decode --src invoice.jpg --format xml
```

The lower-level `qr_reader::extract_payloads_from_bytes` API returns the text
from every detected QR code. `qr_reader::decode_document_from_bytes` selects
and validates exactly one supported by-square document;
`qr_reader::decode_pay_from_bytes` remains available for PAY-only consumers.
`qr_reader::decode_invoice_items_from_bytes` reassembles every compatible Items
block found in one image. PNG and JPEG decoding, including rendered INVOICE and
ITEMS QR codes, are covered by the end-to-end test suite.

## Build

To build a project, ensure you have latest [Rust](https://www.rust-lang.org/tools/install) installed. Then, run build using `cargo`:

```shell
cargo build --release
```

You can find `bysqrcli` executable and rust library in `target/release`.

## Rust API

Both deserialization and encoding return typed errors:

```rust
use bysqr::pay;

let pay = pay::try_deserialize_pay(include_str!("payment.xml"))?;
let payload = pay::encode(&pay)?;
let decoded = pay::decode(&payload)?;
assert_eq!(decoded, pay);
# Ok::<(), bysqr::error::Error>(())
```

INVOICE uses the parallel domain API:

```rust
use bysqr::{invoice, Document};

let invoice = invoice::try_deserialize_invoice(include_str!("invoice.json"))?;
let payload = invoice::encode(&invoice)?;
let decoded = invoice::decode(&payload)?;
assert_eq!(decoded, invoice);

assert!(matches!(bysqr::decode(&payload)?, Document::Invoice(_)));
# Ok::<(), bysqr::error::Error>(())
```

Logo-manual-compliant PAY and INVOICE rendering uses the same closed semantic
theme type. The renderer selects the approved family-specific palette:

```rust
use bysqr::qr::{
    create_invoice_svg_with_theme, create_pay_svg_with_theme, LogoColor,
    LogoLayout, LogoPosition, LogoTheme,
};

let theme = LogoTheme::new(
    LogoLayout::Electronic,
    LogoPosition::Right,
    LogoColor::Black,
);
let pay_svg = create_pay_svg_with_theme(&pay_payload, theme)?;
let invoice_svg = create_invoice_svg_with_theme(&invoice_payload, theme)?;
# Ok::<(), bysqr::error::Error>(())
```

`create_pay_svg`, `create_invoice_svg` and `create_invoice_items_svg` use their
approved default compositions. SVG creation and the PNG/JPEG raster helpers
return `bysqr::error::Result` instead of panicking on invalid input. Raster
dimensions are limited to 8,192 pixels per side; JPEG quality must be 1–100.

`LogoLayout::ALL`, `LogoPosition::ALL` and `LogoColor::ALL` expose the full
2 × 4 × 4 preset matrix. A deterministic visual gallery can be generated with:

```shell
cargo run --bin theme-preview
```

The resulting `target/theme-preview.html` compares PAY and INVOICE across all
32 variants. No public API accepts arbitrary colors; INVOICE ITEMS remains
fixed to its black composition.

INVOICE ITEMS exposes both individual-block and complete-list APIs. The
convenience encoder follows the specification's conservative recommendation of
four lines per QR; the decoder also accepts larger deployed blocks:

```rust
use bysqr::invoice_items;

let source = include_str!("invoice-items.json");
let block = invoice_items::try_deserialize_invoice_items(source)?;
let lines = block.invoice_lines.invoice_line.clone();

let payloads = invoice_items::encode_chunks(block.invoice_id.clone(), lines.clone())?;
let reassembled = invoice_items::decode_chunks(&payloads)?;
assert_eq!(reassembled.invoice_id, block.invoice_id);
assert_eq!(reassembled.invoice_lines, lines);
# Ok::<(), bysqr::error::Error>(())
```

When the parent `invoice::Invoice` is available, call
`reassembled.validate_against_invoice(&invoice)` as well. It detects a mismatched
`InvoiceID` and a missing final block by comparing the reassembled length with
the parent's `NumberOfInvoiceLines`.

`pay::encode_sequence` exposes the uncompressed tab-delimited form for
conformance tooling. `codec::decode_payload` validates and inspects an encoded
envelope, including its header, LZMA data, declared two-byte size, CRC32, and
UTF-8 sequence. `pay::decode` validates and reconstructs the complete PAY
model, while `pay::decode_sequence` can inspect an already uncompressed
sequence. The same schema used by the fixture suite is embedded in the library
as `bysqr::pay::JSON_SCHEMA` for consumers that want to validate JSON before
encoding it. INVOICE provides the corresponding `invoice::encode_sequence`,
`invoice::decode_sequence` and `invoice::JSON_SCHEMA` APIs. INVOICE ITEMS
provides `invoice_items::encode_sequence`, `invoice_items::decode_sequence`,
`invoice_items::JSON_SCHEMA`, plus chunking and strict reassembly helpers.

The PAY and INVOICE domain `encode` and `encode_sequence` functions enforce the
550-character QR limit. For non-QR transport use the domain's
`encode_with_limit` function and `SequenceLimit::Unbounded`. INVOICE ITEMS
avoids a global sequence cut-off: its high-level encoder chunks by the
specification's recommended line count, while explicit single-block encoding
remains interoperable with larger deployed blocks. No mode silently drops
fields; the protocol-level 16-bit payload limit always applies.

With the `qr-reader` feature enabled, Rust consumers can pass raster bytes to
`qr_reader::decode_document_from_bytes`. Without it they can feed text from any
external scanner directly to `bysqr::decode`. The crate-level function
classifies the payload and returns a `bysqr::Document`. Likewise,
`bysqr::try_deserialize` classifies canonical JSON/XML source data.

## Tests

Run the complete suite with:

```shell
cargo test --all-features
```

Valid offline fixtures under `tests/fixtures/pay`, `tests/fixtures/invoice` and
`tests/fixtures/invoice-items` cover known payloads and XSD-derived cases. The
Items fixtures include named/EAN lines, references, a negative deposit, and a
deployed two-QR sequence. Tests compare decoded data instead of compressed
strings because different LZMA streams can represent the same valid payload.
The suite has no network dependency and also verifies that its own branded PAY,
INVOICE and ITEMS raster output can be scanned back into typed data.

### WASM build

`bysqr` can be built for Web Assembly target, which allows you to run encoder and decoder in the browser, without need for a server.

Before building for `wasm` target, you need to install `wasm-pack`.

```shell
cargo install wasm-pack
```

After installing, you can start build:

```shell
wasm-pack build --target web --features wasm
```

Built wasm module will be located in `pkg` folder.

To include raster QR reading, enable both features:

```shell
wasm-pack build --target web --features wasm,qr-reader
```

This additionally exports `decode_image_to_json` and `decode_image_to_xml`,
which accept PNG or JPEG bytes and classify PAY, INVOICE or INVOICE ITEMS
documents. The text-only encoding and decoding exports support all three
families without the QR reader.

#### Building for wasm on Ubuntu

Before building wasm on Ubuntu, make sure to install all necessary tools:

```shell
sudo apt install -y build-essential clang
```

#### Building for wasm on macOS

Apple's system clang does not provide the `wasm32-unknown-unknown` target needed
to compile the bundled LZMA C library. Install Homebrew LLVM and make sure its
clang appears before `/usr/bin/clang` when running `wasm-pack`.

```shell
brew install llvm

# These can be placed in ~/.zshrc.
export PATH="$(brew --prefix llvm)/bin:$PATH"
export LDFLAGS="-L$(brew --prefix llvm)/lib"
export CPPFLAGS="-I$(brew --prefix llvm)/include"

# Must report Homebrew clang, not Apple clang from /usr/bin.
which clang
clang --version

wasm-pack build --target web --features wasm
```

## Roadmap to v1.0

- [x] PAY payment-order encoder
- [x] PAY standing-order encoder
- [x] PAY direct-debit encoder
- [x] PAY decoder
- [x] PAY JSON input/output and JSON Schema
- [x] PAY QR and unbounded sequence-length policies
- [x] optional generic QR image reader
- [x] INVOICE model, encoder and decoder for all five document types
- [x] INVOICE JSON/XML and JSON Schema
- [x] INVOICE QR branding and raster scan verification
- [x] INVOICE ITEMS model, encoder, decoder and multi-QR reassembly
- [x] additional INVOICE and INVOICE ITEMS interoperability fixtures
- [x] approved PAY and INVOICE themes
- [x] approved logo positions and print/electronic layouts
- [x] fallible public QR and raster API
- [x] PAY encoder conformance tests
- [x] INVOICE encoder/decoder conformance tests
- [ ] stabilize the public API based on 0.x integration feedback
- [ ] add browser and npm integration
