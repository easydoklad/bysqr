//! INVOICE ITEMS by square sequence, payload, and chunk encoding.

use super::models::{InvoiceItems, InvoiceLine, InvoiceLines};
use crate::{
    codec::{self, Header},
    error::{Error, Result},
    invoice::{Invoice, InvoiceModelError},
};

/// Conservative maximum recommended by the INVOICE ITEMS specification.
pub const RECOMMENDED_INVOICE_LINES_PER_QR: usize = 4;

const HEADER: Header = Header {
    by_square_type: 2,
    version: 0,
    document_type: 0,
    reserved: 0,
};

/// Reassembled semantic line set shared by one parent invoice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReassembledInvoiceLines {
    pub invoice_id: String,
    pub invoice_lines: Vec<InvoiceLine>,
}

impl ReassembledInvoiceLines {
    /// Verify pairing and completeness against the parent multi-line Invoice.
    pub fn validate_against_invoice(
        &self,
        invoice: &Invoice,
    ) -> std::result::Result<(), InvoiceModelError> {
        if self.invoice_id != invoice.data.invoice_id {
            return Err(InvoiceModelError::invalid(
                "InvoiceID",
                "Items blocks do not belong to the supplied Invoice",
            ));
        }
        let expected = invoice.data.number_of_invoice_lines.ok_or_else(|| {
            InvoiceModelError::invalid(
                "NumberOfInvoiceLines",
                "parent Invoice does not declare a multi-line item count",
            )
        })?;
        let expected = usize::try_from(expected).map_err(|_| {
            InvoiceModelError::invalid(
                "NumberOfInvoiceLines",
                "must be a non-negative count that fits usize",
            )
        })?;
        if self.invoice_lines.len() != expected {
            return Err(InvoiceModelError::invalid(
                "InvoiceLines",
                format!(
                    "parent Invoice declares {expected} lines, reassembled {}",
                    self.invoice_lines.len()
                ),
            ));
        }
        Ok(())
    }
}

/// Encode one INVOICE ITEMS block into its Base32hex QR payload.
pub fn encode(document: &InvoiceItems) -> Result<String> {
    codec::encode_payload(HEADER, &encode_sequence(document)?)
}

/// Chunk a complete ordered line list and encode every resulting QR payload.
pub fn encode_chunks(
    invoice_id: impl Into<String>,
    invoice_lines: Vec<InvoiceLine>,
) -> Result<Vec<String>> {
    chunk_invoice_lines(invoice_id, invoice_lines)
        .map_err(|error| Error::invalid(error.field(), error.message()))?
        .iter()
        .map(encode)
        .collect()
}

/// Serialize one INVOICE ITEMS block into the specification's TSV sequence.
pub fn encode_sequence(document: &InvoiceItems) -> Result<String> {
    document
        .validate()
        .map_err(|error| Error::invalid(error.field(), error.message()))?;

    let lines = &document.invoice_lines.invoice_line;
    let mut fields = Vec::with_capacity(3 + 12 * lines.len());
    fields.push(sanitized(&document.invoice_id));
    fields.push(sanitized(&document.first_invoice_line_id));
    fields.push(lines.len().to_string());

    for line in lines {
        let order = line.order_reference.as_ref();
        fields.push(optional_text(
            order.and_then(|reference| reference.order_id.as_deref()),
        ));
        fields.push(optional_text(
            order.and_then(|reference| reference.order_line_id.as_deref()),
        ));

        let delivery = line.delivery_note_reference.as_ref();
        fields.push(optional_text(
            delivery.and_then(|reference| reference.delivery_note_id.as_deref()),
        ));
        fields.push(optional_text(
            delivery.and_then(|reference| reference.delivery_note_line_id.as_deref()),
        ));

        fields.push(optional_text(line.item_name.as_deref()));
        fields.push(optional_text(line.item_ean_code.as_deref()));
        fields.push(optional_date(line.period_from_date.as_ref()));
        fields.push(optional_date(line.period_to_date.as_ref()));
        fields.push(line.invoiced_quantity.to_string());
        fields.push(line.unit_price_tax_exclusive_amount.to_string());
        fields.push(line.unit_price_tax_amount.to_string());
        fields.push(line.classified_tax_category.as_str().to_owned());
    }

    debug_assert_eq!(fields.len(), 3 + 12 * lines.len());
    Ok(fields.join("\t"))
}

