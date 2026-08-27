//! INVOICE ITEMS by square documents for multi-line invoices.

pub mod decode;
pub mod encode;
pub mod models;

pub use decode::{decode, decode_chunks, decode_sequence};
pub use encode::{
    chunk_invoice_items_list, chunk_invoice_lines, encode, encode_chunks,
    encode_invoice_items_list, encode_sequence, reassemble_invoice_lines, ReassembledInvoiceLines,
    RECOMMENDED_INVOICE_LINES_PER_QR,
};
pub use models::*;

/// Canonical INVOICE ITEMS JSON Schema (Draft 2020-12), derived from `bysquare.xsd`.
pub const JSON_SCHEMA: &str = include_str!("../../spec/invoice-items-by-square.schema.json");

/// Standalone JSON Schema for a complete application-level INVOICE ITEMS list.
pub const JSON_SCHEMA_LIST: &str = include_str!("../../spec/invoice-items-list.schema.json");
