use std::str::Split;

use chrono::NaiveDate;

use crate::{
    codec::{self, Header},
    error::{Error, Result},
    pay::{
        encoder,
        models::{
            Amount, BankAccount, BankAccounts, DirectDebitExt, DirectDebitScheme, DirectDebitType,
            Months, Pay, Payment, PaymentOptions, Payments, Periodicity, StandingOrderExt,
        },
    },
};

/// Decode and validate a Base32hex PAY by square payload.
pub fn decode(payload: &str) -> Result<Pay> {
    let decoded = codec::decode_payload(payload)?;
    if decoded.header != Header::PAY {
        return Err(Error::InvalidPayload(format!(
            "expected PAY header 0/0/0/0, got {}/{}/{}/{}",
            decoded.header.by_square_type,
            decoded.header.version,
            decoded.header.document_type,
            decoded.header.reserved
        )));
    }

    decode_sequence(&decoded.sequence)
}

/// Decode and validate an uncompressed tab-delimited PAY sequence.
pub fn decode_sequence(sequence: &str) -> Result<Pay> {
    let mut reader = SequenceReader::new(sequence);
    let invoice_id = optional_string(reader.next("InvoiceID")?);
    let payment_count = parse_usize(&mut reader, "Payments")?;
    if payment_count == 0 {
        return Err(reader.invalid("Payments", "must contain at least one Payment"));
    }

    let mut payments = Vec::new();
    for _ in 0..payment_count {
        payments.push(parse_payment(&mut reader)?);
    }

    // Version 1.1 places beneficiary values after the core data of every
    // payment to preserve compatibility with older bulk-payment decoders.
    for payment in &mut payments {
        payment.beneficiary_name = optional_string(reader.next("BeneficiaryName")?);
        payment.beneficiary_address_line1 =
            optional_string(reader.next("BeneficiaryAddressLine1")?);
        payment.beneficiary_address_line2 =
            optional_string(reader.next("BeneficiaryAddressLine2")?);
    }
    reader.finish()?;

    let pay = Pay {
        invoice_id,
        payments: Payments { payment: payments },
    };
    encoder::encode_sequence_with_limit(&pay, encoder::SequenceLimit::Unbounded).map_err(
        |error| {
            Error::InvalidPayload(format!(
                "decoded PAY sequence violates the data model: {error}"
            ))
        },
    )?;

    Ok(pay)
}

fn parse_payment(reader: &mut SequenceReader<'_>) -> Result<Payment> {
    let classifier = parse_u8(reader, "PaymentOptions")?;
    let payment_options = PaymentOptions::from_classifier(classifier)
        .map_err(|error| reader.invalid("PaymentOptions", error.to_string()))?;
    let amount = parse_amount(reader, "Amount")?;
    let currency_code = reader.next("CurrencyCode")?.to_owned();
    let payment_due_date = parse_date(reader, "PaymentDueDate")?;
    let variable_symbol = optional_string(reader.next("VariableSymbol")?);
    let constant_symbol = optional_string(reader.next("ConstantSymbol")?);
    let specific_symbol = optional_string(reader.next("SpecificSymbol")?);
    let originators_reference_information =
        optional_string(reader.next("OriginatorsReferenceInformation")?);
    let payment_note = optional_string(reader.next("PaymentNote")?);

    let account_count = parse_usize(reader, "BankAccounts")?;
    if account_count == 0 {
        return Err(reader.invalid("BankAccounts", "must contain at least one BankAccount"));
    }
    let mut bank_accounts = Vec::new();
    for _ in 0..account_count {
        bank_accounts.push(BankAccount {
            iban: reader.next("IBAN")?.to_owned(),
            bic: optional_string(reader.next("BIC")?),
        });
    }

    let standing_order_ext = match parse_occurrence(reader, "StandingOrderExt")? {
        false => None,
        true => Some(parse_standing_order(reader)?),
    };
    let direct_debit_ext = match parse_occurrence(reader, "DirectDebitExt")? {
        false => None,
        true => Some(parse_direct_debit(reader)?),
    };

    Ok(Payment {
        payment_options,
        amount,
        currency_code,
        payment_due_date,
        variable_symbol,
        constant_symbol,
        specific_symbol,
        originators_reference_information,
        payment_note,
        bank_accounts: BankAccounts {
            bank_account: bank_accounts,
        },
        standing_order_ext,
        direct_debit_ext,
        beneficiary_name: None,
        beneficiary_address_line1: None,
        beneficiary_address_line2: None,
    })
}