/// Split a complete ordered line list into conservative four-line QR blocks.
pub fn chunk_invoice_lines(
    invoice_id: impl Into<String>,
    invoice_lines: Vec<InvoiceLine>,
) -> std::result::Result<Vec<InvoiceItems>, InvoiceModelError> {
    let invoice_id = invoice_id.into();
    if invoice_lines.is_empty() {
        return Err(InvoiceModelError::invalid(
            "InvoiceLines",
            "must contain at least one InvoiceLine",
        ));
    }
    for line in &invoice_lines {
        line.validate()?;
    }

    invoice_lines
        .chunks(RECOMMENDED_INVOICE_LINES_PER_QR)
        .enumerate()
        .map(|(index, chunk)| {
            InvoiceItems::new(
                invoice_id.clone(),
                (index * RECOMMENDED_INVOICE_LINES_PER_QR + 1).to_string(),
                InvoiceLines::new(chunk.to_vec()),
            )
        })
        .collect()
}

/// Sort and reassemble sequential blocks, rejecting mixed invoices, gaps, and overlaps.
pub fn reassemble_invoice_lines(
    documents: impl IntoIterator<Item = InvoiceItems>,
) -> std::result::Result<ReassembledInvoiceLines, InvoiceModelError> {
    let mut documents = documents.into_iter().collect::<Vec<_>>();
    if documents.is_empty() {
        return Err(InvoiceModelError::invalid(
            "InvoiceItems",
            "at least one block is required",
        ));
    }
    for document in &documents {
        document.validate()?;
    }

    let invoice_id = documents[0].invoice_id.clone();
    if documents
        .iter()
        .any(|document| document.invoice_id != invoice_id)
    {
        return Err(InvoiceModelError::invalid(
            "InvoiceID",
            "all blocks must belong to the same invoice",
        ));
    }

    let parse_first = |document: &InvoiceItems| {
        if document.first_invoice_line_id.is_empty()
            || !document
                .first_invoice_line_id
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        {
            return Err(InvoiceModelError::invalid(
                "FirstInvoiceLineID",
                "reassembly requires a positive ASCII integer",
            ));
        }
        document
            .first_invoice_line_id
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                InvoiceModelError::invalid(
                    "FirstInvoiceLineID",
                    "reassembly requires a positive ASCII integer",
                )
            })
    };

    let mut indexed = documents
        .drain(..)
        .map(|document| parse_first(&document).map(|first| (first, document)))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    indexed.sort_by_key(|(first, _)| *first);

    let mut expected = 1_usize;
    let mut invoice_lines = Vec::new();
    for (first, document) in indexed {
        if first != expected {
            return Err(InvoiceModelError::invalid(
                "FirstInvoiceLineID",
                format!("expected {expected}, found {first}"),
            ));
        }
        expected = expected
            .checked_add(document.invoice_lines.invoice_line.len())
            .ok_or_else(|| InvoiceModelError::invalid("InvoiceLines", "line count is too large"))?;
        invoice_lines.extend(document.invoice_lines.invoice_line);
    }

    Ok(ReassembledInvoiceLines {
        invoice_id,
        invoice_lines,
    })
}

fn optional_text(value: Option<&str>) -> String {
    value.map_or_else(String::new, sanitized)
}

fn optional_date(date: Option<&crate::invoice::Date>) -> String {
    date.map_or_else(String::new, |value| value.as_str().replace('-', ""))
}

