use chrono::NaiveDate;

use crate::{
    codec::{self, Header},
    error::{Error, Result},
    models::{BankAccount, Pay, Payment, PaymentOption, Periodicity, StandingOrderExt},
};

pub const MAX_SEQUENCE_CHARACTERS: usize = 550;

/// Encode a PAY by square document into its Base32hex QR payload.
pub fn encode(pay: &Pay) -> Result<String> {
    codec::encode_payload(Header::PAY, &encode_sequence(pay)?)
}

/// Serialize a PAY document into the tab-delimited sequence defined by the
/// by-square specification.
///
/// This is public mainly to make conformance testing and integrations that
/// inspect the uncompressed data straightforward.
pub fn encode_sequence(pay: &Pay) -> Result<String> {
    let payments = &pay.payments.payment;
    if payments.is_empty() {
        return Err(Error::invalid(
            "Payments",
            "must contain at least one Payment",
        ));
    }

    let mut fields = Vec::new();
    fields.push(optional_text("InvoiceID", pay.invoice_id.as_deref(), 10)?);
    fields.push(payments.len().to_string());

    // Appendix E of the specification places every beneficiary tuple after
    // the core data of every payment, rather than inside each payment.
    for payment in payments {
        append_payment_core(&mut fields, payment)?;
    }
    for payment in payments {
        fields.push(optional_text(
            "BeneficiaryName",
            payment.beneficiary_name.as_deref(),
            140,
        )?);
        fields.push(optional_text(
            "BeneficiaryAddressLine1",
            payment.beneficiary_address_line1.as_deref(),
            70,
        )?);
        fields.push(optional_text(
            "BeneficiaryAddressLine2",
            payment.beneficiary_address_line2.as_deref(),
            70,
        )?);
    }

    let sequence = fields.join("\t");
    let character_count = sequence.chars().count();
    if character_count > MAX_SEQUENCE_CHARACTERS {
        return Err(Error::SequenceTooLong {
            actual: character_count,
            maximum: MAX_SEQUENCE_CHARACTERS,
        });
    }

    Ok(sequence)
}

fn append_payment_core(fields: &mut Vec<String>, payment: &Payment) -> Result<()> {
    let options = payment.payment_options;
    let has_standing_order = options.contains(PaymentOption::StandingOrder);
    let has_direct_debit = options.contains(PaymentOption::DirectDebit);

    if has_direct_debit || payment.direct_debit_ext.is_some() {
        return Err(Error::Unsupported(
            "direct-debit encoding is not implemented yet".to_owned(),
        ));
    }

    let standing_order = match (has_standing_order, payment.standing_order_ext.as_ref()) {
        (false, None) => None,
        (true, Some(extension)) => Some(extension),
        (true, None) => {
            return Err(Error::invalid(
                "StandingOrderExt",
                "is required when PaymentOptions contains standingorder",
            ));
        }
        (false, Some(_)) => {
            return Err(Error::invalid(
                "PaymentOptions",
                "must contain standingorder when StandingOrderExt is present",
            ));
        }
    };

    fields.push(options.classifier().to_string());

    let amount = match &payment.amount {
        None => String::new(),
        Some(amount) if amount.is_zero() => {
            return Err(Error::invalid("Amount", "must be greater than zero"));
        }
        Some(amount) if amount.as_str().chars().count() > 15 => {
            return Err(Error::invalid(
                "Amount",
                "must contain no more than 15 characters",
            ));
        }
        Some(amount) => amount.to_string(),
    };
    fields.push(amount);

    let currency = sanitized(&payment.currency_code);
    if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(Error::invalid(
            "CurrencyCode",
            "must contain exactly three ASCII uppercase letters",
        ));
    }
    fields.push(currency);

    fields.push(match payment.payment_due_date.as_deref() {
        Some(value) => valid_date("PaymentDueDate", value)?,
        None => String::new(),
    });

    let has_symbols = payment.variable_symbol.is_some()
        || payment.constant_symbol.is_some()
        || payment.specific_symbol.is_some();
    if has_symbols && payment.originators_reference_information.is_some() {
        return Err(Error::invalid(
            "payment reference",
            "OriginatorsReferenceInformation cannot be combined with payment symbols",
        ));
    }

    fields.push(optional_digits(
        "VariableSymbol",
        payment.variable_symbol.as_deref(),
        10,
    )?);
    fields.push(optional_digits(
        "ConstantSymbol",
        payment.constant_symbol.as_deref(),
        4,
    )?);
    fields.push(optional_digits(
        "SpecificSymbol",
        payment.specific_symbol.as_deref(),
        10,
    )?);
    fields.push(optional_text(
        "OriginatorsReferenceInformation",
        payment.originators_reference_information.as_deref(),
        35,
    )?);
    fields.push(optional_text(
        "PaymentNote",
        payment.payment_note.as_deref(),
        140,
    )?);

    let accounts = &payment.bank_accounts.bank_account;
    if accounts.is_empty() {
        return Err(Error::invalid(
            "BankAccounts",
            "must contain at least one BankAccount",
        ));
    }
    fields.push(accounts.len().to_string());
    for account in accounts {
        append_bank_account(fields, account)?;
    }

    append_standing_order(fields, standing_order)?;

    // DirectDebitExt remains absent until its encoder is implemented.
    fields.push("0".to_owned());

    Ok(())
}

