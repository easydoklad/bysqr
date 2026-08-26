use std::str::Split;

use crate::{
    codec,
    error::{Error, Result},
};

use super::models::{
    Contact, CountryCode, CurrencyCode, CustomerParty, Date, Decimal, DocumentType, Invoice,
    InvoiceData, MonetarySummary, PaymentMeans, Percentage, PostalAddress, SingleInvoiceLine,
    SupplierParty, TaxCategorySummaries, TaxCategorySummary,
};

/// Decode and validate a Base32hex INVOICE by square payload.
pub fn decode(payload: &str) -> Result<Invoice> {
    let decoded = codec::decode_payload(payload)?;
    let header = decoded.header;

    if header.by_square_type != 1 || header.version != 0 || header.reserved != 0 {
        return Err(Error::InvalidPayload(format!(
            "expected INVOICE header type=1, version=0, reserved=0, got type={}, version={}, document type={}, reserved={}",
            header.by_square_type, header.version, header.document_type, header.reserved
        )));
    }

    let document_type = DocumentType::from_classifier(header.document_type).map_err(|error| {
        Error::InvalidPayload(format!(
            "invalid INVOICE document type classifier {}: {error}",
            header.document_type
        ))
    })?;

    decode_sequence(&decoded.sequence, document_type)
}

/// Decode and validate an uncompressed tab-delimited INVOICE sequence.
///
/// `document_type` is carried by the binary envelope and is therefore explicit
/// for callers that already have an uncompressed sequence.
pub fn decode_sequence(sequence: &str, document_type: DocumentType) -> Result<Invoice> {
    let mut reader = SequenceReader::new(sequence);

    let invoice_id = reader.next("InvoiceID")?.to_owned();
    let issue_date = parse_date(&mut reader, "IssueDate")?;
    let tax_point_date = parse_optional_date(&mut reader, "TaxPointDate")?;
    let order_id = optional_string(reader.next("OrderID")?);
    let delivery_note_id = optional_string(reader.next("DeliveryNoteID")?);
    let local_currency_code = parse_currency_code(&mut reader, "LocalCurrencyCode")?;
    let foreign_currency_code = parse_optional_currency_code(&mut reader, "ForeignCurrencyCode")?;
    let curr_rate = parse_optional_decimal(&mut reader, "CurrRate")?;
    let reference_curr_rate = parse_optional_decimal(&mut reader, "ReferenceCurrRate")?;

    let supplier_party = parse_supplier_party(&mut reader)?;
    let customer_party = parse_customer_party(&mut reader)?;
    let number_of_invoice_lines = parse_optional_count(&mut reader, "NumberOfInvoiceLines")?;
    let invoice_description = optional_string(reader.next("InvoiceDescription")?);
    let single_invoice_line = parse_single_invoice_line(&mut reader)?;

    let summary_count = parse_usize(&mut reader, "TaxCategorySummaries")?;
    if summary_count == 0 {
        return Err(reader.invalid(
            "TaxCategorySummaries",
            "must contain at least one TaxCategorySummary",
        ));
    }
    let mut summaries = Vec::new();
    for _ in 0..summary_count {
        summaries.push(parse_tax_category_summary(&mut reader)?);
    }

    let payable_rounding_amount = parse_decimal(&mut reader, "PayableRoundingAmount")?;
    let paid_deposits_amount = parse_decimal(&mut reader, "PaidDepositsAmount")?;
    let payment_means = parse_optional_payment_means(&mut reader)?;
    reader.finish()?;

    let data = InvoiceData {
        invoice_id,
        issue_date,
        tax_point_date,
        order_id,
        delivery_note_id,
        local_currency_code,
        foreign_currency_code,
        curr_rate,
        reference_curr_rate,
        supplier_party,
        customer_party,
        number_of_invoice_lines,
        invoice_description,
        single_invoice_line,
        tax_category_summaries: TaxCategorySummaries {
            tax_category_summary: summaries,
        },
        monetary_summary: MonetarySummary {
            tax_exclusive_amount: None,
            tax_inclusive_amount: None,
            tax_amount: None,
            already_claimed_tax_exclusive_amount: None,
            already_claimed_tax_inclusive_amount: None,
            already_claimed_tax_amount: None,
            difference_tax_exclusive_amount: None,
            difference_tax_inclusive_amount: None,
            difference_tax_amount: None,
            payable_rounding_amount,
            paid_deposits_amount,
            payable_amount: None,
        },
        payment_means,
    };

    Invoice::new(document_type, data).map_err(|error| Error::InvalidSequence {
        position: reader.position,
        field: error.field(),
        message: error.message().to_owned(),
    })
}

