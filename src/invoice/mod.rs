pub mod models;

pub use models::*;

/// Canonical INVOICE JSON Schema (Draft 2020-12), derived from `bysquare.xsd`.
pub const JSON_SCHEMA: &str = include_str!("../../spec/invoice-by-square.schema.json");
