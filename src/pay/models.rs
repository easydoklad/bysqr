use std::{fmt, str::FromStr};

use serde::{
    de::{self, IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentOption {
    PaymentOrder,
    StandingOrder,
    DirectDebit,
}

impl PaymentOption {
    const fn classifier(self) -> u8 {
        match self {
            Self::PaymentOrder => 1,
            Self::StandingOrder => 2,
            Self::DirectDebit => 4,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::PaymentOrder => "paymentorder",
            Self::StandingOrder => "standingorder",
            Self::DirectDebit => "directdebit",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub struct PaymentOptions(u8);

impl PaymentOptions {
    pub fn new(options: impl IntoIterator<Item = PaymentOption>) -> Result<Self> {
        let mut classifier = 0;
        for option in options {
            classifier |= option.classifier();
        }

        if classifier == 0 {
            return Err(Error::invalid(
                "PaymentOptions",
                "must contain at least one option",
            ));
        }

        Ok(Self(classifier))
    }

    pub const fn classifier(self) -> u8 {
        self.0
    }

    pub const fn contains(self, option: PaymentOption) -> bool {
        self.0 & option.classifier() != 0
    }

    pub fn from_classifier(classifier: u8) -> Result<Self> {
        if (1..=7).contains(&classifier) {
            Ok(Self(classifier))
        } else {
            Err(Error::invalid(
                "PaymentOptions",
                format!("unknown classifier {classifier}"),
            ))
        }
    }
}

impl FromStr for PaymentOptions {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let mut classifier = 0;
        let mut found = false;

        for name in value.split_whitespace() {
            found = true;
            let option = match name {
                "paymentorder" => PaymentOption::PaymentOrder,
                "standingorder" => PaymentOption::StandingOrder,
                "directdebit" => PaymentOption::DirectDebit,
                _ => {
                    return Err(Error::invalid(
                        "PaymentOptions",
                        format!("unknown option {name:?}"),
                    ));
                }
            };
            let bit = option.classifier();
            if classifier & bit != 0 {
                return Err(Error::invalid(
                    "PaymentOptions",
                    format!("contains duplicate option {name:?}"),
                ));
            }
            classifier |= bit;
        }

        if !found {
            return Err(Error::invalid(
                "PaymentOptions",
                "must contain at least one option",
            ));
        }

        Ok(Self(classifier))
    }
}

impl TryFrom<String> for PaymentOptions {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for PaymentOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let options = [
            PaymentOption::PaymentOrder,
            PaymentOption::StandingOrder,
            PaymentOption::DirectDebit,
        ];
        let mut separator = "";
        for option in options {
            if self.contains(option) {
                formatter.write_str(separator)?;
                formatter.write_str(option.name())?;
                separator = " ";
            }
        }
        Ok(())
    }
}

impl Serialize for PaymentOptions {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Pay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "InvoiceID")]
    pub invoice_id: Option<String>,
    pub payments: Payments,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Payments {
    #[serde(default)]
    pub payment: Vec<Payment>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Payment {
    pub payment_options: PaymentOptions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    pub currency_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constant_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specific_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub originators_reference_information: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_note: Option<String>,
    pub bank_accounts: BankAccounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing_order_ext: Option<StandingOrderExt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_debit_ext: Option<DirectDebitExt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beneficiary_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "BeneficiaryAddressLine1")]
    pub beneficiary_address_line1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "BeneficiaryAddressLine2")]
    pub beneficiary_address_line2: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct BankAccounts {
    #[serde(default)]
    pub bank_account: Vec<BankAccount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub struct BankAccount {
    pub iban: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bic: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct StandingOrderExt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<Months>,
    pub periodicity: Periodicity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_date: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub enum Periodicity {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
    Bimonthly,
    Quarterly,
    Annually,
    Semiannually,
}

impl Periodicity {
    pub const fn classifier(self) -> char {
        match self {
            Self::Daily => 'd',
            Self::Weekly => 'w',
            Self::Biweekly => 'b',
            Self::Monthly => 'm',
            Self::Bimonthly => 'B',
            Self::Quarterly => 'q',
            Self::Annually => 'a',
            Self::Semiannually => 's',
        }
    }

    pub const fn allows_months(self) -> bool {
        matches!(
            self,
            Self::Weekly | Self::Biweekly | Self::Monthly | Self::Bimonthly
        )
    }

    pub fn from_classifier(classifier: char) -> Result<Self> {
        match classifier {
            'd' => Ok(Self::Daily),
            'w' => Ok(Self::Weekly),
            'b' => Ok(Self::Biweekly),
            'm' => Ok(Self::Monthly),
            'B' => Ok(Self::Bimonthly),
            'q' => Ok(Self::Quarterly),
            'a' => Ok(Self::Annually),
            's' => Ok(Self::Semiannually),
            _ => Err(Error::invalid(
                "Periodicity",
                format!("unknown classifier {classifier:?}"),
            )),
        }
    }
}

impl FromStr for Periodicity {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "Daily" => Ok(Self::Daily),
            "Weekly" => Ok(Self::Weekly),
            "Biweekly" => Ok(Self::Biweekly),
            "Monthly" => Ok(Self::Monthly),
            "Bimonthly" => Ok(Self::Bimonthly),
            "Quarterly" => Ok(Self::Quarterly),
            "Annually" => Ok(Self::Annually),
            "Semiannually" => Ok(Self::Semiannually),
            _ => Err(Error::invalid(
                "Periodicity",
                format!("unknown value {value:?}"),
            )),
        }
    }
}