fn parse_supplier_party(reader: &mut SequenceReader<'_>) -> Result<SupplierParty> {
    let party_name = reader.next("SupplierParty.PartyName")?.to_owned();
    let company_tax_id = optional_string(reader.next("SupplierParty.CompanyTaxID")?);
    let company_vat_id = optional_string(reader.next("SupplierParty.CompanyVATID")?);
    let company_register_id = optional_string(reader.next("SupplierParty.CompanyRegisterID")?);
    let street_name = reader
        .next("SupplierParty.PostalAddress.StreetName")?
        .to_owned();
    let building_number =
        optional_string(reader.next("SupplierParty.PostalAddress.BuildingNumber")?);
    let city_name = reader
        .next("SupplierParty.PostalAddress.CityName")?
        .to_owned();
    let postal_zone = reader
        .next("SupplierParty.PostalAddress.PostalZone")?
        .to_owned();
    let state = optional_string(reader.next("SupplierParty.PostalAddress.State")?);
    let country = parse_country_code(reader, "SupplierParty.PostalAddress.Country")?;

    let contact_name = optional_string(reader.next("SupplierParty.Contact.Name")?);
    let contact_telephone = optional_string(reader.next("SupplierParty.Contact.Telephone")?);
    let contact_email = optional_string(reader.next("SupplierParty.Contact.EMail")?);
    let contact =
        if contact_name.is_some() || contact_telephone.is_some() || contact_email.is_some() {
            Some(Contact {
                name: contact_name,
                telephone: contact_telephone,
                email: contact_email,
            })
        } else {
            None
        };

    Ok(SupplierParty {
        party_name,
        company_tax_id,
        company_vat_id,
        company_register_id,
        postal_address: PostalAddress {
            street_name,
            building_number,
            city_name,
            postal_zone,
            state,
            country,
        },
        contact,
    })
}

fn parse_customer_party(reader: &mut SequenceReader<'_>) -> Result<CustomerParty> {
    Ok(CustomerParty {
        party_name: reader.next("CustomerParty.PartyName")?.to_owned(),
        company_tax_id: optional_string(reader.next("CustomerParty.CompanyTaxID")?),
        company_vat_id: optional_string(reader.next("CustomerParty.CompanyVATID")?),
        company_register_id: optional_string(reader.next("CustomerParty.CompanyRegisterID")?),
        party_identification: optional_string(reader.next("CustomerParty.PartyIdentification")?),
    })
}

