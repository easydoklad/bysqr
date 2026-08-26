//! INVOICE by square sequence and payload encoding.

use super::models::{Date, Invoice};
use crate::{
    codec::{self, Header},
    error::{Error, Result},
};

/// Maximum number of Unicode characters in an INVOICE QR sequence.
pub const MAX_SEQUENCE_CHARACTERS: usize = 550;

/// Length policy for the uncompressed INVOICE sequence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SequenceLimit {
    /// Enforce the 550-character limit required for a reliably readable QR code.
    #[default]
    QrCode,
    /// Disable the QR-oriented character limit for non-QR transports.
    ///
    /// The binary envelope still has its protocol-level 16-bit uncompressed
    /// byte limit.
    Unbounded,
}

/// Encode an INVOICE by square document into its Base32hex QR payload.
pub fn encode(invoice: &Invoice) -> Result<String> {
    encode_with_limit(invoice, SequenceLimit::QrCode)
}

/// Encode an INVOICE document using an explicit sequence-length policy.
pub fn encode_with_limit(invoice: &Invoice, limit: SequenceLimit) -> Result<String> {
    let header = Header {
        by_square_type: 1,
        version: 0,
        document_type: invoice.document_type.classifier(),
        reserved: 0,
    };
    codec::encode_payload(header, &encode_sequence_with_limit(invoice, limit)?)
}

/// Serialize an INVOICE document into the specification's tab-delimited sequence.
pub fn encode_sequence(invoice: &Invoice) -> Result<String> {
    encode_sequence_with_limit(invoice, SequenceLimit::QrCode)
}

/// Serialize an INVOICE document with an explicit sequence-length policy.
///
/// `SequenceLimit::Unbounded` is intended for non-QR transports. It only
/// disables the QR-oriented character limit; [`codec::encode_payload`] still
/// enforces the protocol's uncompressed byte limit when a payload is produced.
pub fn encode_sequence_with_limit(invoice: &Invoice, limit: SequenceLimit) -> Result<String> {
    invoice
        .validate()
        .map_err(|error| Error::invalid(error.field(), error.message()))?;

    let data = &invoice.data;
    let summaries = &data.tax_category_summaries.tax_category_summary;
    let mut fields = Vec::with_capacity(40 + 5 * summaries.len());

    fields.push(sanitized(&data.invoice_id));
    fields.push(compact_date(&data.issue_date));
    fields.push(optional_date(data.tax_point_date.as_ref()));
    fields.push(optional_text(data.order_id.as_deref()));
    fields.push(optional_text(data.delivery_note_id.as_deref()));
    fields.push(data.local_currency_code.as_str().to_owned());
    fields.push(
        data.foreign_currency_code
            .as_ref()
            .map_or_else(String::new, |value| value.as_str().to_owned()),
    );
    fields.push(
        data.curr_rate
            .as_ref()
            .map_or_else(String::new, ToString::to_string),
    );
    fields.push(
        data.reference_curr_rate
            .as_ref()
            .map_or_else(String::new, ToString::to_string),
    );

    let supplier = &data.supplier_party;
    fields.push(sanitized(&supplier.party_name));
    fields.push(optional_text(supplier.company_tax_id.as_deref()));
    fields.push(optional_text(supplier.company_vat_id.as_deref()));
    fields.push(optional_text(supplier.company_register_id.as_deref()));

    let address = &supplier.postal_address;
    fields.push(sanitized(&address.street_name));
    fields.push(optional_text(address.building_number.as_deref()));
    fields.push(sanitized(&address.city_name));
    fields.push(sanitized(&address.postal_zone));
    fields.push(optional_text(address.state.as_deref()));
    fields.push(address.country.as_str().to_owned());

    let contact = supplier.contact.as_ref();
    fields.push(optional_text(
        contact.and_then(|value| value.name.as_deref()),
    ));
    fields.push(optional_text(
        contact.and_then(|value| value.telephone.as_deref()),
    ));
    fields.push(optional_text(
        contact.and_then(|value| value.email.as_deref()),
    ));

    let customer = &data.customer_party;
    fields.push(sanitized(&customer.party_name));
    fields.push(optional_text(customer.company_tax_id.as_deref()));
    fields.push(optional_text(customer.company_vat_id.as_deref()));
    fields.push(optional_text(customer.company_register_id.as_deref()));
    fields.push(optional_text(customer.party_identification.as_deref()));

    fields.push(
        data.number_of_invoice_lines
            .map_or_else(String::new, |value| value.to_string()),
    );
    fields.push(optional_text(data.invoice_description.as_deref()));

    if let Some(line) = &data.single_invoice_line {
        fields.push(optional_text(line.order_line_id.as_deref()));
        fields.push(optional_text(line.delivery_note_line_id.as_deref()));
        fields.push(optional_text(line.item_name.as_deref()));
        fields.push(optional_text(line.item_ean_code.as_deref()));
        fields.push(optional_date(line.period_from_date.as_ref()));
        fields.push(optional_date(line.period_to_date.as_ref()));
        fields.push(line.invoiced_quantity.to_string());
    } else {
        fields.extend(std::iter::repeat_n(String::new(), 7));
    }

    fields.push(summaries.len().to_string());
    for summary in summaries {
        fields.push(summary.classified_tax_category.as_str().to_owned());
        fields.push(summary.tax_exclusive_amount.to_string());
        fields.push(summary.tax_amount.to_string());
        fields.push(summary.already_claimed_tax_exclusive_amount.to_string());
        fields.push(summary.already_claimed_tax_amount.to_string());
    }

    fields.push(data.monetary_summary.payable_rounding_amount.to_string());
    fields.push(data.monetary_summary.paid_deposits_amount.to_string());
    fields.push(
        data.payment_means
            .map_or_else(String::new, |value| value.classifier().to_string()),
    );

    debug_assert_eq!(fields.len(), 40 + 5 * summaries.len());
    let sequence = fields.join("\t");
    let character_count = sequence.chars().count();
    if limit == SequenceLimit::QrCode && character_count > MAX_SEQUENCE_CHARACTERS {
        return Err(Error::SequenceTooLong {
            actual: character_count,
            maximum: MAX_SEQUENCE_CHARACTERS,
        });
    }

    Ok(sequence)
}

