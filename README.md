# bysqr

Open source PAY by square encoder written in Rust.

## Notice

Work on the project is still in progress. It is not suitable for a production run, until version 1.0, since not
all features are implemented. Current version is very rough proof of concept. 
It is very likely there will be breaking changes until 1.0, before settling on some stable API.

The goal of the project is to provide full encoder and decoder implementations for PAY by square and Invoice by square,
without relying on external services and with enough portability to compile for various targets.

## Installation

You can download `bysqr` cli application from [Releases](https://github.com/bysqr/bysqr/releases) page.
There are precompiled binaries for macOS, Linux and Windows, for x86 and ARM architectures. 
You can find there also a wasm build if you are interested.

All binaries are compiled in two versions - the full version and headless version. The headless version does not have
GUI and is ment to be run only from command line or shipped with your application. Hence, the headless version does not have
any GUI related features, such as QR code preview. It's size is however much smaller than the full version.

## Usage

You can use the `bysqr` binary to encode and decode PAY by square data. Payment
orders, standing orders, and direct debits are supported.

### Encoding to QR code

To encode `Pay` to a QR code, you can run the `encode` command with the following arguments:

```shell
bysqr encode --src payment.xml --save ~/Desktop/qr.svg

bysqr encode --src '<?xml version="1.0"?><Pay type="Pay">...</Pay>' --save ~/Desktop/qr.svg
```

Provided source (`--src`) must be a valid PAY by square XML structure. You can either pass a path to the XML file
or directly provide an XML content.

JSON with the same PascalCase field names is also accepted. Amounts are kept as exact decimals, so values such as
`12.345678` are not rounded through floating-point arithmetic.

To save generated QR code as image, use `--save` option with path where to save the image. Type of the file is
determined by the output file extension. We support generating `svg`, `png` and `jpeg` images.

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

The decoder accepts the Base32hex content carried by the QR code and prints a
PAY document as JSON or XML. It evaluates the data payload; scanning an image is
outside the scope of the decoder.

```shell
bysqr decode --src '000620000...' --format json
bysqr decode --src payload.txt --format xml
```

## Build

To build a project, ensure you have latest [Rust](https://www.rust-lang.org/tools/install) installed. Then, run build using `cargo`:

```shell
cargo build --release
```

You can find `bysqrcli` executable and rust library in `target/release`.

## Rust API

Both deserialization and encoding return typed errors:

```rust
use bysqr::{decoder, encoder, models::try_deserialize_pay};

let pay = try_deserialize_pay(include_str!("payment.xml"))?;
let payload = encoder::encode(&pay)?;
let decoded = decoder::decode(&payload)?;
assert_eq!(decoded, pay);
# Ok::<(), bysqr::error::Error>(())
```

`encoder::encode_sequence` exposes the uncompressed tab-delimited form for
conformance tooling. `codec::decode_payload` validates and inspects an encoded
envelope, including its header, LZMA data, declared two-byte size, CRC32, and
UTF-8 sequence. `decoder::decode` validates and reconstructs the complete PAY
model, while `decoder::decode_sequence` can inspect an already uncompressed
sequence.

## Tests

Run the complete suite with:

```shell
cargo test --all-features
```

Offline fixtures under `tests/fixtures/pay` include known-good PAY by square
payloads and cases derived from `spec/bysquare.xsd`. Tests compare decoded data
instead of compressed strings because different LZMA streams can represent the
same valid payload. No external service or PNG scanning is required during
tests.

### WASM build

`bysqr` can be built for Web Assembly target, which allows you to run encoder and decoder in the browser, without need for a server.

Before building for `wasm` target, you need to install `wasm-pack`.

```shell
cargo install wasm-pack
```

After installing, you can start build:

```shell
wasm-pack build --target web
```

Built wasm module will be located in `pkg` folder.

#### Building for wasm on Ubuntu

Before building wasm on Ubuntu, make sure to install all necessary tools:

```shell
sudo apt install -y build-essential clang
```

#### Building for wasm on macOS

Apple clang is not supported when building for wasm target and you have to instal `llvm` instead.

```shell
# Install llvm
brew install llvm

# Add llvm to $PATH, you may place it to .zshrc
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/llvm/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm/include"

# Verify installation
llvm-config --version
```

## Roadmap to v1.0

- [x] PAY payment-order encoder
- [x] PAY standing-order encoder
- [x] PAY direct-debit encoder
- [x] PAY decoder
- [ ] Invoice encoder
- [ ] Invoice decoder
- [ ] alternative JSON input and output structure
- [ ] theming
- [ ] support for different logo position
- [ ] general code refactoring
- [x] PAY encoder conformance tests
