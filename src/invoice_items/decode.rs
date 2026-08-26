//! INVOICE ITEMS by square payload and TSV decoding.

use super::encode::{reassemble_invoice_lines, ReassembledInvoiceLines};
use super::models::{
    DeliveryNoteReference, InvoiceItems, InvoiceLine, InvoiceLines, OrderReference,
};
use crate::{
    codec::{self, Header},
    error::{Error, Result},
    invoice::{Date, Decimal, Percentage},
};

const HEADER: Header = Header {
    by_square_type: 2,
    version: 0,
    document_type: 0,
    reserved: 0,
};

/// Decode one Base32hex INVOICE ITEMS payload.
pub fn decode(payload: &str) -> Result<InvoiceItems> {
    let decoded = codec::decode_payload(payload)?;
    if decoded.header != HEADER {
        return Err(Error::Unsupported(format!(
            "expected INVOICE ITEMS header 2/0/0/0, found {}/{}/{}/{}",
            decoded.header.by_square_type,
            decoded.header.version,
            decoded.header.document_type,
            decoded.header.reserved
        )));
    }
    decode_sequence(&decoded.sequence)
}

/// Decode and reassemble a complete set of INVOICE ITEMS payloads.
pub fn decode_chunks<I, S>(payloads: I) -> Result<ReassembledInvoiceLines>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let documents = payloads
        .into_iter()
        .map(|payload| decode(payload.as_ref()))
        .collect::<Result<Vec<_>>>()?;
    reassemble_invoice_lines(documents)
        .map_err(|error| Error::invalid(error.field(), error.message()))
}

/// Parse the tab-delimited sequence of one INVOICE ITEMS block.
pub fn decode_sequence(sequence: &str) -> Result<InvoiceItems> {
    let mut reader = SequenceReader::new(sequence);
    let invoice_id = reader.text("InvoiceID")?;
    let first_invoice_line_id = reader.text("FirstInvoiceLineID")?;
    let line_count = reader.positive_count("InvoiceLines")?;
    let mut lines = Vec::with_capacity(line_count);

    for _ in 0..line_count {
        let order_id = reader.optional_text("OrderID")?;
        let order_line_id = reader.optional_text("OrderLineID")?;
        let delivery_note_id = reader.optional_text("DeliveryNoteID")?;
        let delivery_note_line_id = reader.optional_text("DeliveryNoteLineID")?;
        let item_name = reader.optional_text("ItemName")?;
        let item_ean_code = reader.optional_text("ItemEANCode")?;
        let period_from_date = reader.optional_date("PeriodFromDate")?;
        let period_to_date = reader.optional_date("PeriodToDate")?;
        let invoiced_quantity = reader.decimal("InvoicedQuantity")?;
        let unit_price_tax_exclusive_amount = reader.decimal("UnitPriceTaxExclusiveAmount")?;
        let unit_price_tax_amount = reader.decimal("UnitPriceTaxAmount")?;
        let classified_tax_category = reader.vat_percentage("ClassifiedTaxCategory")?;

        lines.push(InvoiceLine {
            order_reference: if order_id.is_some() || order_line_id.is_some() {
                Some(OrderReference {
                    order_id,
                    order_line_id,
                })
            } else {
                None
            },
            delivery_note_reference: if delivery_note_id.is_some()
                || delivery_note_line_id.is_some()
            {
                Some(DeliveryNoteReference {
                    delivery_note_id,
                    delivery_note_line_id,
                })
            } else {
                None
            },
            item_name,
            item_ean_code,
            period_from_date,
            period_to_date,
            invoiced_quantity,
            unit_price_tax_exclusive_amount,
            unit_price_tax_inclusive_amount: None,
            unit_price_tax_amount,
            line_tax_exclusive_amount: None,
            line_tax_inclusive_amount: None,
            line_tax_amount: None,
            classified_tax_category,
        });
    }

    reader.finish()?;
    InvoiceItems::new(invoice_id, first_invoice_line_id, InvoiceLines::new(lines))
        .map_err(|error| reader.invalid_at_current(error.field(), error.message()))
}

struct SequenceReader<'a> {
    fields: Vec<&'a str>,
    position: usize,
}

impl<'a> SequenceReader<'a> {
    fn new(sequence: &'a str) -> Self {
        Self {
            fields: sequence.split('\t').collect(),
            position: 0,
        }
    }

