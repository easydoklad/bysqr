#!/usr/bin/env bash
set -euo pipefail

wasm-pack build --target web "$@" --features wasm
node --test tests/wasm/*.test.mjs
