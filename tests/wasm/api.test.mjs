import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  CHECKSUM_MISMATCH_PAYLOAD,
  INVALID_SEQUENCE_PAYLOAD,
  cargoPackageVersion,
  expectWasmError,
  fixture,
  fixtureJson,
  loadWasm,
} from "./helpers.mjs";

const DOMAIN_EXPORTS = [
  "bysqr_version",
  "decode_document",
  "decode_invoice_items_chunks",
  "document_diagnostics",
  "encode_document",
  "encode_invoice",
  "encode_invoice_items",
  "encode_invoice_items_chunks",
  "encode_pay",
  "render_jpeg",
  "render_png",
  "render_svg",
  "wasm_api_version",
];

const REMOVED_LEGACY_EXPORTS = [
  "decode_image_to_json",
  "decode_image_to_xml",
  "decode_to_json",
  "decode_to_xml",
  "encode_to_jpeg",
  "encode_to_png",
  "encode_to_svg",
];

test("generated module exposes exactly the v1 callable allowlist", async () => {
  const wasm = await loadWasm();
  assert.deepEqual(Object.keys(wasm).sort(), ["default", ...DOMAIN_EXPORTS, "initSync"].sort());
  for (const name of [...DOMAIN_EXPORTS, "default", "initSync"]) {
    assert.equal(typeof wasm[name], "function", `${name} must be callable`);
  }
  for (const name of REMOVED_LEGACY_EXPORTS) {
    assert.equal(name in wasm, false, `${name} must not be exported`);
  }
});

test("generated declarations contain only the v1 API and wasm-pack initializers", async () => {
  const declarations = await readFile(
    new URL("../../pkg/bysqr.d.ts", import.meta.url),
    "utf8",
  );
  const namedFunctions = [...declarations.matchAll(/^export function (\w+)/gm)].map(
    (match) => match[1],
  );

  assert.deepEqual(namedFunctions.sort(), [...DOMAIN_EXPORTS, "initSync"].sort());
  assert.match(declarations, /^export default function __wbg_init /m);
  for (const name of REMOVED_LEGACY_EXPORTS) {
    assert.doesNotMatch(declarations, new RegExp(`export function ${name}\\b`));
  }
});

test("metadata reports ABI 1 and the Cargo package version", async () => {
  const wasm = await loadWasm();
  assert.equal(wasm.wasm_api_version(), 1);
  assert.equal(wasm.bysqr_version(), await cargoPackageVersion());
});

test("encode_document classifies all families and accepts surrounding whitespace", async () => {
  const wasm = await loadWasm();
  const pay = await fixture("pay");
  const invoice = await fixture("invoice");
  const items = await fixture("invoiceItems");

  assert.equal(wasm.encode_document(` \n${pay}\n`), wasm.encode_pay(pay));
  assert.equal(wasm.encode_document(invoice), wasm.encode_invoice(invoice));
  assert.equal(wasm.encode_document(items), wasm.encode_invoice_items(items));
});

test("domain encoders reject wrong families and malformed source structurally", async () => {
  const wasm = await loadWasm();
  const pay = await fixture("pay");
  const invoice = await fixture("invoice");

  const payError = expectWasmError(() => wasm.encode_pay(invoice), "DESERIALIZE");
  assert.equal(payError.format, "JSON");

  const invoiceError = expectWasmError(
    () => wasm.encode_invoice(pay),
    "INVALID_INPUT",
  );
  assert.equal(invoiceError.field, "Invoice JSON");

  const itemsError = expectWasmError(
    () => wasm.encode_invoice_items(invoice),
    "INVALID_INPUT",
  );
  assert.equal(itemsError.field, "InvoiceItems JSON");

  const malformed = expectWasmError(() => wasm.encode_pay("{"), "DESERIALIZE");
  assert.equal(malformed.format, "JSON");

  const malformedDocument = expectWasmError(
    () => wasm.encode_document("{"),
    "DESERIALIZE",
  );
  assert.equal(malformedDocument.format, "JSON");

  const malformedInvoice = expectWasmError(
    () => wasm.encode_invoice("{"),
    "INVALID_INPUT",
  );
  assert.equal(malformedInvoice.field, "Invoice JSON");

  const malformedItems = expectWasmError(
    () => wasm.encode_invoice_items("{"),
    "INVALID_INPUT",
  );
  assert.equal(malformedItems.field, "InvoiceItems JSON");
});