    fn next(&mut self, field: &'static str) -> Result<&'a str> {
        let value =
            self.fields.get(self.position).copied().ok_or_else(|| {
                self.invalid_at_current(field, "sequence ended before this field")
            })?;
        self.position += 1;
        Ok(value)
    }

    fn text(&mut self, field: &'static str) -> Result<String> {
        Ok(self.next(field)?.to_owned())
    }

    fn optional_text(&mut self, field: &'static str) -> Result<Option<String>> {
        let value = self.next(field)?;
        Ok((!value.is_empty()).then(|| value.to_owned()))
    }

    fn positive_count(&mut self, field: &'static str) -> Result<usize> {
        let value = self.next(field)?;
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(self.invalid(field, "must be a positive ASCII integer"));
        }
        value
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| self.invalid(field, "must be a positive ASCII integer"))
    }

    fn optional_date(&mut self, field: &'static str) -> Result<Option<Date>> {
        let value = self.next(field)?;
        if value.is_empty() {
            return Ok(None);
        }
        if value.len() != 8 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(self.invalid(field, "must use YYYYMMDD format"));
        }
        let canonical = format!("{}-{}-{}", &value[..4], &value[4..6], &value[6..]);
        Date::new(canonical)
            .map(Some)
            .map_err(|error| self.invalid(field, error.to_string()))
    }

    fn decimal(&mut self, field: &'static str) -> Result<Decimal> {
        let value = self.next(field)?;
        Decimal::new(value).map_err(|error| self.invalid(field, error.to_string()))
    }

    fn vat_percentage(&mut self, field: &'static str) -> Result<Percentage> {
        let raw = self.next(field)?;
        if raw.trim_start().starts_with('-') {
            return Err(self.invalid(field, "VAT percentage must not be negative"));
        }
        let value = Decimal::new(raw).map_err(|error| self.invalid(field, error.to_string()))?;
        if let Ok(percentage) = Percentage::try_from(value.clone()) {
            return Ok(percentage);
        }
        let deployed = divide_decimal_by_hundred(value.as_str());
        Percentage::new(deployed).map_err(|_| {
            self.invalid(
                field,
                "must be in canonical range [0,1] or deployed percent-point range (1,100]",
            )
        })
    }

    fn finish(&self) -> Result<()> {
        if self.position == self.fields.len() {
            Ok(())
        } else {
            Err(self.invalid_at_current(
                "InvoiceItems",
                format!(
                    "contains {} trailing field(s)",
                    self.fields.len() - self.position
                ),
            ))
        }
    }

    fn invalid(&self, field: &'static str, message: impl Into<String>) -> Error {
        Error::InvalidSequence {
            position: self.position.saturating_sub(1),
            field,
            message: message.into(),
        }
    }

    fn invalid_at_current(&self, field: &'static str, message: impl Into<String>) -> Error {
        Error::InvalidSequence {
            position: self.position,
            field,
            message: message.into(),
        }
    }
}

fn divide_decimal_by_hundred(value: &str) -> String {
    let (integer, fraction) = value.split_once('.').unwrap_or((value, ""));
    let digits = format!("{integer}{fraction}");
    match integer.len() {
        0 => format!("0.00{digits}"),
        1 => format!("0.0{digits}"),
        2 => format!("0.{digits}"),
        length => format!("{}.{}", &digits[..length - 2], &digits[length - 2..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice_items::encode;

    const VERIFIED_PAYLOAD: &str = "400AG00014R2FDVNG9S0VU46DJBOAIN8NDM4OMOTDGO08G2JNI84ME2TMG49NFI4K7NQEANVLOIIIGKKN8F6S1CA6PUPF20GHUQR8QRMMAJRMEB21ACNQG8VLMSBKORCHA3RRJDNEN3ILAR0B7Q7L85RQI8QHOT8G6DD2OGCPMOE81JAADR5JBB68JK8Q1BNQ2A0KA1NIK4V6NOK6A07C00";

    #[test]
    fn decodes_verified_three_line_payload_and_round_trips_semantics() {
        let document = decode(VERIFIED_PAYLOAD).unwrap();
        assert_eq!(document.invoice_id, "INV-MULTI-2026");
        assert_eq!(document.first_invoice_line_id, "1");
        assert_eq!(document.invoice_lines.invoice_line.len(), 3);
        assert_eq!(
            document.invoice_lines.invoice_line[1]
                .item_ean_code
                .as_deref(),
            Some("8581234567890")
        );
        assert_eq!(
            document.invoice_lines.invoice_line[2]
                .unit_price_tax_exclusive_amount
                .as_str(),
            "-20"
        );
        assert_eq!(decode(&encode(&document).unwrap()).unwrap(), document);
    }

    #[test]
    fn rejects_wrong_header_and_field_count() {
        let document = decode(VERIFIED_PAYLOAD).unwrap();
        let sequence = super::super::encode_sequence(&document).unwrap();
        assert!(decode_sequence(&format!("{sequence}\textra")).is_err());

        let mut fields = sequence.split('\t').collect::<Vec<_>>();
        fields[2] = "4";
        assert!(decode_sequence(&fields.join("\t")).is_err());

        let wrong = crate::codec::encode_payload(
            crate::codec::Header {
                by_square_type: 2,
                version: 0,
                document_type: 1,
                reserved: 0,
            },
            &sequence,
        )
        .unwrap();
        assert!(matches!(decode(&wrong), Err(Error::Unsupported(_))));
    }

    #[test]
    fn accepts_deployed_percent_point_vat_but_normalizes_it() {
        let document = decode(VERIFIED_PAYLOAD).unwrap();
        let sequence = super::super::encode_sequence(&document).unwrap();
        let mut fields = sequence.split('\t').collect::<Vec<_>>();
        fields[14] = "20";
        let decoded = decode_sequence(&fields.join("\t")).unwrap();
        assert_eq!(
            decoded.invoice_lines.invoice_line[0]
                .classified_tax_category
                .as_str(),
            "0.2"
        );
    }
}