fn compact_date(date: &Date) -> String {
    date.as_str()
        .chars()
        .filter(|character| *character != '-')
        .collect()
}

fn optional_date(date: Option<&Date>) -> String {
    date.map_or_else(String::new, compact_date)
}

fn optional_text(value: Option<&str>) -> String {
    value.map_or_else(String::new, sanitized)
}

fn sanitized(value: &str) -> String {
    value.replace('\t', " ")
}

#[cfg(test)]
mod tests {
    use super::{
        encode, encode_sequence, encode_sequence_with_limit, encode_with_limit, SequenceLimit,
        MAX_SEQUENCE_CHARACTERS,
    };
    use crate::{
        codec::{decode_payload, Header},
        error::Error,
        invoice::models::{try_deserialize_invoice, Decimal, DocumentType, Invoice, Percentage},
    };
    use serde_json::json;

    const LEGACY_SEQUENCE: &str = include_str!(
        "../../tests/fixtures/invoice/valid-interoperability-offline-forsys-legacy.sequence.tsv"
    );

    fn decimal(value: &str) -> Decimal {
        Decimal::new(value).unwrap()
    }

    fn legacy_invoice() -> Invoice {
        let source = json!({
            "DocumentType": "Invoice",
            "InvoiceID": "201300001",
            "IssueDate": "2013-02-27",
            "TaxPointDate": "2013-02-27",
            "LocalCurrencyCode": "EUR",
            "SupplierParty": {
                "PartyName": "Forsys a. s.",
                "CompanyTaxID": "2022683003",
                "CompanyVATID": "SK2022683003",
                "CompanyRegisterID": "44232730",
                "PostalAddress": {
                    "StreetName": "Zochova",
                    "BuildingNumber": "6",
                    "CityName": "Bratislava",
                    "PostalZone": "81103",
                    "Country": "SVK"
                },
                "Contact": { "EMail": "info@bysquare.com" }
            },
            "CustomerParty": {
                "PartyName": "Slovenská banková asociácia",
                "CompanyTaxID": "2020809978",
                "CompanyVATID": "SK2020809978",
                "CompanyRegisterID": "30813182"
            },
            "SingleInvoiceLine": {
                "ItemName": "Vzorová faktúra pre štandard by square",
                "InvoicedQuantity": "1"
            },
            "TaxCategorySummaries": {
                "TaxCategorySummary": [{
                    "ClassifiedTaxCategory": "0.2",
                    "TaxExclusiveAmount": "1",
                    "TaxAmount": "0.2",
                    "AlreadyClaimedTaxExclusiveAmount": "0",
                    "AlreadyClaimedTaxAmount": "0"
                }]
            },
            "MonetarySummary": {
                "PayableRoundingAmount": "0",
                "PaidDepositsAmount": "0"
            },
            "PaymentMeans": "moneyTransfer"
        });
        try_deserialize_invoice(&source.to_string()).unwrap()
    }