fn append_standing_order(
    fields: &mut Vec<String>,
    standing_order: Option<&StandingOrderExt>,
) -> Result<()> {
    let Some(standing_order) = standing_order else {
        fields.push("0".to_owned());
        return Ok(());
    };

    let day = match standing_order.day {
        None => String::new(),
        Some(_) if standing_order.periodicity == Periodicity::Daily => {
            return Err(Error::invalid(
                "Day",
                "must not be specified for Daily periodicity",
            ));
        }
        Some(day)
            if matches!(
                standing_order.periodicity,
                Periodicity::Weekly | Periodicity::Biweekly
            ) && !(1..=7).contains(&day) =>
        {
            return Err(Error::invalid(
                "Day",
                "must be between 1 and 7 for Weekly or Biweekly periodicity",
            ));
        }
        Some(day) if !(1..=31).contains(&day) => {
            return Err(Error::invalid("Day", "must be between 1 and 31"));
        }
        Some(day) => day.to_string(),
    };

    let month = match standing_order.month {
        None => String::new(),
        Some(_) if !standing_order.periodicity.allows_months() => {
            return Err(Error::invalid(
                "Month",
                "is allowed only for Weekly, Biweekly, Monthly, or Bimonthly periodicity",
            ));
        }
        Some(months) => months.classifier().to_string(),
    };

    fields.push("1".to_owned());
    fields.push(day);
    fields.push(month);
    fields.push(standing_order.periodicity.classifier().to_string());
    fields.push(match standing_order.last_date.as_deref() {
        Some(value) => valid_date("LastDate", value)?,
        None => String::new(),
    });

    Ok(())
}

fn append_bank_account(fields: &mut Vec<String>, account: &BankAccount) -> Result<()> {
    let iban = sanitized(&account.iban);
    let bytes = iban.as_bytes();
    let valid_iban = (4..=34).contains(&bytes.len())
        && bytes[..2].iter().all(u8::is_ascii_uppercase)
        && bytes[2..4].iter().all(u8::is_ascii_digit)
        && bytes[4..]
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
    if !valid_iban {
        return Err(Error::invalid(
            "IBAN",
            "must match [A-Z]{2}[0-9]{2}[A-Z0-9]{0,30}",
        ));
    }
    fields.push(iban);

    let bic = match account.bic.as_deref() {
        None => String::new(),
        Some(value) => {
            let value = sanitized(value);
            let bytes = value.as_bytes();
            let valid_bic = matches!(bytes.len(), 8 | 11)
                && bytes[..6].iter().all(u8::is_ascii_uppercase)
                && bytes[6..]
                    .iter()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
            if !valid_bic {
                return Err(Error::invalid(
                    "BIC",
                    "must match [A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?",
                ));
            }
            value
        }
    };
    fields.push(bic);

    Ok(())
}

fn valid_date(field: &'static str, value: &str) -> Result<String> {
    let value = sanitized(value);
    let parsed = match value.len() {
        8 => NaiveDate::parse_from_str(&value, "%Y%m%d"),
        10 => NaiveDate::parse_from_str(&value, "%Y-%m-%d"),
        _ => {
            return Err(Error::invalid(
                field,
                "must use YYYY-MM-DD or YYYYMMDD format",
            ));
        }
    };

    parsed
        .map(|date| date.format("%Y%m%d").to_string())
        .map_err(|_| Error::invalid(field, format!("{value:?} is not a valid date")))
}

fn optional_digits(field: &'static str, value: Option<&str>, maximum: usize) -> Result<String> {
    let value = optional_text(field, value, maximum)?;
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::invalid(
            field,
            format!("must contain at most {maximum} ASCII digits"),
        ));
    }
    Ok(value)
}

fn optional_text(field: &'static str, value: Option<&str>, maximum: usize) -> Result<String> {
    let value = value.map(sanitized).unwrap_or_default();
    let actual = value.chars().count();
    if actual > maximum {
        return Err(Error::invalid(
            field,
            format!("contains {actual} characters; the maximum is {maximum}"),
        ));
    }
    Ok(value)
}

fn sanitized(value: &str) -> String {
    value.replace('\t', " ")
}