fn parse_standing_order(reader: &mut SequenceReader<'_>) -> Result<StandingOrderExt> {
    let day = parse_optional_u8(reader, "Day")?;
    let month = match parse_optional_u16(reader, "Month")? {
        None => None,
        Some(classifier) => Some(
            Months::from_classifier(classifier)
                .map_err(|error| reader.invalid("Month", error.to_string()))?,
        ),
    };

    let value = reader.next("Periodicity")?;
    let mut characters = value.chars();
    let classifier = characters
        .next()
        .filter(|_| characters.next().is_none())
        .ok_or_else(|| reader.invalid("Periodicity", "must be one classifier character"))?;
    let periodicity = Periodicity::from_classifier(classifier)
        .map_err(|error| reader.invalid("Periodicity", error.to_string()))?;

    Ok(StandingOrderExt {
        day,
        month,
        periodicity,
        last_date: parse_date(reader, "LastDate")?,
    })
}

fn parse_direct_debit(reader: &mut SequenceReader<'_>) -> Result<DirectDebitExt> {
    let scheme_classifier = parse_u8(reader, "DirectDebitScheme")?;
    let direct_debit_scheme = DirectDebitScheme::from_classifier(scheme_classifier)
        .map_err(|error| reader.invalid("DirectDebitScheme", error.to_string()))?;
    let type_classifier = parse_u8(reader, "DirectDebitType")?;
    let direct_debit_type = DirectDebitType::from_classifier(type_classifier)
        .map_err(|error| reader.invalid("DirectDebitType", error.to_string()))?;

    Ok(DirectDebitExt {
        direct_debit_scheme,
        direct_debit_type,
        variable_symbol: optional_string(reader.next("DirectDebitExt.VariableSymbol")?),
        specific_symbol: optional_string(reader.next("DirectDebitExt.SpecificSymbol")?),
        originators_reference_information: optional_string(
            reader.next("DirectDebitExt.OriginatorsReferenceInformation")?,
        ),
        mandate_id: optional_string(reader.next("DirectDebitExt.MandateID")?),
        creditor_id: optional_string(reader.next("DirectDebitExt.CreditorID")?),
        contract_id: optional_string(reader.next("DirectDebitExt.ContractID")?),
        max_amount: parse_amount(reader, "DirectDebitExt.MaxAmount")?,
        valid_till_date: parse_date(reader, "DirectDebitExt.ValidTillDate")?,
    })
}

fn parse_occurrence(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<bool> {
    match reader.next(field)? {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(reader.invalid(field, "occurrence must be 0 or 1")),
    }
}

fn parse_amount(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<Option<Amount>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Amount::new(value)
            .map(Some)
            .map_err(|error| reader.invalid(field, error.to_string()))
    }
}

fn parse_date(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<Option<String>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() != 8 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(reader.invalid(field, "must use YYYYMMDD format"));
    }

    NaiveDate::parse_from_str(value, "%Y%m%d")
        .map(|date| Some(date.format("%Y-%m-%d").to_string()))
        .map_err(|_| reader.invalid(field, format!("{value:?} is not a valid date")))
}

fn parse_optional_u8(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<Option<u8>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        Ok(None)
    } else {
        parse_ascii_integer(value, field, reader.position)
            .and_then(|value| {
                u8::try_from(value).map_err(|_| reader.invalid(field, "integer does not fit in u8"))
            })
            .map(Some)
    }
}

fn parse_optional_u16(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<Option<u16>> {
    let value = reader.next(field)?;
    if value.is_empty() {
        Ok(None)
    } else {
        parse_ascii_integer(value, field, reader.position)
            .and_then(|value| {
                u16::try_from(value)
                    .map_err(|_| reader.invalid(field, "integer does not fit in u16"))
            })
            .map(Some)
    }
}

fn parse_u8(reader: &mut SequenceReader<'_>, field: &'static str) -> Result<u8> {
    let value = parse_usize(reader, field)?;
    u8::try_from(value).map_err(|_| reader.invalid(field, "integer does not fit in u8"))
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

    fn finish(mut self) -> Result<()> {
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