    fn committed_legacy_sequence() -> &'static str {
        let sequence = LEGACY_SEQUENCE
            .strip_suffix('\n')
            .unwrap_or(LEGACY_SEQUENCE);
        sequence.strip_suffix('\r').unwrap_or(sequence)
    }

    #[test]
    fn uses_exact_fixed_positions_and_five_fields_per_tax_summary() {
        let mut invoice = legacy_invoice();
        let sequence = encode_sequence(&invoice).unwrap();
        assert_eq!(sequence, committed_legacy_sequence());

        let fields = sequence.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 45);
        assert_eq!(fields[0], "201300001");
        assert_eq!(fields[1], "20130227");
        assert_eq!(fields[9], "Forsys a. s.");
        assert_eq!(fields[22], "Slovenská banková asociácia");
        assert_eq!(
            &fields[29..36],
            [
                "",
                "",
                "Vzorová faktúra pre štandard by square",
                "",
                "",
                "",
                "1"
            ]
        );
        assert_eq!(&fields[36..42], ["1", "0.2", "1", "0.2", "0", "0"]);
        assert_eq!(&fields[42..45], ["0", "0", "1"]);

        let mut second = invoice.data.tax_category_summaries.tax_category_summary[0].clone();
        second.classified_tax_category = Percentage::new("0.1").unwrap();
        second.tax_exclusive_amount = decimal("2");
        second.tax_amount = decimal("0.2");
        second.already_claimed_tax_exclusive_amount = decimal("1");
        second.already_claimed_tax_amount = decimal("0.1");
        invoice
            .data
            .tax_category_summaries
            .tax_category_summary
            .push(second);

        let sequence = encode_sequence(&invoice).unwrap();
        let fields = sequence.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 50);
        assert_eq!(fields[36], "2");
        assert_eq!(&fields[37..42], ["0.2", "1", "0.2", "0", "0"]);
        assert_eq!(&fields[42..47], ["0.1", "2", "0.2", "1", "0.1"]);
        assert_eq!(&fields[47..50], ["0", "0", "1"]);
    }

    #[test]
    fn omits_every_computed_model_field() {
        let mut invoice = legacy_invoice();
        let expected = encode_sequence(&invoice).unwrap();
        let computed = || Some(decimal("987654321.123"));

        let line = invoice.data.single_invoice_line.as_mut().unwrap();
        line.unit_price_tax_exclusive_amount = computed();
        line.unit_price_tax_inclusive_amount = computed();
        line.unit_price_tax_amount = computed();

        let tax = &mut invoice.data.tax_category_summaries.tax_category_summary[0];
        tax.tax_inclusive_amount = computed();
        tax.already_claimed_tax_inclusive_amount = computed();
        tax.difference_tax_exclusive_amount = computed();
        tax.difference_tax_inclusive_amount = computed();
        tax.difference_tax_amount = computed();

        let monetary = &mut invoice.data.monetary_summary;
        monetary.tax_exclusive_amount = computed();
        monetary.tax_inclusive_amount = computed();
        monetary.tax_amount = computed();
        monetary.already_claimed_tax_exclusive_amount = computed();
        monetary.already_claimed_tax_inclusive_amount = computed();
        monetary.already_claimed_tax_amount = computed();
        monetary.difference_tax_exclusive_amount = computed();
        monetary.difference_tax_inclusive_amount = computed();
        monetary.difference_tax_amount = computed();
        monetary.payable_amount = computed();

        assert_eq!(encode_sequence(&invoice).unwrap(), expected);
    }

    #[test]
    fn replaces_user_tabs_in_textual_fields_with_spaces() {
        let mut invoice = legacy_invoice();
        invoice.data.invoice_id = "left\tright".to_owned();
        invoice.data.supplier_party.party_name = "supplier\tname".to_owned();
        invoice.data.supplier_party.postal_address.street_name = "main\tstreet".to_owned();
        invoice.data.supplier_party.contact.as_mut().unwrap().name =
            Some("contact\tname".to_owned());
        invoice.data.customer_party.party_name = "customer\tname".to_owned();
        invoice.data.single_invoice_line.as_mut().unwrap().item_name =
            Some("item\tname".to_owned());

        let sequence = encode_sequence(&invoice).unwrap();
        let fields = sequence.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 45);
        assert_eq!(fields[0], "left right");
        assert_eq!(fields[9], "supplier name");
        assert_eq!(fields[13], "main street");
        assert_eq!(fields[19], "contact name");
        assert_eq!(fields[22], "customer name");
        assert_eq!(fields[31], "item name");
    }

    #[test]
    fn enforces_550_unicode_characters_without_truncating_unbounded_sequences() {
        let mut invoice = legacy_invoice();
        invoice.data.invoice_id.clear();
        let baseline = encode_sequence_with_limit(&invoice, SequenceLimit::Unbounded).unwrap();
        let fill = MAX_SEQUENCE_CHARACTERS - baseline.chars().count();
        invoice.data.invoice_id = "Ž".repeat(fill);
        assert!(!invoice.advisory_diagnostics().is_empty());

        let sequence = encode_sequence(&invoice).unwrap();
        assert_eq!(sequence.chars().count(), MAX_SEQUENCE_CHARACTERS);

        invoice.data.invoice_id.push('Ž');
        assert!(matches!(
            encode_sequence(&invoice),
            Err(Error::SequenceTooLong {
                actual: 551,
                maximum: 550
            })
        ));
        assert!(matches!(
            encode(&invoice),
            Err(Error::SequenceTooLong {
                actual: 551,
                maximum: 550
            })
        ));

        let unbounded = encode_sequence_with_limit(&invoice, SequenceLimit::Unbounded).unwrap();
        assert_eq!(unbounded.chars().count(), 551);
        assert_eq!(
            unbounded.split('\t').next().unwrap().chars().count(),
            fill + 1
        );
        let payload = encode_with_limit(&invoice, SequenceLimit::Unbounded).unwrap();
        assert_eq!(decode_payload(&payload).unwrap().sequence, unbounded);
    }

    #[test]
    fn unbounded_sequence_defers_the_protocol_byte_limit_to_the_codec() {
        let mut invoice = legacy_invoice();
        invoice.data.invoice_id = "x".repeat(u16::MAX as usize);

        let sequence = encode_sequence_with_limit(&invoice, SequenceLimit::Unbounded).unwrap();
        assert!(sequence.len() > u16::MAX as usize);
        assert!(matches!(
            encode_with_limit(&invoice, SequenceLimit::Unbounded),
            Err(Error::PayloadTooLong(_))
        ));
    }

    #[test]
    fn encodes_all_five_document_type_header_classifiers() {
        let mut invoice = legacy_invoice();
        for (classifier, document_type) in [
            (0, DocumentType::Invoice),
            (1, DocumentType::ProformaInvoice),
            (2, DocumentType::CreditNote),
            (3, DocumentType::DebitNote),
            (4, DocumentType::AdvanceInvoice),
        ] {
            invoice.document_type = document_type;
            let payload = encode(&invoice).unwrap();
            assert_eq!(
                decode_payload(&payload).unwrap().header,
                Header {
                    by_square_type: 1,
                    version: 0,
                    document_type: classifier,
                    reserved: 0,
                }
            );
        }
    }

    #[test]
    fn rejects_hard_model_rule_violations_but_not_advisories() {
        let mut invoice = legacy_invoice();
        invoice.data.curr_rate = Some(decimal("1"));
        assert!(matches!(
            encode_sequence(&invoice),
            Err(Error::InvalidInput {
                field: "ForeignCurrencyCode",
                ..
            })
        ));
    }
}
