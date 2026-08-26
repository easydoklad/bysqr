//! Classification and decoding shared by all by-square document families.

use crate::{
    codec::{self, Header},
    error::{Error, Result},
    pay,
};

/// A decoded by-square document.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Document {
    Pay(pay::Pay),
}

/// Decode a payload and dispatch it according to its by-square header.
pub fn decode(payload: &str) -> Result<Document> {
    let decoded = codec::decode_payload(payload)?;

    if decoded.header == Header::PAY {
        return pay::decode_sequence(&decoded.sequence).map(Document::Pay);
    }

    Err(Error::Unsupported(format!(
        "unsupported by-square header {}/{}/{}/{}",
        decoded.header.by_square_type,
        decoded.header.version,
        decoded.header.document_type,
        decoded.header.reserved
    )))
}
