pub mod encode;
pub mod models;

pub use encode::{
    encode, encode_sequence, encode_sequence_with_limit, encode_with_limit, SequenceLimit,
    MAX_SEQUENCE_CHARACTERS,
};
pub use models::*;

/// Canonical INVOICE JSON Schema (Draft 2020-12), derived from `bysquare.xsd`.
pub const JSON_SCHEMA: &str = include_str!("../../spec/invoice-by-square.schema.json");