fn parse_single_invoice_line(reader: &mut SequenceReader<'_>) -> Result<Option<SingleInvoiceLine>> {
    let order_line_id = optional_string(reader.next("SingleInvoiceLine.OrderLineID")?);
    let delivery_note_line_id =
        optional_string(reader.next("SingleInvoiceLine.DeliveryNoteLineID")?);
    let item_name = optional_string(reader.next("SingleInvoiceLine.ItemName")?);
    let item_ean_code = optional_string(reader.next("SingleInvoiceLine.ItemEANCode")?);
    let period_from_date = parse_optional_date(reader, "SingleInvoiceLine.PeriodFromDate")?;
    let period_to_date = parse_optional_date(reader, "SingleInvoiceLine.PeriodToDate")?;
    let quantity = reader.next("SingleInvoiceLine.InvoicedQuantity")?;

    let line_present = order_line_id.is_some()
        || delivery_note_line_id.is_some()
        || item_name.is_some()
        || item_ean_code.is_some()
        || period_from_date.is_some()
        || period_to_date.is_some()
        || !quantity.is_empty();
    if !line_present {
        return Ok(None);
    }
    if quantity.is_empty() {
        return Err(reader.invalid(
            "SingleInvoiceLine.InvoicedQuantity",
            "must not be empty when SingleInvoiceLine is present",
        ));
    }
    let invoiced_quantity = Decimal::new(quantity)
        .map_err(|error| reader.invalid("SingleInvoiceLine.InvoicedQuantity", error.to_string()))?;

    Ok(Some(SingleInvoiceLine {
        order_line_id,
        delivery_note_line_id,
        item_name,
        item_ean_code,
        period_from_date,
        period_to_date,
        invoiced_quantity,
        unit_price_tax_exclusive_amount: None,
        unit_price_tax_inclusive_amount: None,
        unit_price_tax_amount: None,
    }))
}

fn parse_tax_category_summary(reader: &mut SequenceReader<'_>) -> Result<TaxCategorySummary> {
    Ok(TaxCategorySummary {
        classified_tax_category: parse_vat_percentage(reader, "ClassifiedTaxCategory")?,
        tax_exclusive_amount: parse_decimal(reader, "TaxExclusiveAmount")?,
        tax_inclusive_amount: None,
        tax_amount: parse_decimal(reader, "TaxAmount")?,
        already_claimed_tax_exclusive_amount: parse_decimal(
            reader,
            "AlreadyClaimedTaxExclusiveAmount",
        )?,
        already_claimed_tax_inclusive_amount: None,
        already_claimed_tax_amount: parse_decimal(reader, "AlreadyClaimedTaxAmount")?,
        difference_tax_exclusive_amount: None,
        difference_tax_inclusive_amount: None,
        difference_tax_amount: None,
    })
}

fn parse_currency_code(
    reader: &mut SequenceReader<'_>,
    field: &'static str,
) -> Result<CurrencyCode> {
    let value = reader.next(field)?;
    CurrencyCode::new(value.to_owned()).map_err(|error| reader.invalid(field, error.to_string()))
}

fn parse_optional_currency_code(
    reader: &mut SequenceReader<'_>,
    field: &'static str,
) -> Result<Option<CurrencyCode>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        Ok(None)
    } else {
        CurrencyCode::new(value.to_owned())
            .map(Some)
            .map_err(|error| reader.invalid(field, error.to_string()))
    }
}

fn parse_country_code(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<CountryCode> {
    let value = reader.next(field)?;
    CountryCode::new(value.to_owned()).map_err(|error| reader.invalid(field, error.to_string()))
}

fn parse_date(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<Date> {
    let value = reader.next(field)?;
    parse_date_value(value, field, reader)
}

fn parse_optional_date(
    reader: &mut SequenceReader<'_>,
    field: &'static str,
) -> Result<Option<Date>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        Ok(None)
    } else {
        parse_date_value(value, field, reader).map(Some)
    }
}

fn parse_date_value(value: &str, field: &'static str, reader: &SequenceReader<'_>) -> Result<Date> {
    if value.len() != 8 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(reader.invalid(field, "must use YYYYMMDD format"));
    }
    let canonical = format!("{}-{}-{}", &value[..4], &value[4..6], &value[6..]);
    Date::new(canonical).map_err(|error| reader.invalid(field, error.to_string()))
}

fn parse_decimal(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<Decimal> {
    let value = reader.next(field)?;
    Decimal::new(value).map_err(|error| reader.invalid(field, error.to_string()))
}

fn parse_optional_decimal(
    reader: &mut SequenceReader<'_>,
    field: &'static str,
) -> Result<Option<Decimal>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Decimal::new(value)
            .map(Some)
            .map_err(|error| reader.invalid(field, error.to_string()))
    }
}