impl TryFrom<String> for Periodicity {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for Periodicity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Biweekly => "Biweekly",
            Self::Monthly => "Monthly",
            Self::Bimonthly => "Bimonthly",
            Self::Quarterly => "Quarterly",
            Self::Annually => "Annually",
            Self::Semiannually => "Semiannually",
        })
    }
}

impl Serialize for Periodicity {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl Month {
    const ALL: [Self; 12] = [
        Self::January,
        Self::February,
        Self::March,
        Self::April,
        Self::May,
        Self::June,
        Self::July,
        Self::August,
        Self::September,
        Self::October,
        Self::November,
        Self::December,
    ];

    const fn classifier(self) -> u16 {
        1 << self.index()
    }

    const fn index(self) -> u8 {
        match self {
            Self::January => 0,
            Self::February => 1,
            Self::March => 2,
            Self::April => 3,
            Self::May => 4,
            Self::June => 5,
            Self::July => 6,
            Self::August => 7,
            Self::September => 8,
            Self::October => 9,
            Self::November => 10,
            Self::December => 11,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::January => "January",
            Self::February => "February",
            Self::March => "March",
            Self::April => "April",
            Self::May => "May",
            Self::June => "June",
            Self::July => "July",
            Self::August => "August",
            Self::September => "September",
            Self::October => "October",
            Self::November => "November",
            Self::December => "December",
        }
    }
}

impl FromStr for Month {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Month::ALL
            .into_iter()
            .find(|month| month.name() == value)
            .ok_or_else(|| Error::invalid("Month", format!("unknown value {value:?}")))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub struct Months(u16);

impl Months {
    pub fn new(months: impl IntoIterator<Item = Month>) -> Result<Self> {
        let mut classifier = 0;
        for month in months {
            classifier |= month.classifier();
        }
        if classifier == 0 {
            return Err(Error::invalid("Month", "must contain at least one month"));
        }
        Ok(Self(classifier))
    }

    pub const fn classifier(self) -> u16 {
        self.0
    }

    pub const fn contains(self, month: Month) -> bool {
        self.0 & month.classifier() != 0
    }

    pub fn from_classifier(classifier: u16) -> Result<Self> {
        if (1..=4095).contains(&classifier) {
            Ok(Self(classifier))
        } else {
            Err(Error::invalid(
                "Month",
                format!("unknown classifier {classifier}"),
            ))
        }
    }
}

impl FromStr for Months {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let mut classifier = 0;
        let mut found = false;
        for name in value.split_whitespace() {
            found = true;
            let month: Month = name.parse()?;
            let bit = month.classifier();
            if classifier & bit != 0 {
                return Err(Error::invalid(
                    "Month",
                    format!("contains duplicate month {name:?}"),
                ));
            }
            classifier |= bit;
        }
        if !found {
            return Err(Error::invalid("Month", "must contain at least one month"));
        }
        Ok(Self(classifier))
    }
}

impl TryFrom<String> for Months {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for Months {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        for month in Month::ALL {
            if self.contains(month) {
                formatter.write_str(separator)?;
                formatter.write_str(month.name())?;
                separator = " ";
            }
        }
        Ok(())
    }
}

impl Serialize for Months {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct DirectDebitExt {
    pub direct_debit_scheme: DirectDebitScheme,
    pub direct_debit_type: DirectDebitType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specific_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub originators_reference_information: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "MandateID")]
    pub mandate_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CreditorID"
    )]
    pub creditor_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "ContractID"
    )]
    pub contract_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<Amount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_till_date: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub enum DirectDebitScheme {
    Other,
    Sepa,
}