fn sanitized(value: &str) -> String {
    value.replace('\t', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice_items::decode_chunks;
    use crate::{
        invoice::{Date, Decimal, Percentage},
        invoice_items::{decode_sequence, DeliveryNoteReference, OrderReference},
    };

    fn line(index: usize) -> InvoiceLine {
        InvoiceLine {
            order_reference: None,
            delivery_note_reference: None,
            item_name: Some(format!("Item {index}")),
            item_ean_code: None,
            period_from_date: None,
            period_to_date: None,
            invoiced_quantity: Decimal::new("1").unwrap(),
            unit_price_tax_exclusive_amount: Decimal::new("10").unwrap(),
            unit_price_tax_inclusive_amount: None,
            unit_price_tax_amount: Decimal::new("2").unwrap(),
            line_tax_exclusive_amount: None,
            line_tax_inclusive_amount: None,
            line_tax_amount: None,
            classified_tax_category: Percentage::new("0.2").unwrap(),
        }
    }

    #[test]
    fn chunks_four_lines_and_reassembles_out_of_order() {
        let lines = (1..=10).map(line).collect::<Vec<_>>();
        let chunks = chunk_invoice_lines("INV-1", lines.clone()).unwrap();
        assert_eq!(
            chunks
                .iter()
                .map(|item| item.invoice_lines.invoice_line.len())
                .collect::<Vec<_>>(),
            vec![4, 4, 2]
        );
        assert_eq!(
            chunks
                .iter()
                .map(|item| item.first_invoice_line_id.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "5", "9"]
        );

        let merged = reassemble_invoice_lines(chunks.into_iter().rev()).unwrap();
        assert_eq!(merged.invoice_id, "INV-1");
        assert_eq!(merged.invoice_lines, lines);
    }

    #[test]
    fn rejects_gaps_and_mixed_invoice_ids() {
        let mut chunks = chunk_invoice_lines("INV-1", (1..=5).map(line).collect()).unwrap();
        chunks[1].first_invoice_line_id = "6".to_owned();
        assert_eq!(
            reassemble_invoice_lines(chunks).unwrap_err().field(),
            "FirstInvoiceLineID"
        );

        let mut chunks = chunk_invoice_lines("INV-1", (1..=5).map(line).collect()).unwrap();
        chunks[1].invoice_id = "INV-2".to_owned();
        assert_eq!(
            reassemble_invoice_lines(chunks).unwrap_err().field(),
            "InvoiceID"
        );
    }

    #[test]
    fn encoded_chunks_round_trip_as_one_line_set() {
        let lines = (1..=9).map(line).collect::<Vec<_>>();
        let payloads = encode_chunks("INV-1", lines.clone()).unwrap();
        assert_eq!(payloads.len(), 3);
        let decoded = decode_chunks(&payloads).unwrap();
        assert_eq!(decoded.invoice_id, "INV-1");
        assert_eq!(decoded.invoice_lines, lines);
    }

    #[test]
    fn transports_periods_partial_references_and_sanitized_text() {
        let mut item = line(1);
        item.item_name = Some("Monthly\tservice".to_owned());
        item.order_reference = Some(OrderReference {
            order_id: None,
            order_line_id: Some("OL-1".to_owned()),
        });
        item.delivery_note_reference = Some(DeliveryNoteReference {
            delivery_note_id: Some("DN-1".to_owned()),
            delivery_note_line_id: None,
        });
        item.period_from_date = Some(Date::new("2026-08-01").unwrap());
        item.period_to_date = Some(Date::new("2026-08-31").unwrap());
        let document = InvoiceItems::new("INV\t1", "1", InvoiceLines::new(vec![item])).unwrap();

        let sequence = encode_sequence(&document).unwrap();
        assert_eq!(sequence.split('\t').count(), 15);
        let decoded = decode_sequence(&sequence).unwrap();
        assert_eq!(decoded.invoice_id, "INV 1");
        assert_eq!(
            decoded.invoice_lines.invoice_line[0].item_name.as_deref(),
            Some("Monthly service")
        );
        assert_eq!(
            decoded.invoice_lines.invoice_line[0]
                .period_to_date
                .as_ref()
                .unwrap()
                .as_str(),
            "2026-08-31"
        );
    }

    #[test]
    fn computed_fields_are_not_part_of_the_sequence() {
        let mut item = line(1);
        item.unit_price_tax_inclusive_amount = Some(Decimal::new("12").unwrap());
        item.line_tax_exclusive_amount = Some(Decimal::new("10").unwrap());
        item.line_tax_inclusive_amount = Some(Decimal::new("12").unwrap());
        item.line_tax_amount = Some(Decimal::new("2").unwrap());
        let document = InvoiceItems::new("INV-1", "1", InvoiceLines::new(vec![item])).unwrap();

        let decoded = decode_sequence(&encode_sequence(&document).unwrap()).unwrap();
        let line = &decoded.invoice_lines.invoice_line[0];
        assert!(line.unit_price_tax_inclusive_amount.is_none());
        assert!(line.line_tax_exclusive_amount.is_none());
        assert!(line.line_tax_inclusive_amount.is_none());
        assert!(line.line_tax_amount.is_none());
    }
}