fn parse_vat_percentage(
    reader: &mut SequenceReader<'_>,
    field: &'static str,
) -> Result<Percentage> {
    let raw = reader.next(field)?;
    if raw.trim_start().starts_with('-') {
        return Err(reader.invalid(field, "VAT percentage must not be negative"));
    }

    let value = Decimal::new(raw).map_err(|error| reader.invalid(field, error.to_string()))?;
    if let Ok(percentage) = Percentage::try_from(value.clone()) {
        return Ok(percentage);
    }

    let deployed = divide_decimal_by_hundred(value.as_str());
    Percentage::new(deployed).map_err(|_| {
        reader.invalid(
            field,
            "must be in canonical range [0,1] or deployed percent-point range (1,100]",
        )
    })
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

fn parse_optional_count(
    reader: &mut SequenceReader<'_>,
    field: &'static str,
) -> Result<Option<i64>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        return Ok(None);
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(reader.invalid(field, "must be a non-negative ASCII integer"));
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| reader.invalid(field, "integer is too large"))
}

fn parse_optional_payment_means(reader: &mut SequenceReader<'_>) -> Result<Option<PaymentMeans>> {
    let field = "PaymentMeans";
    let value = reader.next(field)?;
    if value.is_empty() {
        return Ok(None);
    }
    let classifier = parse_ascii_integer(value, field, reader.position)?;
    let classifier = u8::try_from(classifier)
        .map_err(|_| reader.invalid(field, "classifier does not fit in u8"))?;
    PaymentMeans::from_classifier(classifier)
        .map(Some)
        .map_err(|error| reader.invalid(field, error.to_string()))
}

fn parse_usize(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<usize> {
    let value = reader.next(field)?;
    parse_ascii_integer(value, field, reader.position)
}

fn parse_ascii_integer(value: &str, field: &'static str, position: usize) -> Result<usize> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::InvalidSequence {
            position,
            field,
            message: "must be an unsigned ASCII integer".to_owned(),
        });
    }
    value.parse().map_err(|_| Error::InvalidSequence {
        position,
        field,
        message: "integer is too large".to_owned(),
    })
}

fn optional_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

struct SequenceReader<'a> {
    fields: Split<'a, char>,
    position: usize,
}

impl<'a> SequenceReader<'a> {
    fn new(sequence: &'a str) -> Self {
        Self {
            fields: sequence.split('\t'),
            position: 0,
        }
    }

