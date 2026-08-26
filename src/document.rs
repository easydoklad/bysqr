//! Classification and operations shared by all by-square document families.

use quick_xml::events::Event;
use serde::Serialize;

use crate::{
    codec::{self, Header},
    error::{Error, Result},
    invoice::{self, DocumentType},
    invoice_items, pay,
};

/// A by-square document decoded from data or a QR payload.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(untagged)]
pub enum Document {
    Pay(pay::Pay),
    Invoice(Box<invoice::Invoice>),
    InvoiceItems(Box<invoice_items::InvoiceItems>),
}

impl Document {
    /// Encode this document into its Base32hex QR payload.
    pub fn encode(&self) -> Result<String> {
        match self {
            Self::Pay(pay) => pay::encode(pay),
            Self::Invoice(invoice) => invoice::encode(invoice),
            Self::InvoiceItems(items) => invoice_items::encode(items),
        }
    }

    /// Serialize this document as canonical, pretty-printed JSON.
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|error| Error::Deserialize {
            format: "JSON",
            message: error.to_string(),
        })
    }

    /// Serialize this document using its canonical XML root and type metadata.
    pub fn to_xml(&self) -> Result<String> {
        match self {
            Self::Pay(pay) => pay.to_xml_string(),
            Self::Invoice(invoice) => invoice.to_xml_string().map_err(|error| Error::Deserialize {
                format: "XML",
                message: error.to_string(),
            }),
            Self::InvoiceItems(items) => {
                items.to_xml_string().map_err(|error| Error::Deserialize {
                    format: "XML",
                    message: error.to_string(),
                })
            }
        }
    }
}

/// Deserialize canonical PAY or INVOICE JSON/XML by inspecting its root data.
pub fn try_deserialize(source: &str) -> Result<Document> {
    let source = source.trim_start();
    if source.starts_with('{') {
        let value: serde_json::Value =
            serde_json::from_str(source).map_err(|error| Error::Deserialize {
                format: "JSON",
                message: error.to_string(),
            })?;
        if value.get("DocumentType").is_some() {
            return invoice::try_deserialize_invoice(source)
                .map(|invoice| Document::Invoice(Box::new(invoice)))
                .map_err(|error| Error::Deserialize {
                    format: "Invoice JSON",
                    message: error.to_string(),
                });
        }
        if value.get("FirstInvoiceLineID").is_some() && value.get("InvoiceLines").is_some() {
            return invoice_items::try_deserialize_invoice_items(source)
                .map(|items| Document::InvoiceItems(Box::new(items)))
                .map_err(|error| Error::Deserialize {
                    format: "InvoiceItems JSON",
                    message: error.to_string(),
                });
        }
        return pay::try_deserialize_pay(source).map(Document::Pay);
    }

    if source.starts_with('<') {
        return match xml_root(source)?.as_str() {
            "Pay" => pay::try_deserialize_pay(source).map(Document::Pay),
            "Invoice" => invoice::try_deserialize_invoice(source)
                .map(|invoice| Document::Invoice(Box::new(invoice)))
                .map_err(|error| Error::Deserialize {
                    format: "Invoice XML",
                    message: error.to_string(),
                }),
            "InvoiceItems" => invoice_items::try_deserialize_invoice_items(source)
                .map(|items| Document::InvoiceItems(Box::new(items)))
                .map_err(|error| Error::Deserialize {
                    format: "InvoiceItems XML",
                    message: error.to_string(),
                }),
            root => Err(Error::Unsupported(format!(
                "unsupported by-square XML root {root:?}"
            ))),
        };
    }

    Err(Error::Deserialize {
        format: "document",
        message: "expected an XML document or JSON object".to_owned(),
    })
}

/// Decode a payload and dispatch it according to its by-square header.
pub fn decode(payload: &str) -> Result<Document> {
    let decoded = codec::decode_payload(payload)?;

    if decoded.header == Header::PAY {
        return pay::decode_sequence(&decoded.sequence).map(Document::Pay);
    }

    if decoded.header.by_square_type == 1
        && decoded.header.version == 0
        && decoded.header.reserved == 0
    {
        let document_type =
            DocumentType::from_classifier(decoded.header.document_type).map_err(|error| {
                Error::InvalidPayload(format!(
                    "invalid INVOICE document type classifier {}: {error}",
                    decoded.header.document_type
                ))
            })?;
        return invoice::decode_sequence(&decoded.sequence, document_type)
            .map(|invoice| Document::Invoice(Box::new(invoice)));
    }

    if decoded.header
        == (Header {
            by_square_type: 2,
            version: 0,
            document_type: 0,
            reserved: 0,
        })
    {
        return invoice_items::decode_sequence(&decoded.sequence)
            .map(|items| Document::InvoiceItems(Box::new(items)));
    }

    Err(Error::Unsupported(format!(
        "unsupported by-square header {}/{}/{}/{}",
        decoded.header.by_square_type,
        decoded.header.version,
        decoded.header.document_type,
        decoded.header.reserved
    )))
}

fn xml_root(source: &str) -> Result<String> {
    let mut reader = quick_xml::Reader::from_str(source);
    loop {
        match reader.read_event() {
            Ok(Event::Start(root)) | Ok(Event::Empty(root)) => {
                return std::str::from_utf8(root.local_name().as_ref())
                    .map(str::to_owned)
                    .map_err(|error| Error::Deserialize {
                        format: "XML",
                        message: error.to_string(),
                    });
            }
            Ok(Event::Eof) => {
                return Err(Error::Deserialize {
                    format: "XML",
                    message: "document has no root element".to_owned(),
                });
            }
            Ok(_) => {}
            Err(error) => {
                return Err(Error::Deserialize {
                    format: "XML",
                    message: error.to_string(),
                });
            }
        }
    }
}
