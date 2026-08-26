//! PAY by square documents.

pub mod decoder;
pub mod encoder;
pub mod models;

pub use decoder::{decode, decode_sequence};
pub use encoder::{
    encode, encode_sequence, encode_sequence_with_limit, encode_with_limit, SequenceLimit,
    MAX_SEQUENCE_CHARACTERS,
};
pub use models::*;

/// Canonical PAY JSON Schema (Draft 2020-12), derived from `bysquare.xsd`.
pub const JSON_SCHEMA: &str = include_str!("../../spec/pay-by-square.schema.json");