    fn next(&mut self, field: &'static str) -> Result<&'a str> {
        let position = self.position + 1;
        let value = self.fields.next().ok_or_else(|| Error::InvalidSequence {
            position,
            field,
            message: "field is missing".to_owned(),
        })?;
        self.position = position;
        Ok(value)
    }

    fn finish(&mut self) -> Result<()> {
        if self.fields.next().is_some() {
            return Err(Error::InvalidSequence {
                position: self.position + 1,
                field: "document",
                message: "contains unexpected trailing fields".to_owned(),
            });
        }
        Ok(())
    }

    fn invalid(&self, field: &'static str, message: impl Into<String>) -> Error {
        Error::InvalidSequence {
            position: self.position,
            field,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, decode_sequence};
    use crate::{
        codec::{self, Header},
        error::Error,
        invoice::{
            encode,
            models::{DocumentType, Invoice},
        },
    };

    const FORSYS_PAYLOAD: &str = include_str!(
        "../../tests/fixtures/invoice/valid-interoperability-offline-forsys-legacy.payload.txt"
    );
    const FORSYS_SEQUENCE: &str = include_str!(
        "../../tests/fixtures/invoice/valid-interoperability-offline-forsys-legacy.sequence.tsv"
    );
    const OFFICIAL_PAYLOAD: &str = include_str!(
        "../../tests/fixtures/invoice/valid-interoperability-offline-official-current.payload.txt"
    );

    fn without_terminal_line_ending(value: &str) -> &str {
        let value = value.strip_suffix('\n').unwrap_or(value);
        value.strip_suffix('\r').unwrap_or(value)
    }

    fn fixture_fields() -> Vec<String> {
        without_terminal_line_ending(FORSYS_SEQUENCE)
            .split('\t')
            .map(str::to_owned)
            .collect()
    }

    fn decode_fields(fields: &[String]) -> crate::error::Result<Invoice> {
        decode_sequence(&fields.join("\t"), DocumentType::Invoice)
    }

    fn assert_computed_fields_are_none(invoice: &Invoice) {
        let line = invoice.data.single_invoice_line.as_ref().unwrap();
        assert!(line.unit_price_tax_exclusive_amount.is_none());
        assert!(line.unit_price_tax_inclusive_amount.is_none());
        assert!(line.unit_price_tax_amount.is_none());

        let summary = &invoice.data.tax_category_summaries.tax_category_summary[0];
        assert!(summary.tax_inclusive_amount.is_none());
        assert!(summary.already_claimed_tax_inclusive_amount.is_none());
        assert!(summary.difference_tax_exclusive_amount.is_none());
        assert!(summary.difference_tax_inclusive_amount.is_none());
        assert!(summary.difference_tax_amount.is_none());

        let monetary = &invoice.data.monetary_summary;
        assert!(monetary.tax_exclusive_amount.is_none());
        assert!(monetary.tax_inclusive_amount.is_none());
        assert!(monetary.tax_amount.is_none());
        assert!(monetary.already_claimed_tax_exclusive_amount.is_none());
        assert!(monetary.already_claimed_tax_inclusive_amount.is_none());
        assert!(monetary.already_claimed_tax_amount.is_none());
        assert!(monetary.difference_tax_exclusive_amount.is_none());
        assert!(monetary.difference_tax_inclusive_amount.is_none());
        assert!(monetary.difference_tax_amount.is_none());
        assert!(monetary.payable_amount.is_none());
    }

    #[test]
    fn decodes_both_committed_payloads_and_normalizes_vat() {
        let expected_header = Header {
            by_square_type: 1,
            version: 0,
            document_type: 0,
            reserved: 0,
        };

        for (payload, expected_id, transported_vat) in [
            (FORSYS_PAYLOAD, "201300001", "0.2"),
            (OFFICIAL_PAYLOAD, "INV-2025-0001", "20"),
        ] {
            let payload = without_terminal_line_ending(payload);
            let envelope = codec::decode_payload(payload).unwrap();
            assert_eq!(envelope.header, expected_header);
            let fields = envelope.sequence.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 45);
            assert_eq!(fields[37], transported_vat);

            let invoice = decode(payload).unwrap();
            assert_eq!(invoice.document_type, DocumentType::Invoice);
            assert_eq!(invoice.data.invoice_id, expected_id);
            assert_eq!(
                invoice.data.tax_category_summaries.tax_category_summary[0]
                    .classified_tax_category
                    .as_str(),
                "0.2"
            );
            assert_computed_fields_are_none(&invoice);
        }
    }

    #[test]
    fn decodes_all_five_document_type_classifiers() {
        let sequence = without_terminal_line_ending(FORSYS_SEQUENCE);
        for classifier in 0..=4 {
            let expected = DocumentType::from_classifier(classifier).unwrap();
            let payload = codec::encode_payload(
                Header {
                    by_square_type: 1,
                    version: 0,
                    document_type: classifier,
                    reserved: 0,
                },
                sequence,
            )
            .unwrap();
            assert_eq!(decode(&payload).unwrap().document_type, expected);
        }
    }

    #[test]
    fn encoder_and_decoder_round_trip_known_vectors_semantically() {
        for payload in [FORSYS_PAYLOAD, OFFICIAL_PAYLOAD] {
            let invoice = decode(without_terminal_line_ending(payload)).unwrap();
            let canonical_payload = encode(&invoice).unwrap();

            assert_eq!(decode(&canonical_payload).unwrap(), invoice);
        }
    }

    #[test]
    fn normalizes_decimal_percent_points_without_floating_point() {
        let mut fields = fixture_fields();
        fields[37] = "20.5".to_owned();
        let invoice = decode_fields(&fields).unwrap();
        assert_eq!(
            invoice.data.tax_category_summaries.tax_category_summary[0]
                .classified_tax_category
                .as_str(),
            "0.205"
        );

        fields[37] = "100".to_owned();
        let invoice = decode_fields(&fields).unwrap();
        assert_eq!(
            invoice.data.tax_category_summaries.tax_category_summary[0]
                .classified_tax_category
                .as_str(),
            "1"
        );
    }

    #[test]
    fn rejects_malformed_counts_and_wrong_field_counts() {
        for value in ["", "-1", "one", "184467440737095516160"] {
            let mut fields = fixture_fields();
            fields[36] = value.to_owned();
            assert!(matches!(
                decode_fields(&fields),
                Err(Error::InvalidSequence { .. })
            ));
        }

        let mut fields = fixture_fields();
        fields[36] = "0".to_owned();
        assert!(matches!(
            decode_fields(&fields),
            Err(Error::InvalidSequence { .. })
        ));

        let mut fields = fixture_fields();
        fields[36] = "2".to_owned();
        assert!(matches!(
            decode_fields(&fields),
            Err(Error::InvalidSequence { .. })
        ));

        let mut fields = fixture_fields();
        fields.pop();
        assert!(matches!(
            decode_fields(&fields),
            Err(Error::InvalidSequence { .. })
        ));

        let mut fields = fixture_fields();
        fields.push("trailing".to_owned());
        assert!(matches!(
            decode_fields(&fields),
            Err(Error::InvalidSequence { .. })
        ));
    }

    #[test]
    fn rejects_invalid_dates_decimals_vat_and_payment_classifier() {
        for (index, value) in [
            (1, "20250230"),
            (7, "not-a-decimal"),
            (37, "-0.2"),
            (37, "100.1"),
            (44, "0"),
            (44, "128"),
        ] {
            let mut fields = fixture_fields();
            fields[index] = value.to_owned();
            assert!(matches!(
                decode_fields(&fields),
                Err(Error::InvalidSequence { .. })
            ));
        }
    }

    #[test]
    fn rejects_invoice_line_choice_violations() {
        let mut both_choices = fixture_fields();
        both_choices[27] = "1".to_owned();
        assert!(matches!(
            decode_fields(&both_choices),
            Err(Error::InvalidSequence { .. })
        ));

        let mut neither_choice = fixture_fields();
        neither_choice[31] = String::new();
        neither_choice[35] = String::new();
        assert!(matches!(
            decode_fields(&neither_choice),
            Err(Error::InvalidSequence { .. })
        ));

        let mut invalid_item_choice = fixture_fields();
        invalid_item_choice[32] = "1234567890123".to_owned();
        assert!(matches!(
            decode_fields(&invalid_item_choice),
            Err(Error::InvalidSequence { .. })
        ));
    }

    #[test]
    fn rejects_non_invoice_headers_and_unknown_document_types() {
        let sequence = without_terminal_line_ending(FORSYS_SEQUENCE);
        for header in [
            Header {
                by_square_type: 0,
                version: 0,
                document_type: 0,
                reserved: 0,
            },
            Header {
                by_square_type: 1,
                version: 1,
                document_type: 0,
                reserved: 0,
            },
            Header {
                by_square_type: 1,
                version: 0,
                document_type: 0,
                reserved: 1,
            },
            Header {
                by_square_type: 1,
                version: 0,
                document_type: 5,
                reserved: 0,
            },
        ] {
            let payload = codec::encode_payload(header, sequence).unwrap();
            assert!(matches!(decode(&payload), Err(Error::InvalidPayload(_))));
        }
    }
}
