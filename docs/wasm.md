# WebAssembly adapter contract

The `wasm` feature exposes the low-level WebAssembly contract used by JavaScript
SDKs. The ABI version is `1`.

The ABI uses strings, numbers, and byte arrays. PAY, INVOICE, and INVOICE ITEMS
documents cross it as JSON or XML strings; encoded QR payloads are Base32hex
strings; PNG and JPEG results are returned as `Uint8Array`. Rust domain structs
are not exposed as JavaScript classes.

## Exports

| Export | Input | Result |
| --- | --- | --- |
| `wasm_api_version()` | none | `1` |
| `bysqr_version()` | none | Rust crate version string |
| `encode_document(source)` | canonical JSON/XML, auto-classified | payload string |
| `encode_pay(source)` | canonical PAY JSON/XML | payload string |
| `encode_invoice(source)` | canonical INVOICE JSON/XML | payload string |
| `encode_invoice_items(source)` | canonical INVOICE ITEMS JSON/XML | payload string |
| `decode_document(payload)` | payload string | tagged canonical JSON string |
| `render_svg(payload, optionsJson)` | payload and render options | SVG string |
| `render_png(payload, width, optionsJson)` | payload, pixel width, options | raw PNG bytes |
| `render_jpeg(payload, width, quality, optionsJson)` | payload, pixel width, quality 1–100, options | raw JPEG bytes |
| `encode_invoice_items_chunks(invoiceId, linesJson)` | ID and canonical `InvoiceLine[]` JSON | JSON array of payload strings |
| `decode_invoice_items_chunks(payloadsJson)` | JSON array of payload strings | reassembled canonical JSON string |
| `document_diagnostics(source)` | canonical JSON/XML | JSON diagnostic array |

`decode_document` returns one of these tagged shapes. The object under `value`
retains the PascalCase field names defined by the JSON Schemas.

```json
{"type":"pay","value":{}}
{"type":"invoice","value":{}}
{"type":"invoiceItems","value":{}}
```

The chunk decoder returns the canonical `InvoiceItemsList` shape:

```json
{"InvoiceID":"...","InvoiceLines":{"InvoiceLine":[]}}
```

## Encoding versus rendering

The `encode_*` functions create payloads from source documents. The `render_*`
functions validate and render an existing payload. Callers can therefore store
or transport a payload independently of its source document and rendering.

## Render options

`optionsJson` must be a JSON object. Every field is optional:

```json
{
  "layout": "print",
  "position": "bottom",
  "color": "dark"
}
```

Allowed layouts are `print` and `electronic`; positions are `bottom`, `top`,
`left`, and `right`; colors are `light`, `dark`, `gray`, and `black`. Missing
fields use the Rust `LogoTheme` default shown above. INVOICE ITEMS only supports
that default theme. An explicitly supplied set of default values is accepted;
any effective non-default option is rejected.

## Diagnostics and errors

`document_diagnostics` never changes or rejects an otherwise valid document for
an advisory length overflow. It returns entries shaped as:

```json
{
  "fieldPath": "Payments.Payment[0].PaymentNote",
  "actualCharacterCount": 141,
  "recommendedMaximum": 140
}
```

Failures are thrown as JavaScript objects. Every object has stable `code` and
`message` fields. Depending on the error it can also have `field`, `position`,
`actual`, `maximum`, `expected`, `format`, `decoded`, or `count`. Consumers
should branch on `code` and read structured fields instead of parsing `message`.

The error codes reachable through this API are:

```text
INVALID_INPUT
UNSUPPORTED
SEQUENCE_TOO_LONG
PAYLOAD_TOO_LONG
INVALID_PAYLOAD
INVALID_SEQUENCE
COMPRESSION
CHECKSUM_MISMATCH
DESERIALIZE
QR_ENCODE
SVG_RENDER
IMAGE_ENCODE
UTF8
```

The adapter can additionally report `SERIALIZE` if it cannot produce one of its
JSON return strings. Family model errors are normalized as `INVALID_INPUT` with
their model `field` preserved.

## Runtime initialization

The release archive uses wasm-pack's `web` target. Browsers can load the
generated `.wasm` URL. In runtimes such as Node, pass the WASM bytes explicitly:

```js
import { readFile } from "node:fs/promises";
import init, { encode_pay } from "./bysqr.js";

const bytes = await readFile(new URL("./bysqr_bg.wasm", import.meta.url));
await init({ module_or_path: bytes });
const payload = encode_pay(canonicalPayJson);
```
