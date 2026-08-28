# bysqr

Open source PAY by square, INVOICE by square and INVOICE ITEMS by square
encoder/decoder written in Rust.

## Status

`bysqr` is a pre-1.0 project. Public Rust and WebAssembly APIs may change before
1.0. Encoding and decoding run locally on native targets and WebAssembly; no
external service is required.

## Installation

Add the Rust library to an application with:

```shell
cargo add bysqr
```

The optional `qr-reader` feature adds QR extraction from PNG and JPEG images:

```shell
cargo add bysqr --features qr-reader
```

Install the headless CLI directly from crates.io with:

```shell
cargo install bysqr
```

Download the `bysqr` CLI from the
[Releases](https://github.com/easydoklad/bysqr/releases) page. Prebuilt native
binaries are available for macOS, Linux and Windows on x86_64 and AArch64.

Each native target has a standard build with the QR preview window and a
smaller headless build for command-line and embedded use. Releases also include
a WebAssembly package.

## Usage

The CLI encodes and decodes PAY by square, INVOICE by square and INVOICE ITEMS
by square. It supports PAY payment orders, standing orders and direct debits,
and all five INVOICE document types.

### Encoding to QR code

To encode a PAY, INVOICE or one INVOICE ITEMS block to a QR code, run `encode`
with the source document:

```shell
bysqr encode --src payment.xml --save ~/Desktop/qr.svg
bysqr encode --src invoice.json --save ~/Desktop/invoice.svg
bysqr encode --src invoice-items.json --save ~/Desktop/items.svg

bysqr encode --src '<?xml version="1.0"?><Pay type="Pay">...</Pay>' --save ~/Desktop/qr.svg
```

`--src` accepts a PAY, INVOICE or INVOICE ITEMS JSON/XML document, either inline
or as a file path. The document data selects the encoder and QR branding.

Pass `--src -` to read textual source data from standard input. This transport
is available for `encode`, `decode`, `encode-items` and `decode-items`.

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

The JSON Schema Draft 2020-12 files define each format:

- [PAY](spec/pay-by-square.schema.json)
- [INVOICE](spec/invoice-by-square.schema.json), including all five
  `DocumentType` values
- [one INVOICE ITEMS QR block](spec/invoice-items-by-square.schema.json)
- [a complete ordered InvoiceItemsList](spec/invoice-items-list.schema.json)

Schema-conformant decimal values are strings to preserve exact precision;
numeric JSON input is also accepted. VAT rates use the range `0` through `1`,
so 20% is `"0.2"`. An `InvoiceItemsList` has no `FirstInvoiceLineID` because
chunking assigns that value to each encoded QR block.

Computed XSD properties are optional read-only values in the JSON Schema and
are not transported in the QR sequence. Use `Invoice::calculate_totals` and
`invoice_items::InvoiceLine::calculate` when computed values are needed.
`bsqr:maxLength` annotations are exposed as advisory diagnostics and never
cause silent truncation; hard XSD constraints and the applicable QR transport
limits are enforced.

Use `--save` to write an image. The file extension selects SVG, PNG or JPEG.

#### PAY and INVOICE visual themes

PAY and INVOICE support the fixed presets defined by the by-square logo manual:
print or electronic layout, branding at the bottom, top, left or right, and
light, dark, gray or black color variations. Light and dark use blue for PAY and
orange for INVOICE. Both families default to the dark print layout with bottom
branding.

```shell
bysqr encode --src invoice.json --save invoice.svg \
  --logo-layout electronic \
  --logo-position left \
  --logo-color gray
```

Custom colors and logo proportions are not supported. The QR matrix remains
black on white, and INVOICE ITEMS uses its fixed black composition.

#### QR code preview

Use `--preview` to display the generated QR code in a window instead of saving
it.

```shell
bysqr encode --src payment.xml --preview
```

This option is not available in headless builds.

#### Output to stdout

Use `--format` instead of `--save` to write the image to standard output. SVG
is emitted as XML; PNG and JPEG are emitted as Base64 data URLs.

```shell
bysqr encode --src payment.xml --format svg # output: <svg xmlns="http://www.w3.org/2000/svg">...</svg>
bysqr encode --src payment.xml --format png # output: data:image/png;base64,...
bysqr encode --src payment.xml --format jpeg # output: data:image/jpeg;base64,...
```

#### Image size

For PNG and JPEG, `--size` sets the image width in pixels. Height is calculated
from the selected QR composition. SVG output ignores this option.

```shell
# Create a PNG image 1024 pixels wide.
bysqr encode --src payment.xml --format png --size 1024
```

#### Image quality

For JPEG, `--quality` accepts a value from 1 to 100 and defaults to 90.

```shell
bysqr encode --src payment.xml --format jpeg --quality 95
```

### Batch INVOICE ITEMS

`encode-items` accepts one complete `InvoiceItemsList` JSON/XML document and
splits it into the specification's recommended four-line QR blocks.

```shell
# stdout is a JSON array containing one SVG string per QR block
bysqr encode-items --src invoice-items-list.json --format svg

# write invoice-items-001.png, invoice-items-002.png, ...
bysqr encode-items --src invoice-items-list.json --format png --save items-qr

# stdin is useful for process wrappers
cat invoice-items-list.json | bysqr encode-items --src - --format jpeg
```

Without `--save`, SVG output is a JSON array of SVG strings and PNG/JPEG output
is a JSON array of Base64 data URLs. With `--save`, the destination is a
directory. Existing generated files are rejected unless `--overwrite` is
provided.

`decode-items` accepts a JSON array containing the textual contents scanned
from all related QR codes. Block order does not matter; gaps, overlaps and
mixed `InvoiceID` values are rejected. The result is one `InvoiceItemsList`
JSON/XML document.

```shell
bysqr decode-items --src scanned-payloads.json --format json
cat scanned-payloads.json | bysqr decode-items --src - --format xml
```

Both batch commands accept an optional `--invoice-src invoice.json` argument to
validate the aggregate `InvoiceID` and item count against the parent INVOICE.

### Decoding a payload

The decoder accepts Base32hex QR content and prints a PAY, INVOICE or INVOICE
ITEMS document as JSON or XML. JSON output conforms to the corresponding schema
in `spec/`.

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
block found in one image.

## Build

Install the latest [Rust](https://www.rust-lang.org/tools/install), then build
the project with Cargo:

```shell
cargo build --release
```

Cargo writes the `bysqr` executable and Rust library to `target/release`.

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

PAY and INVOICE rendering uses the same fixed `LogoTheme` presets. The renderer
selects the family-specific palette:

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
default compositions. SVG creation and the PNG/JPEG raster helpers return
`bysqr::error::Result`. Raster dimensions are limited to 8,192 pixels per side;
JPEG quality must be 1–100.

`LogoLayout::ALL`, `LogoPosition::ALL` and `LogoColor::ALL` expose the full
2 × 4 × 4 preset matrix. A deterministic visual gallery can be generated with:

```shell
cargo run --example theme_preview
```

The resulting `target/theme-preview.html` compares PAY and INVOICE across all
32 variants.

`InvoiceItemsList` represents a complete ordered item list. Chunking assigns
the block-local `FirstInvoiceLineID` automatically and uses the specification's
recommended four lines per QR. The decoder also accepts larger deployed blocks:

```rust
use bysqr::invoice_items::{self, InvoiceItemsList};

let list: InvoiceItemsList = serde_json::from_str(include_str!(
    "invoice-items-list.json"
))?;

let payloads = list.encode_chunks()?;
let reassembled = invoice_items::decode_chunks(&payloads)?;
assert_eq!(reassembled, list);
# Ok::<(), Box<dyn std::error::Error>>(())
```

When a parent `invoice::Invoice` is available,
`reassembled.validate_against_invoice(&invoice)` checks its `InvoiceID` and
`NumberOfInvoiceLines`.

Lower-level `encode_sequence`, `decode_sequence` and `codec::decode_payload`
APIs are available for conformance tooling. The embedded `JSON_SCHEMA`
constants expose the PAY, INVOICE and single-block INVOICE ITEMS schemas;
`invoice_items::JSON_SCHEMA_LIST` exposes the aggregate schema.

PAY and INVOICE `encode` and `encode_sequence` enforce the 550-character QR
limit. Non-QR integrations can use `encode_with_limit` with
`SequenceLimit::Unbounded`; the protocol-level 16-bit payload limit still
applies. The INVOICE ITEMS high-level encoder chunks the item list instead of
applying a global sequence limit.

With `qr-reader`, pass raster bytes to
`qr_reader::decode_document_from_bytes`. Otherwise, pass text from an external
scanner to `bysqr::decode`. `bysqr::try_deserialize` provides the corresponding
JSON/XML document classification.

## Tests

Run the complete suite with:

```shell
cargo test --all-features
```

The `Tests` GitHub Actions workflow runs formatting, Clippy, the complete Rust
suite, crate package verification, and the WASM/Node boundary suite on every
push and pull request.

Maintainer release instructions, including the initial crates.io publication
and subsequent Trusted Publishing workflow, are in
[`docs/releasing.md`](docs/releasing.md).

Offline fixtures cover known payloads, XSD-derived cases and multi-QR INVOICE
ITEMS. End-to-end tests also render and scan PAY, INVOICE and ITEMS images. They
compare decoded data rather than compressed strings because equivalent LZMA
streams need not be byte-identical.

### WASM build

The `wasm` feature runs the encoder, decoder and renderer in a browser without a
server.

The low-level API and its structured errors are documented in
[`docs/wasm.md`](docs/wasm.md).

Before building for `wasm` target, install the same pinned wasm-pack release used
by CI.

```shell
cargo install wasm-pack --version 0.15.0 --locked
```

Build the web package and run the complete Node boundary suite with one command:

```shell
./scripts/test-wasm.sh
```

The generated module is written to `pkg`.

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

- Stabilize the public API based on 0.x integration feedback.
- Complete browser and npm integration in the JavaScript SDK.
