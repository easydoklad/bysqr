import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import init, * as wasm from "../../pkg/bysqr.js";

const fixtureUrls = {
  pay: new URL("../fixtures/pay/json/direct-debit-sepa.json", import.meta.url),
  invoice: new URL(
    "../fixtures/invoice/valid-interoperability-offline-single-line.json",
    import.meta.url,
  ),
  invoiceMinimal: new URL(
    "../fixtures/invoice/schema/minimal-header-invoice.json",
    import.meta.url,
  ),
  invoiceItems: new URL(
    "../fixtures/invoice-items/valid-interoperability-offline-mixed-lines.json",
    import.meta.url,
  ),
};

let initialized;

export async function loadWasm() {
  initialized ??= (async () => {
    const wasmBytes = await readFile(
      new URL("../../pkg/bysqr_bg.wasm", import.meta.url),
    );
    await init({ module_or_path: wasmBytes });
    return wasm;
  })();
  return initialized;
}

export async function fixture(name) {
  return readFile(fixtureUrls[name], "utf8");
}

export async function fixtureJson(name) {
  return JSON.parse(await fixture(name));
}

export async function cargoPackageVersion() {
  const manifest = await readFile(
    new URL("../../Cargo.toml", import.meta.url),
    "utf8",
  );
  const packageSection = manifest.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const version = packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  assert.ok(version, "Cargo package version must be readable");
  return version;
}

export function expectWasmError(action, code) {
  let caught;
  try {
    action();
  } catch (error) {
    caught = error;
  }

  assert.ok(caught && typeof caught === "object", "expected a structured error object");
  assert.equal(caught.code, code);
  assert.equal(typeof caught.message, "string");
  assert.ok(caught.message.length > 0);
  return caught;
}

// PAY payloads with valid LZMA envelopes built from deliberately malformed
// uncompressed data. They provide deterministic boundary coverage for errors
// that cannot be produced by a valid model encoder.
export const CHECKSUM_MISMATCH_PAYLOAD =
  "0001400001K8194RRVDGITAN3OFEUUDSEJD47VVV5480000";
export const INVALID_SEQUENCE_PAYLOAD =
  "0000E000FM742Q3223698PV4BVVVUQP40000";
