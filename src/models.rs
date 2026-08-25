use std::fmt;

use serde::{
    de::{self, IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::error::{Error, Result};

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Pay {
    #[serde(default)]
    #[serde(rename = "InvoiceID")]
    pub invoice_id: Option<String>,
    pub payments: Payments,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Payments {
    #[serde(default)]
    pub payment: Vec<Payment>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Payment {
    pub payment_options: String,
    #[serde(default)]
    pub amount: Option<Amount>,
    pub currency_code: String,
    #[serde(default)]
    pub payment_due_date: Option<String>,
    #[serde(default)]
    pub variable_symbol: Option<String>,
    #[serde(default)]
    pub constant_symbol: Option<String>,
    #[serde(default)]
    pub specific_symbol: Option<String>,
    #[serde(default)]
    pub originators_reference_information: Option<String>,
    #[serde(default)]
    pub payment_note: Option<String>,
    pub bank_accounts: BankAccounts,
    #[serde(default)]
    pub standing_order_ext: Option<StandingOrderExt>,
    #[serde(default)]
    pub direct_debit_ext: Option<DirectDebitExt>,
    #[serde(default)]
    pub beneficiary_name: Option<String>,
    #[serde(default)]
    #[serde(rename = "BeneficiaryAddressLine1")]
    pub beneficiary_address_line1: Option<String>,
    #[serde(default)]
    #[serde(rename = "BeneficiaryAddressLine2")]
    pub beneficiary_address_line2: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct BankAccounts {
    #[serde(default)]
    pub bank_account: Vec<BankAccount>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub struct BankAccount {
    pub iban: String,
    #[serde(default)]
    pub bic: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct StandingOrderExt {
    #[serde(default)]
    pub day: Option<u8>,
    #[serde(default)]
    pub month: Option<String>,
    pub periodicity: String,
    #[serde(default)]
    pub last_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct DirectDebitExt {
    pub direct_debit_scheme: String,
    pub direct_debit_type: String,
    #[serde(default)]
    pub variable_symbol: Option<String>,
    #[serde(default)]
    pub specific_symbol: Option<String>,
    #[serde(default)]
    pub originators_reference_information: Option<String>,
    #[serde(rename = "MandateID")]
    pub mandate_id: String,
    #[serde(rename = "CreditorID")]
    pub creditor_id: String,
    #[serde(rename = "ContractID")]
    pub contract_id: String,
    #[serde(default)]
    pub max_amount: Option<Amount>,
    #[serde(default)]
    pub valid_till_date: Option<String>,
}

/// A decimal amount kept in its canonical string form.
///
/// Floating-point numbers cannot represent every amount accepted by the
/// by-square schema. Keeping the value as text also preserves its precision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Amount(String);

impl Amount {
    pub fn new(value: impl AsRef<str>) -> Result<Self> {
        normalize_number(value.as_ref(), false).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == "0"
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AmountVisitor;

        impl<'de> Visitor<'de> for AmountVisitor {
            type Value = Amount;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a non-negative decimal amount")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Amount::new(value).map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Amount::new(value.to_string()).map_err(E::custom)
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Amount::new(value.to_string()).map_err(E::custom)
            }

            fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !value.is_finite() {
                    return Err(E::custom("amount must be finite"));
                }
                normalize_number(&value.to_string(), true)
                    .map(Amount)
                    .map_err(E::custom)
            }

            fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut text = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "$text" || key == "$value" {
                        text = Some(map.next_value::<String>()?);
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }

                let text = text.ok_or_else(|| de::Error::custom("amount has no text value"))?;
                Amount::new(text).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(AmountVisitor)
    }
}

pub fn try_deserialize_pay(content: &str) -> Result<Pay> {
    let trimmed = content.trim_start();

    if trimmed.starts_with('<') {
        quick_xml::de::from_str(trimmed).map_err(|error| Error::Deserialize {
            format: "XML",
            message: error.to_string(),
        })
    } else if trimmed.starts_with('{') {
        serde_json::from_str(trimmed).map_err(|error| Error::Deserialize {
            format: "JSON",
            message: error.to_string(),
        })
    } else {
        Err(Error::Deserialize {
            format: "input",
            message: "expected an XML document or JSON object".to_owned(),
        })
    }
}

fn normalize_number(value: &str, allow_exponent: bool) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') {
        return Err(Error::invalid(
            "Amount",
            "must be a non-negative decimal number",
        ));
    }
    let value = value.strip_prefix('+').unwrap_or(value);
    if value.is_empty() {
        return Err(Error::invalid("Amount", "must be a decimal number"));
    }

    let (mantissa, exponent) = match value.find(['e', 'E']) {
        Some(index) if allow_exponent => {
            let exponent = value[index + 1..]
                .parse::<i32>()
                .map_err(|_| Error::invalid("Amount", "has an invalid exponent"))?;
            (&value[..index], exponent)
        }
        Some(_) => {
            return Err(Error::invalid(
                "Amount",
                "exponent notation is not valid for an XSD decimal",
            ));
        }
        None => (value, 0),
    };

    let mut parts = mantissa.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || (integer.is_empty() && fraction.is_empty())
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(Error::invalid("Amount", "must be a decimal number"));
    }

    let digits = format!("{integer}{fraction}");
    let decimal_position = i32::try_from(integer.len())
        .map_err(|_| Error::invalid("Amount", "is too long"))?
        .checked_add(exponent)
        .ok_or_else(|| Error::invalid("Amount", "has an exponent outside the supported range"))?;
    if !(-1000..=1000).contains(&decimal_position) {
        return Err(Error::invalid(
            "Amount",
            "has an exponent outside the supported range",
        ));
    }

    let expanded = if decimal_position <= 0 {
        format!("0.{}{}", "0".repeat((-decimal_position) as usize), digits)
    } else if decimal_position as usize >= digits.len() {
        format!(
            "{}{}",
            digits,
            "0".repeat(decimal_position as usize - digits.len())
        )
    } else {
        let position = decimal_position as usize;
        format!("{}.{}", &digits[..position], &digits[position..])
    };

    let (integer, fraction) = expanded.split_once('.').unwrap_or((&expanded, ""));
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let fraction = fraction.trim_end_matches('0');

    Ok(if fraction.is_empty() {
        integer.to_owned()
    } else {
        format!("{integer}.{fraction}")
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_number, try_deserialize_pay, Amount};

    #[test]
    fn normalizes_decimal_without_losing_precision() {
        assert_eq!(Amount::new("0012.34567800").unwrap().as_str(), "12.345678");
        assert_eq!(Amount::new("+1.20").unwrap().as_str(), "1.2");
        assert!(Amount::new("1e-8").is_err());
        assert_eq!(normalize_number("1e-8", true).unwrap(), "0.00000001");
        assert_eq!(normalize_number("1.2e3", true).unwrap(), "1200");
    }

    #[test]
    fn deserializes_exact_xml_amount() {
        let pay = try_deserialize_pay(
            r#"<Pay><Payments><Payment><PaymentOptions>paymentorder</PaymentOptions><Amount>12.345678</Amount><CurrencyCode>EUR</CurrencyCode><BankAccounts><BankAccount><IBAN>SK7700000000000000000000</IBAN></BankAccount></BankAccounts></Payment></Payments></Pay>"#,
        )
        .unwrap();

        assert_eq!(
            pay.payments.payment[0].amount.as_ref().unwrap().as_str(),
            "12.345678"
        );
    }

    #[test]
    fn deserializes_json_numeric_amount() {
        let amount: Amount = serde_json::from_str("12.345678").unwrap();
        assert_eq!(amount.as_str(), "12.345678");
    }
}