test("decode_document returns tagged canonical shapes for every family", async () => {
  const wasm = await loadWasm();
  const cases = [
    ["pay", await fixture("pay"), "Payments"],
    ["invoice", await fixture("invoice"), "DocumentType"],
    ["invoiceItems", await fixture("invoiceItems"), "InvoiceLines"],
  ];

  for (const [type, source, canonicalField] of cases) {
    const payload = wasm.encode_document(source);
    const decoded = JSON.parse(wasm.decode_document(` \n${payload}\t`));
    assert.equal(decoded.type, type);
    assert.ok(Object.hasOwn(decoded.value, canonicalField));
  }
});

test("decode_document exposes invalid payload, checksum, and sequence details", async () => {
  const wasm = await loadWasm();

  expectWasmError(() => wasm.decode_document("not-a-payload"), "INVALID_PAYLOAD");

  const checksum = expectWasmError(
    () => wasm.decode_document(CHECKSUM_MISMATCH_PAYLOAD),
    "CHECKSUM_MISMATCH",
  );
  assert.equal(checksum.expected, 0);
  assert.equal(typeof checksum.actual, "number");

  const sequence = expectWasmError(
    () => wasm.decode_document(INVALID_SEQUENCE_PAYLOAD),
    "INVALID_SEQUENCE",
  );
  assert.equal(sequence.field, "Payments");
  assert.equal(sequence.position, 2);
});