impl DirectDebitScheme {
    pub const fn classifier(self) -> u8 {
        match self {
            Self::Other => 0,
            Self::Sepa => 1,
        }
    }

    pub fn from_classifier(classifier: u8) -> Result<Self> {
        match classifier {
            0 => Ok(Self::Other),
            1 => Ok(Self::Sepa),
            _ => Err(Error::invalid(
                "DirectDebitScheme",
                format!("unknown classifier {classifier}"),
            )),
        }
    }
}

impl FromStr for DirectDebitScheme {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "other" => Ok(Self::Other),
            "SEPA" => Ok(Self::Sepa),
            _ => Err(Error::invalid(
                "DirectDebitScheme",
                format!("unknown value {value:?}"),
            )),
        }
    }
}

impl TryFrom<String> for DirectDebitScheme {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for DirectDebitScheme {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Other => "other",
            Self::Sepa => "SEPA",
        })
    }
}

impl Serialize for DirectDebitScheme {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub enum DirectDebitType {
    OneOff,
    Recurrent,
}

impl DirectDebitType {
    pub const fn classifier(self) -> u8 {
        match self {
            Self::OneOff => 0,
            Self::Recurrent => 1,
        }
    }

    pub fn from_classifier(classifier: u8) -> Result<Self> {
        match classifier {
            0 => Ok(Self::OneOff),
            1 => Ok(Self::Recurrent),
            _ => Err(Error::invalid(
                "DirectDebitType",
                format!("unknown classifier {classifier}"),
            )),
        }
    }
}

impl FromStr for DirectDebitType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "one-off" => Ok(Self::OneOff),
            "recurrent" => Ok(Self::Recurrent),
            _ => Err(Error::invalid(
                "DirectDebitType",
                format!("unknown value {value:?}"),
            )),
        }
    }
}

impl TryFrom<String> for DirectDebitType {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for DirectDebitType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OneOff => "one-off",
            Self::Recurrent => "recurrent",
        })
    }
}

impl Serialize for DirectDebitType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
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

impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
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
    use super::{
        normalize_number, try_deserialize_pay, Amount, DirectDebitScheme, DirectDebitType, Month,
        Months, PaymentOption, PaymentOptions, Periodicity,
    };

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

    #[test]
    fn maps_payment_options_to_classifier() {
        let options: PaymentOptions = "directdebit paymentorder standingorder".parse().unwrap();

        assert_eq!(options.classifier(), 7);
        assert!(options.contains(PaymentOption::PaymentOrder));
        assert!(options.contains(PaymentOption::StandingOrder));
        assert!(options.contains(PaymentOption::DirectDebit));
        assert_eq!(
            options.to_string(),
            "paymentorder standingorder directdebit"
        );
        assert!("paymentorder paymentorder"
            .parse::<PaymentOptions>()
            .is_err());
    }

    #[test]
    fn maps_periodicities_to_classifiers() {
        for (value, classifier) in [
            ("Daily", 'd'),
            ("Weekly", 'w'),
            ("Biweekly", 'b'),
            ("Monthly", 'm'),
            ("Bimonthly", 'B'),
            ("Quarterly", 'q'),
            ("Annually", 'a'),
            ("Semiannually", 's'),
        ] {
            assert_eq!(
                value.parse::<Periodicity>().unwrap().classifier(),
                classifier
            );
        }
        assert!("monthly".parse::<Periodicity>().is_err());
    }

    #[test]
    fn sums_month_classifiers() {
        let months: Months = "January April July October".parse().unwrap();

        assert_eq!(months.classifier(), 585);
        assert!(months.contains(Month::January));
        assert!(months.contains(Month::October));
        assert_eq!(months.to_string(), "January April July October");
        assert!("January January".parse::<Months>().is_err());
    }

    #[test]
    fn maps_direct_debit_classifiers() {
        assert_eq!(
            "other".parse::<DirectDebitScheme>().unwrap().classifier(),
            0
        );
        assert_eq!("SEPA".parse::<DirectDebitScheme>().unwrap().classifier(), 1);
        assert_eq!(
            "one-off".parse::<DirectDebitType>().unwrap().classifier(),
            0
        );
        assert_eq!(
            "recurrent".parse::<DirectDebitType>().unwrap().classifier(),
            1
        );
        assert!("sepa".parse::<DirectDebitScheme>().is_err());
        assert!("recurring".parse::<DirectDebitType>().is_err());
    }
}