test("render_svg supports defaults, themes, and trimmed payloads", async () => {
  const wasm = await loadWasm();
  const payload = wasm.encode_pay(await fixture("pay"));

  const defaultSvg = wasm.render_svg(`\n${payload} `, "{}");
  assert.match(defaultSvg, /^<svg/);
  assert.match(defaultSvg, /viewBox="0 0 512 600"/);
  assert.match(defaultSvg, /#6FA4D7/);

  const themedSvg = wasm.render_svg(
    payload,
    JSON.stringify({ layout: "electronic", position: "right", color: "gray" }),
  );
  assert.match(themedSvg, /viewBox="0 0 600 512"/);
  assert.match(themedSvg, /#5F6062/);
});

test("render_svg rejects malformed, unknown, and unsupported options", async () => {
  const wasm = await loadWasm();
  const payPayload = wasm.encode_pay(await fixture("pay"));
  const itemsPayload = wasm.encode_invoice_items(await fixture("invoiceItems"));

  const malformed = expectWasmError(
    () => wasm.render_svg(payPayload, "{"),
    "DESERIALIZE",
  );
  assert.equal(malformed.format, "render options JSON");

  const unknown = expectWasmError(
    () => wasm.render_svg(payPayload, '{"unknown":true}'),
    "DESERIALIZE",
  );
  assert.equal(unknown.format, "render options JSON");

  const invalidValue = expectWasmError(
    () => wasm.render_svg(payPayload, '{"layout":"screen"}'),
    "INVALID_INPUT",
  );
  assert.equal(invalidValue.field, "layout");

  const itemsTheme = expectWasmError(
    () => wasm.render_svg(itemsPayload, '{"color":"black"}'),
    "INVALID_INPUT",
  );
  assert.equal(itemsTheme.field, "options");
});

test("render_png returns raw PNG bytes and validates width", async () => {
  const wasm = await loadWasm();
  const payload = wasm.encode_pay(await fixture("pay"));
  const png = wasm.render_png(payload, 256, "{}");

  assert.ok(png instanceof Uint8Array);
  assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);

  const width = expectWasmError(
    () => wasm.render_png(payload, 0, "{}"),
    "INVALID_INPUT",
  );
  assert.equal(width.field, "size");
});

test("render_jpeg returns raw JPEG bytes and validates quality", async () => {
  const wasm = await loadWasm();
  const payload = wasm.encode_pay(await fixture("pay"));
  const jpeg = wasm.render_jpeg(payload, 256, 85, "{}");

  assert.ok(jpeg instanceof Uint8Array);
  assert.deepEqual([...jpeg.subarray(0, 2)], [255, 216]);
  assert.deepEqual([...jpeg.subarray(-2)], [255, 217]);

  const quality = expectWasmError(
    () => wasm.render_jpeg(payload, 256, 0, "{}"),
    "INVALID_INPUT",
  );
  assert.equal(quality.field, "quality");
});

test("INVOICE ITEMS chunk functions round trip more than four lines", async () => {
  const wasm = await loadWasm();
  const items = await fixtureJson("invoiceItems");
  const lines = Array.from({ length: 5 }, () => structuredClone(items.InvoiceLines.InvoiceLine[0]));

  const payloads = JSON.parse(
    wasm.encode_invoice_items_chunks("INV-CHUNKS", JSON.stringify(lines)),
  );
  assert.equal(payloads.length, 2);

  const decoded = JSON.parse(
    wasm.decode_invoice_items_chunks(JSON.stringify(payloads)),
  );
  assert.equal(decoded.InvoiceID, "INV-CHUNKS");
  assert.deepEqual(decoded.InvoiceLines.InvoiceLine, lines);
});

test("INVOICE ITEMS chunk functions reject malformed input and incomplete reassembly", async () => {
  const wasm = await loadWasm();
  const items = await fixtureJson("invoiceItems");
  const lines = Array.from({ length: 5 }, () => structuredClone(items.InvoiceLines.InvoiceLine[0]));
  const payloads = JSON.parse(
    wasm.encode_invoice_items_chunks("INV-CHUNKS", JSON.stringify(lines)),
  );

  const malformedLines = expectWasmError(
    () => wasm.encode_invoice_items_chunks("INV-CHUNKS", "{"),
    "DESERIALIZE",
  );
  assert.equal(malformedLines.format, "InvoiceLine[] JSON");

  const malformedPayloads = expectWasmError(
    () => wasm.decode_invoice_items_chunks("{"),
    "DESERIALIZE",
  );
  assert.equal(malformedPayloads.format, "payload array JSON");

  const incomplete = expectWasmError(
    () => wasm.decode_invoice_items_chunks(JSON.stringify(payloads.slice(1))),
    "INVALID_INPUT",
  );
  assert.equal(incomplete.field, "FirstInvoiceLineID");
});

test("document_diagnostics reports correct empty and non-empty family results", async () => {
  const wasm = await loadWasm();

  const pay = await fixtureJson("pay");
  assert.deepEqual(JSON.parse(wasm.document_diagnostics(JSON.stringify(pay))), []);
  pay.Payments.Payment[0].PaymentNote = "x".repeat(141);
  const payDiagnostics = JSON.parse(wasm.document_diagnostics(JSON.stringify(pay)));
  assert.ok(
    payDiagnostics.some(
      (entry) =>
        entry.fieldPath.endsWith("PaymentNote") &&
        entry.actualCharacterCount === 141 &&
        entry.recommendedMaximum === 140,
    ),
  );

  const invoiceMinimal = await fixture("invoiceMinimal");
  assert.deepEqual(JSON.parse(wasm.document_diagnostics(invoiceMinimal)), []);
  const invoiceDiagnostics = JSON.parse(
    wasm.document_diagnostics(await fixture("invoice")),
  );
  assert.ok(invoiceDiagnostics.some((entry) => entry.fieldPath === "InvoiceID"));

  const items = await fixtureJson("invoiceItems");
  const itemsDiagnostics = JSON.parse(
    wasm.document_diagnostics(JSON.stringify(items)),
  );
  assert.ok(itemsDiagnostics.some((entry) => entry.fieldPath === "InvoiceID"));
  items.InvoiceID = "INV-1";
  assert.deepEqual(JSON.parse(wasm.document_diagnostics(JSON.stringify(items))), []);
});

test("structured errors preserve count limits and mandatory base fields", async () => {
  const wasm = await loadWasm();
  const pay = await fixtureJson("pay");
  pay.Payments.Payment[0].PaymentNote = "x".repeat(1_000);

  const limit = expectWasmError(
    () => wasm.encode_pay(JSON.stringify(pay)),
    "SEQUENCE_TOO_LONG",
  );
  assert.equal(limit.maximum, 550);
  assert.ok(limit.actual > limit.maximum);
  assert.equal(typeof limit.code, "string");
  assert.equal(typeof limit.message, "string");
});
