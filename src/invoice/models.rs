use std::{fmt, str::FromStr};

use serde::{
    de::{self, IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

pub use crate::diagnostic::AdvisoryDiagnostic;

const BYSQUARE_NAMESPACE: &str = "http://www.bysquare.com/bysquare";
const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

pub type ModelResult<T> = std::result::Result<T, InvoiceModelError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceModelError {
    field: &'static str,
    message: String,
}

impl InvoiceModelError {
    pub(crate) fn invalid(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for InvoiceModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid {}: {}", self.field, self.message)
    }
}

impl std::error::Error for InvoiceModelError {}

/// The semantic Invoice document type.
///
/// Canonical JSON transports this value as `DocumentType`. XML transports the
/// same value in the root element's `xsi:type` attribute.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DocumentType {
    Invoice,
    ProformaInvoice,
    CreditNote,
    DebitNote,
    AdvanceInvoice,
}

impl DocumentType {
    pub const fn classifier(self) -> u8 {
        match self {
            Self::Invoice => 0,
            Self::ProformaInvoice => 1,
            Self::CreditNote => 2,
            Self::DebitNote => 3,
            Self::AdvanceInvoice => 4,
        }
    }

    pub fn from_classifier(classifier: u8) -> ModelResult<Self> {
        match classifier {
            0 => Ok(Self::Invoice),
            1 => Ok(Self::ProformaInvoice),
            2 => Ok(Self::CreditNote),
            3 => Ok(Self::DebitNote),
            4 => Ok(Self::AdvanceInvoice),
            _ => Err(InvoiceModelError::invalid(
                "DocumentType",
                format!("unknown classifier {classifier}"),
            )),
        }
    }

    pub const fn as_xsi_type(self) -> &'static str {
        match self {
            Self::Invoice => "Invoice",
            Self::ProformaInvoice => "ProformaInvoice",
            Self::CreditNote => "CreditNote",
            Self::DebitNote => "DebitNote",
            Self::AdvanceInvoice => "AdvanceInvoice",
        }
    }
}

impl FromStr for DocumentType {
    type Err = InvoiceModelError;

    fn from_str(value: &str) -> ModelResult<Self> {
        let value = value.rsplit_once(':').map_or(value, |(_, local)| local);
        match value {
            "Invoice" => Ok(Self::Invoice),
            "ProformaInvoice" => Ok(Self::ProformaInvoice),
            "CreditNote" => Ok(Self::CreditNote),
            "DebitNote" => Ok(Self::DebitNote),
            "AdvanceInvoice" => Ok(Self::AdvanceInvoice),
            _ => Err(InvoiceModelError::invalid(
                "DocumentType",
                format!("unknown xsi:type {value:?}"),
            )),
        }
    }
}

/// An arbitrary-precision XSD decimal in canonical textual form.
///
/// Serialization always emits a JSON string. Deserialization also accepts a
/// JSON number as a convenience and immediately normalizes it to text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decimal(String);

impl Decimal {
    pub fn new(value: impl AsRef<str>) -> ModelResult<Self> {
        normalize_decimal(value.as_ref(), false).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == "0"
    }

    pub fn add_exact(&self, other: &Self) -> Self {
        ScaledDecimal::from(self)
            .add(&ScaledDecimal::from(other))
            .into_decimal()
    }

    pub fn subtract_exact(&self, other: &Self) -> Self {
        let mut other = ScaledDecimal::from(other);
        if !other.is_zero() {
            other.negative = !other.negative;
        }
        ScaledDecimal::from(self).add(&other).into_decimal()
    }

    /// Multiplies two arbitrary-precision decimals without rounding.
    pub fn multiply_exact(&self, other: &Self) -> Self {
        ScaledDecimal::from(self)
            .multiply(&ScaledDecimal::from(other))
            .into_decimal()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecimalVisitor;

        impl<'de> Visitor<'de> for DecimalVisitor {
            type Value = Decimal;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an XSD decimal string or a finite JSON number")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Decimal::new(value).map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Decimal::new(value.to_string()).map_err(E::custom)
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Decimal::new(value.to_string()).map_err(E::custom)
            }

            fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !value.is_finite() {
                    return Err(E::custom("decimal must be finite"));
                }
                normalize_decimal(&value.to_string(), true)
                    .map(Decimal)
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
                let text = text.ok_or_else(|| de::Error::custom("decimal has no text value"))?;
                Decimal::new(text).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(DecimalVisitor)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Percentage(Decimal);

impl Percentage {
    pub fn new(value: impl AsRef<str>) -> ModelResult<Self> {
        Self::try_from(Decimal::new(value)?)
    }

    pub fn as_decimal(&self) -> &Decimal {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<Decimal> for Percentage {
    type Error = InvoiceModelError;

    fn try_from(value: Decimal) -> ModelResult<Self> {
        let valid = value.as_str() == "0"
            || value.as_str() == "1"
            || value
                .as_str()
                .strip_prefix("0.")
                .is_some_and(|fraction| !fraction.is_empty());
        if valid {
            Ok(Self(value))
        } else {
            Err(InvoiceModelError::invalid(
                "Percentage",
                "must be in the canonical VAT range from 0 to 1 inclusive",
            ))
        }
    }
}

impl<'de> Deserialize<'de> for Percentage {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(Decimal::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

macro_rules! code_type {
    ($name:ident, $field:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> ModelResult<Self> {
                let value = value.into();
                if value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase()) {
                    Ok(Self(value))
                } else {
                    Err(InvoiceModelError::invalid(
                        $field,
                        "must contain exactly three ASCII uppercase letters",
                    ))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvoiceModelError;

            fn try_from(value: String) -> ModelResult<Self> {
                Self::new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
            }
        }
    };
}

code_type!(CurrencyCode, "CurrencyCode");
code_type!(CountryCode, "CountryCode");

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Date(String);

impl Date {
    pub fn new(value: impl Into<String>) -> ModelResult<Self> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes[..4].iter().all(u8::is_ascii_digit)
            || !bytes[5..7].iter().all(u8::is_ascii_digit)
            || !bytes[8..].iter().all(u8::is_ascii_digit)
        {
            return Err(InvoiceModelError::invalid(
                "Date",
                "must be a valid calendar date in YYYY-MM-DD form",
            ));
        }
        chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
            InvoiceModelError::invalid("Date", "must be a valid calendar date in YYYY-MM-DD form")
        })?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentMean {
    MoneyTransfer,
    Cash,
    CashOnDelivery,
    CreditCard,
    Advance,
    MutualOffset,
    Other,
}

impl PaymentMean {
    const ALL: [Self; 7] = [
        Self::MoneyTransfer,
        Self::Cash,
        Self::CashOnDelivery,
        Self::CreditCard,
        Self::Advance,
        Self::MutualOffset,
        Self::Other,
    ];

    pub const fn classifier(self) -> u8 {
        match self {
            Self::MoneyTransfer => 1,
            Self::Cash => 2,
            Self::CashOnDelivery => 4,
            Self::CreditCard => 8,
            Self::Advance => 16,
            Self::MutualOffset => 32,
            Self::Other => 64,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MoneyTransfer => "moneyTransfer",
            Self::Cash => "cash",
            Self::CashOnDelivery => "cashOnDelivery",
            Self::CreditCard => "creditCard",
            Self::Advance => "advance",
            Self::MutualOffset => "mutualOffset",
            Self::Other => "other",
        }
    }
}

impl FromStr for PaymentMean {
    type Err = InvoiceModelError;

    fn from_str(value: &str) -> ModelResult<Self> {
        Self::ALL
            .into_iter()
            .find(|mean| mean.as_str() == value)
            .ok_or_else(|| {
                InvoiceModelError::invalid("PaymentMeans", format!("unknown value {value:?}"))
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaymentMeans(u8);

impl PaymentMeans {
    pub fn new(means: impl IntoIterator<Item = PaymentMean>) -> ModelResult<Self> {
        let mut classifier = 0;
        for mean in means {
            let bit = mean.classifier();
            if classifier & bit != 0 {
                return Err(InvoiceModelError::invalid(
                    "PaymentMeans",
                    format!("contains duplicate value {:?}", mean.as_str()),
                ));
            }
            classifier |= bit;
        }
        Self::from_classifier(classifier)
    }

    pub fn from_classifier(classifier: u8) -> ModelResult<Self> {
        if (1..=127).contains(&classifier) {
            Ok(Self(classifier))
        } else {
            Err(InvoiceModelError::invalid(
                "PaymentMeans",
                format!("unknown classifier {classifier}"),
            ))
        }
    }

    pub const fn classifier(self) -> u8 {
        self.0
    }

    pub const fn contains(self, mean: PaymentMean) -> bool {
        self.0 & mean.classifier() != 0
    }
}

impl FromStr for PaymentMeans {
    type Err = InvoiceModelError;

    fn from_str(value: &str) -> ModelResult<Self> {
        Self::new(
            value
                .split_whitespace()
                .map(str::parse)
                .collect::<ModelResult<Vec<_>>>()?,
        )
    }
}

impl fmt::Display for PaymentMeans {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        for mean in PaymentMean::ALL {
            if self.contains(mean) {
                formatter.write_str(separator)?;
                formatter.write_str(mean.as_str())?;
                separator = " ";
            }
        }
        Ok(())
    }
}

impl Serialize for PaymentMeans {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PaymentMeans {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Invoice {
    pub document_type: DocumentType,
    #[serde(flatten)]
    pub data: InvoiceData,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InvoiceWire {
    document_type: DocumentType,
    #[serde(flatten)]
    data: InvoiceData,
}

impl<'de> Deserialize<'de> for Invoice {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = InvoiceWire::deserialize(deserializer)?;
        Self::new(wire.document_type, wire.data).map_err(de::Error::custom)
    }
}

impl Invoice {
    pub fn new(document_type: DocumentType, data: InvoiceData) -> ModelResult<Self> {
        let invoice = Self {
            document_type,
            data,
        };
        invoice.validate()?;
        Ok(invoice)
    }

    pub fn validate(&self) -> ModelResult<()> {
        self.data.validate()
    }

    /// Reports transported fields that exceed their advisory XSD
    /// `bsqr:maxLength` value.
    ///
    /// This method never truncates or changes the Invoice, and these
    /// diagnostics never make [`Invoice::validate`] reject a document.
    pub fn advisory_diagnostics(&self) -> Vec<AdvisoryDiagnostic> {
        let mut diagnostics = Vec::new();
        let data = &self.data;

        diagnose_text(&mut diagnostics, "InvoiceID", &data.invoice_id, 10);
        diagnose_date(&mut diagnostics, "IssueDate", &data.issue_date, 8);
        diagnose_optional_date(
            &mut diagnostics,
            "TaxPointDate",
            data.tax_point_date.as_ref(),
            8,
        );
        diagnose_optional_text(&mut diagnostics, "OrderID", data.order_id.as_deref(), 10);
        diagnose_optional_text(
            &mut diagnostics,
            "DeliveryNoteID",
            data.delivery_note_id.as_deref(),
            10,
        );
        diagnose_text(
            &mut diagnostics,
            "LocalCurrencyCode",
            data.local_currency_code.as_str(),
            3,
        );
        if let Some(value) = &data.foreign_currency_code {
            diagnose_text(&mut diagnostics, "ForeignCurrencyCode", value.as_str(), 3);
        }
        diagnose_optional_decimal(&mut diagnostics, "CurrRate", data.curr_rate.as_ref(), 15);
        diagnose_optional_decimal(
            &mut diagnostics,
            "ReferenceCurrRate",
            data.reference_curr_rate.as_ref(),
            15,
        );

        diagnose_supplier_party(&mut diagnostics, &data.supplier_party);
        diagnose_customer_party(&mut diagnostics, &data.customer_party);

        if let Some(value) = data.number_of_invoice_lines {
            diagnose_text(
                &mut diagnostics,
                "NumberOfInvoiceLines",
                &value.to_string(),
                11,
            );
        }
        diagnose_optional_text(
            &mut diagnostics,
            "InvoiceDescription",
            data.invoice_description.as_deref(),
            30,
        );
        if let Some(line) = &data.single_invoice_line {
            diagnose_single_invoice_line(&mut diagnostics, line);
        }

        for (index, summary) in data
            .tax_category_summaries
            .tax_category_summary
            .iter()
            .enumerate()
        {
            diagnose_tax_category_summary(&mut diagnostics, index, summary);
        }

        diagnose_text(
            &mut diagnostics,
            "MonetarySummary.PayableRoundingAmount",
            data.monetary_summary.payable_rounding_amount.as_str(),
            15,
        );
        diagnose_text(
            &mut diagnostics,
            "MonetarySummary.PaidDepositsAmount",
            data.monetary_summary.paid_deposits_amount.as_str(),
            15,
        );
        if let Some(payment_means) = data.payment_means {
            diagnose_text(
                &mut diagnostics,
                "PaymentMeans",
                &payment_means.classifier().to_string(),
                3,
            );
        }

        diagnostics
    }

    pub fn from_xml_str(source: &str) -> ModelResult<Self> {
        let document_type = document_type_from_xml(source)?;
        let data: InvoiceData = quick_xml::de::from_str(source)
            .map_err(|error| InvoiceModelError::invalid("Invoice XML", error.to_string()))?;
        Self::new(document_type, data)
    }

    pub fn to_xml_string(&self) -> ModelResult<String> {
        self.validate()?;
        let body = quick_xml::se::to_string(&self.data)
            .map_err(|error| InvoiceModelError::invalid("Invoice XML", error.to_string()))?;
        let end = body.find('>').ok_or_else(|| {
            InvoiceModelError::invalid("Invoice XML", "serialized root element has no end")
        })?;
        if !body[..end].starts_with("<Invoice") {
            return Err(InvoiceModelError::invalid(
                "Invoice XML",
                "serialized root element is not Invoice",
            ));
        }
        Ok(format!(
            "{} xmlns=\"{}\" xmlns:xsi=\"{}\" xsi:type=\"{}\"{}",
            &body[..end],
            BYSQUARE_NAMESPACE,
            XSI_NAMESPACE,
            self.document_type.as_xsi_type(),
            &body[end..]
        ))
    }

    /// Calculates every sum and difference exactly from transported fields.
    /// Existing optional computed fields are deliberately ignored.
    pub fn calculate_totals(&self) -> InvoiceTotalsCalculation {
        let tax_categories = self
            .data
            .tax_category_summaries
            .tax_category_summary
            .iter()
            .map(TaxCategorySummary::calculate)
            .collect::<Vec<_>>();

        let zero = || Decimal("0".to_owned());
        let sum = |values: Vec<&Decimal>| {
            values
                .into_iter()
                .fold(zero(), |total, value| total.add_exact(value))
        };

        let monetary_summary = MonetarySummaryCalculation {
            tax_exclusive_amount: sum(tax_categories
                .iter()
                .map(|item| &item.tax_exclusive_amount)
                .collect()),
            tax_inclusive_amount: sum(tax_categories
                .iter()
                .map(|item| &item.tax_inclusive_amount)
                .collect()),
            tax_amount: sum(tax_categories.iter().map(|item| &item.tax_amount).collect()),
            already_claimed_tax_exclusive_amount: sum(tax_categories
                .iter()
                .map(|item| &item.already_claimed_tax_exclusive_amount)
                .collect()),
            already_claimed_tax_inclusive_amount: sum(tax_categories
                .iter()
                .map(|item| &item.already_claimed_tax_inclusive_amount)
                .collect()),
            already_claimed_tax_amount: sum(tax_categories
                .iter()
                .map(|item| &item.already_claimed_tax_amount)
                .collect()),
            difference_tax_exclusive_amount: sum(tax_categories
                .iter()
                .map(|item| &item.difference_tax_exclusive_amount)
                .collect()),
            difference_tax_inclusive_amount: sum(tax_categories
                .iter()
                .map(|item| &item.difference_tax_inclusive_amount)
                .collect()),
            difference_tax_amount: sum(tax_categories
                .iter()
                .map(|item| &item.difference_tax_amount)
                .collect()),
            payable_amount: zero(),
        };

        let payable_amount = monetary_summary
            .difference_tax_inclusive_amount
            .subtract_exact(&self.data.monetary_summary.paid_deposits_amount)
            .add_exact(&self.data.monetary_summary.payable_rounding_amount);

        InvoiceTotalsCalculation {
            tax_category_summaries: tax_categories,
            monetary_summary: MonetarySummaryCalculation {
                payable_amount,
                ..monetary_summary
            },
        }
    }

    /// Calculates all computed values. Division is delegated to the caller so
    /// non-terminating unit prices never acquire an implicit rounding rule.
    pub fn calculate_with_division<D: DecimalDivision>(
        &self,
        division: &D,
    ) -> std::result::Result<InvoiceCalculation, D::Error> {
        let totals = self.calculate_totals();
        let single_invoice_line = match &self.data.single_invoice_line {
            Some(line) => Some(SingleInvoiceLineCalculation {
                unit_price_tax_exclusive_amount: division.divide(
                    &totals.monetary_summary.tax_exclusive_amount,
                    &line.invoiced_quantity,
                )?,
                unit_price_tax_inclusive_amount: division.divide(
                    &totals.monetary_summary.tax_inclusive_amount,
                    &line.invoiced_quantity,
                )?,
                unit_price_tax_amount: division
                    .divide(&totals.monetary_summary.tax_amount, &line.invoiced_quantity)?,
            }),
            None => None,
        };
        Ok(InvoiceCalculation {
            totals,
            single_invoice_line,
        })
    }
}

fn diagnose_supplier_party(diagnostics: &mut Vec<AdvisoryDiagnostic>, party: &SupplierParty) {
    diagnose_text(
        diagnostics,
        "SupplierParty.PartyName",
        &party.party_name,
        20,
    );
    diagnose_optional_text(
        diagnostics,
        "SupplierParty.CompanyTaxID",
        party.company_tax_id.as_deref(),
        12,
    );
    diagnose_optional_text(
        diagnostics,
        "SupplierParty.CompanyVATID",
        party.company_vat_id.as_deref(),
        14,
    );
    diagnose_optional_text(
        diagnostics,
        "SupplierParty.CompanyRegisterID",
        party.company_register_id.as_deref(),
        14,
    );

    let address = &party.postal_address;
    diagnose_text(
        diagnostics,
        "SupplierParty.PostalAddress.StreetName",
        &address.street_name,
        20,
    );
    diagnose_optional_text(
        diagnostics,
        "SupplierParty.PostalAddress.BuildingNumber",
        address.building_number.as_deref(),
        3,
    );
    diagnose_text(
        diagnostics,
        "SupplierParty.PostalAddress.CityName",
        &address.city_name,
        20,
    );
    diagnose_text(
        diagnostics,
        "SupplierParty.PostalAddress.PostalZone",
        &address.postal_zone,
        10,
    );
    diagnose_optional_text(
        diagnostics,
        "SupplierParty.PostalAddress.State",
        address.state.as_deref(),
        10,
    );
    diagnose_text(
        diagnostics,
        "SupplierParty.PostalAddress.Country",
        address.country.as_str(),
        3,
    );

    if let Some(contact) = &party.contact {
        diagnose_optional_text(
            diagnostics,
            "SupplierParty.Contact.Name",
            contact.name.as_deref(),
            20,
        );
        diagnose_optional_text(
            diagnostics,
            "SupplierParty.Contact.Telephone",
            contact.telephone.as_deref(),
            12,
        );
        diagnose_optional_text(
            diagnostics,
            "SupplierParty.Contact.EMail",
            contact.email.as_deref(),
            40,
        );
    }
}

fn diagnose_customer_party(diagnostics: &mut Vec<AdvisoryDiagnostic>, party: &CustomerParty) {
    diagnose_text(
        diagnostics,
        "CustomerParty.PartyName",
        &party.party_name,
        20,
    );
    diagnose_optional_text(
        diagnostics,
        "CustomerParty.CompanyTaxID",
        party.company_tax_id.as_deref(),
        12,
    );
    diagnose_optional_text(
        diagnostics,
        "CustomerParty.CompanyVATID",
        party.company_vat_id.as_deref(),
        14,
    );
    diagnose_optional_text(
        diagnostics,
        "CustomerParty.CompanyRegisterID",
        party.company_register_id.as_deref(),
        14,
    );
    diagnose_optional_text(
        diagnostics,
        "CustomerParty.PartyIdentification",
        party.party_identification.as_deref(),
        20,
    );
}

fn diagnose_single_invoice_line(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    line: &SingleInvoiceLine,
) {
    diagnose_optional_text(
        diagnostics,
        "SingleInvoiceLine.OrderLineID",
        line.order_line_id.as_deref(),
        10,
    );
    diagnose_optional_text(
        diagnostics,
        "SingleInvoiceLine.DeliveryNoteLineID",
        line.delivery_note_line_id.as_deref(),
        10,
    );
    diagnose_optional_text(
        diagnostics,
        "SingleInvoiceLine.ItemName",
        line.item_name.as_deref(),
        30,
    );
    diagnose_optional_text(
        diagnostics,
        "SingleInvoiceLine.ItemEANCode",
        line.item_ean_code.as_deref(),
        30,
    );
    diagnose_optional_date(
        diagnostics,
        "SingleInvoiceLine.PeriodFromDate",
        line.period_from_date.as_ref(),
        8,
    );
    diagnose_optional_date(
        diagnostics,
        "SingleInvoiceLine.PeriodToDate",
        line.period_to_date.as_ref(),
        8,
    );
    diagnose_text(
        diagnostics,
        "SingleInvoiceLine.InvoicedQuantity",
        line.invoiced_quantity.as_str(),
        15,
    );
}

fn diagnose_tax_category_summary(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    index: usize,
    summary: &TaxCategorySummary,
) {
    let prefix = format!("TaxCategorySummaries.TaxCategorySummary[{index}]");
    diagnose_text(
        diagnostics,
        format!("{prefix}.ClassifiedTaxCategory"),
        summary.classified_tax_category.as_str(),
        15,
    );
    diagnose_text(
        diagnostics,
        format!("{prefix}.TaxExclusiveAmount"),
        summary.tax_exclusive_amount.as_str(),
        15,
    );
    diagnose_text(
        diagnostics,
        format!("{prefix}.TaxAmount"),
        summary.tax_amount.as_str(),
        15,
    );
    diagnose_text(
        diagnostics,
        format!("{prefix}.AlreadyClaimedTaxExclusiveAmount"),
        summary.already_claimed_tax_exclusive_amount.as_str(),
        15,
    );
    diagnose_text(
        diagnostics,
        format!("{prefix}.AlreadyClaimedTaxAmount"),
        summary.already_claimed_tax_amount.as_str(),
        15,
    );
}

fn diagnose_optional_decimal(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: impl Into<String>,
    value: Option<&Decimal>,
    recommended_maximum: usize,
) {
    if let Some(value) = value {
        diagnose_text(diagnostics, field_path, value.as_str(), recommended_maximum);
    }
}

fn diagnose_optional_date(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: impl Into<String>,
    value: Option<&Date>,
    recommended_maximum: usize,
) {
    if let Some(value) = value {
        diagnose_date(diagnostics, field_path, value, recommended_maximum);
    }
}

fn diagnose_date(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: impl Into<String>,
    value: &Date,
    recommended_maximum: usize,
) {
    diagnose_count(
        diagnostics,
        field_path,
        value
            .as_str()
            .chars()
            .filter(|character| *character != '-')
            .count(),
        recommended_maximum,
    );
}

fn diagnose_optional_text(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: impl Into<String>,
    value: Option<&str>,
    recommended_maximum: usize,
) {
    if let Some(value) = value {
        diagnose_text(diagnostics, field_path, value, recommended_maximum);
    }
}

fn diagnose_text(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: impl Into<String>,
    value: &str,
    recommended_maximum: usize,
) {
    diagnose_count(
        diagnostics,
        field_path,
        value.chars().count(),
        recommended_maximum,
    );
}

fn diagnose_count(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: impl Into<String>,
    actual_character_count: usize,
    recommended_maximum: usize,
) {
    if actual_character_count > recommended_maximum {
        diagnostics.push(AdvisoryDiagnostic {
            field_path: field_path.into(),
            actual_character_count,
            recommended_maximum,
        });
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "Invoice", rename_all = "PascalCase")]
pub struct InvoiceData {
    #[serde(rename = "InvoiceID")]
    pub invoice_id: String,
    pub issue_date: Date,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_point_date: Option<Date>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "OrderID")]
    pub order_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "DeliveryNoteID"
    )]
    pub delivery_note_id: Option<String>,
    pub local_currency_code: CurrencyCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreign_currency_code: Option<CurrencyCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curr_rate: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_curr_rate: Option<Decimal>,
    pub supplier_party: SupplierParty,
    pub customer_party: CustomerParty,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_of_invoice_lines: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single_invoice_line: Option<SingleInvoiceLine>,
    pub tax_category_summaries: TaxCategorySummaries,
    pub monetary_summary: MonetarySummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_means: Option<PaymentMeans>,
}

impl InvoiceData {
    fn validate(&self) -> ModelResult<()> {
        let foreign_fields = [
            self.foreign_currency_code.is_some(),
            self.curr_rate.is_some(),
            self.reference_curr_rate.is_some(),
        ];
        if foreign_fields.iter().any(|present| *present)
            && !foreign_fields.iter().all(|present| *present)
        {
            return Err(InvoiceModelError::invalid(
                "ForeignCurrencyCode",
                "ForeignCurrencyCode, CurrRate, and ReferenceCurrRate must occur together",
            ));
        }

        if matches!(self.number_of_invoice_lines, Some(value) if value < 0 || value == 1) {
            return Err(InvoiceModelError::invalid(
                "NumberOfInvoiceLines",
                "must be 0 for a header-only invoice or at least 2 for a multi-line invoice; use SingleInvoiceLine for one line",
            ));
        }

        match (
            self.number_of_invoice_lines.is_some(),
            self.single_invoice_line.as_ref(),
        ) {
            (true, None) => {}
            (false, Some(line)) if self.invoice_description.is_none() => line.validate()?,
            _ => {
                return Err(InvoiceModelError::invalid(
                    "Invoice lines",
                    "choose NumberOfInvoiceLines (with optional InvoiceDescription) or SingleInvoiceLine",
                ));
            }
        }

        if self.tax_category_summaries.tax_category_summary.is_empty() {
            return Err(InvoiceModelError::invalid(
                "TaxCategorySummaries",
                "must contain at least one TaxCategorySummary",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SupplierParty {
    pub party_name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CompanyTaxID"
    )]
    pub company_tax_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CompanyVATID"
    )]
    pub company_vat_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CompanyRegisterID"
    )]
    pub company_register_id: Option<String>,
    pub postal_address: PostalAddress,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CustomerParty {
    pub party_name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CompanyTaxID"
    )]
    pub company_tax_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CompanyVATID"
    )]
    pub company_vat_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "CompanyRegisterID"
    )]
    pub company_register_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_identification: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostalAddress {
    pub street_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building_number: Option<String>,
    pub city_name: String,
    pub postal_zone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub country: CountryCode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Contact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telephone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "EMail")]
    pub email: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SingleInvoiceLine {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "OrderLineID"
    )]
    pub order_line_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "DeliveryNoteLineID"
    )]
    pub delivery_note_line_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "ItemEANCode"
    )]
    pub item_ean_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_from_date: Option<Date>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_to_date: Option<Date>,
    pub invoiced_quantity: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_price_tax_exclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_price_tax_inclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_price_tax_amount: Option<Decimal>,
}

impl SingleInvoiceLine {
    fn validate(&self) -> ModelResult<()> {
        if self.item_name.is_some() == self.item_ean_code.is_some() {
            return Err(InvoiceModelError::invalid(
                "SingleInvoiceLine",
                "exactly one of ItemName and ItemEANCode is required",
            ));
        }
        if self.period_from_date.is_some() != self.period_to_date.is_some() {
            return Err(InvoiceModelError::invalid(
                "SingleInvoiceLine",
                "PeriodFromDate and PeriodToDate must occur together",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaxCategorySummaries {
    #[serde(default)]
    pub tax_category_summary: Vec<TaxCategorySummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaxCategorySummary {
    pub classified_tax_category: Percentage,
    pub tax_exclusive_amount: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_inclusive_amount: Option<Decimal>,
    pub tax_amount: Decimal,
    pub already_claimed_tax_exclusive_amount: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_claimed_tax_inclusive_amount: Option<Decimal>,
    pub already_claimed_tax_amount: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference_tax_exclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference_tax_inclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference_tax_amount: Option<Decimal>,
}

impl TaxCategorySummary {
    pub fn calculate(&self) -> TaxCategoryCalculation {
        let tax_inclusive_amount = self.tax_exclusive_amount.add_exact(&self.tax_amount);
        let already_claimed_tax_inclusive_amount = self
            .already_claimed_tax_exclusive_amount
            .add_exact(&self.already_claimed_tax_amount);
        TaxCategoryCalculation {
            tax_exclusive_amount: self.tax_exclusive_amount.clone(),
            tax_inclusive_amount: tax_inclusive_amount.clone(),
            tax_amount: self.tax_amount.clone(),
            already_claimed_tax_exclusive_amount: self.already_claimed_tax_exclusive_amount.clone(),
            already_claimed_tax_inclusive_amount: already_claimed_tax_inclusive_amount.clone(),
            already_claimed_tax_amount: self.already_claimed_tax_amount.clone(),
            difference_tax_exclusive_amount: self
                .tax_exclusive_amount
                .subtract_exact(&self.already_claimed_tax_exclusive_amount),
            difference_tax_inclusive_amount: tax_inclusive_amount
                .subtract_exact(&already_claimed_tax_inclusive_amount),
            difference_tax_amount: self
                .tax_amount
                .subtract_exact(&self.already_claimed_tax_amount),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MonetarySummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_exclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_inclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_claimed_tax_exclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_claimed_tax_inclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_claimed_tax_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference_tax_exclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference_tax_inclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difference_tax_amount: Option<Decimal>,
    pub payable_rounding_amount: Decimal,
    pub paid_deposits_amount: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payable_amount: Option<Decimal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaxCategoryCalculation {
    pub tax_exclusive_amount: Decimal,
    pub tax_inclusive_amount: Decimal,
    pub tax_amount: Decimal,
    pub already_claimed_tax_exclusive_amount: Decimal,
    pub already_claimed_tax_inclusive_amount: Decimal,
    pub already_claimed_tax_amount: Decimal,
    pub difference_tax_exclusive_amount: Decimal,
    pub difference_tax_inclusive_amount: Decimal,
    pub difference_tax_amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonetarySummaryCalculation {
    pub tax_exclusive_amount: Decimal,
    pub tax_inclusive_amount: Decimal,
    pub tax_amount: Decimal,
    pub already_claimed_tax_exclusive_amount: Decimal,
    pub already_claimed_tax_inclusive_amount: Decimal,
    pub already_claimed_tax_amount: Decimal,
    pub difference_tax_exclusive_amount: Decimal,
    pub difference_tax_inclusive_amount: Decimal,
    pub difference_tax_amount: Decimal,
    pub payable_amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceTotalsCalculation {
    pub tax_category_summaries: Vec<TaxCategoryCalculation>,
    pub monetary_summary: MonetarySummaryCalculation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleInvoiceLineCalculation {
    pub unit_price_tax_exclusive_amount: Decimal,
    pub unit_price_tax_inclusive_amount: Decimal,
    pub unit_price_tax_amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceCalculation {
    pub totals: InvoiceTotalsCalculation,
    pub single_invoice_line: Option<SingleInvoiceLineCalculation>,
}

pub trait DecimalDivision {
    type Error;

    fn divide(
        &self,
        dividend: &Decimal,
        divisor: &Decimal,
    ) -> std::result::Result<Decimal, Self::Error>;
}

pub fn try_deserialize_invoice(source: &str) -> ModelResult<Invoice> {
    let source = source.trim_start();
    if source.starts_with('<') {
        Invoice::from_xml_str(source)
    } else if source.starts_with('{') {
        serde_json::from_str(source)
            .map_err(|error| InvoiceModelError::invalid("Invoice JSON", error.to_string()))
    } else {
        Err(InvoiceModelError::invalid(
            "Invoice",
            "expected an XML document or JSON object",
        ))
    }
}

fn document_type_from_xml(source: &str) -> ModelResult<DocumentType> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(source);
    loop {
        match reader.read_event() {
            Ok(Event::Start(root)) | Ok(Event::Empty(root)) => {
                if root.local_name().as_ref() != b"Invoice" {
                    return Err(InvoiceModelError::invalid(
                        "Invoice XML",
                        "root element must be Invoice",
                    ));
                }
                for attribute in root.attributes() {
                    let attribute = attribute.map_err(|error| {
                        InvoiceModelError::invalid("Invoice XML", error.to_string())
                    })?;
                    let key = attribute.key.as_ref();
                    if key == b"type" || key.ends_with(b":type") {
                        let value = attribute
                            .decode_and_unescape_value(reader.decoder())
                            .map_err(|error| {
                                InvoiceModelError::invalid("Invoice XML", error.to_string())
                            })?;
                        return value.parse();
                    }
                }
                return Err(InvoiceModelError::invalid(
                    "DocumentType",
                    "Invoice XML root is missing xsi:type",
                ));
            }
            Ok(Event::Eof) => {
                return Err(InvoiceModelError::invalid(
                    "Invoice XML",
                    "document has no root element",
                ));
            }
            Ok(_) => {}
            Err(error) => {
                return Err(InvoiceModelError::invalid("Invoice XML", error.to_string()));
            }
        }
    }
}

fn normalize_decimal(value: &str, allow_exponent: bool) -> ModelResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(InvoiceModelError::invalid("Decimal", "must not be empty"));
    }
    let (negative, value) = match value.as_bytes()[0] {
        b'-' => (true, &value[1..]),
        b'+' => (false, &value[1..]),
        _ => (false, value),
    };
    if value.is_empty() {
        return Err(InvoiceModelError::invalid("Decimal", "must contain digits"));
    }

    let (mantissa, exponent) = match value.find(['e', 'E']) {
        Some(index) if allow_exponent => {
            let exponent = value[index + 1..]
                .parse::<i32>()
                .map_err(|_| InvoiceModelError::invalid("Decimal", "has an invalid exponent"))?;
            (&value[..index], exponent)
        }
        Some(_) => {
            return Err(InvoiceModelError::invalid(
                "Decimal",
                "exponent notation is not valid for an XSD decimal string",
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
        return Err(InvoiceModelError::invalid(
            "Decimal",
            "must use XSD decimal syntax",
        ));
    }

    let digits = format!("{integer}{fraction}");
    let decimal_position = i64::try_from(integer.len())
        .map_err(|_| InvoiceModelError::invalid("Decimal", "is too long"))?
        .checked_add(i64::from(exponent))
        .ok_or_else(|| InvoiceModelError::invalid("Decimal", "exponent is out of range"))?;
    if !(-100_000..=100_000).contains(&decimal_position) {
        return Err(InvoiceModelError::invalid(
            "Decimal",
            "exponent is outside the supported normalization range",
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
    let unsigned = if fraction.is_empty() {
        integer.to_owned()
    } else {
        format!("{integer}.{fraction}")
    };
    if negative && unsigned != "0" {
        Ok(format!("-{unsigned}"))
    } else {
        Ok(unsigned)
    }
}

#[derive(Clone, Debug)]
struct ScaledDecimal {
    negative: bool,
    digits: Vec<u8>,
    scale: usize,
}

impl From<&Decimal> for ScaledDecimal {
    fn from(value: &Decimal) -> Self {
        let (negative, value) = value
            .as_str()
            .strip_prefix('-')
            .map_or((false, value.as_str()), |value| (true, value));
        let (integer, fraction) = value.split_once('.').unwrap_or((value, ""));
        let mut digits = format!("{integer}{fraction}")
            .bytes()
            .rev()
            .map(|byte| byte - b'0')
            .collect::<Vec<_>>();
        while digits.len() > 1 && digits.last() == Some(&0) {
            digits.pop();
        }
        Self {
            negative,
            digits,
            scale: fraction.len(),
        }
    }
}

impl ScaledDecimal {
    fn is_zero(&self) -> bool {
        self.digits.iter().all(|digit| *digit == 0)
    }

    fn align(&mut self, scale: usize) {
        if scale > self.scale {
            self.digits
                .splice(0..0, std::iter::repeat_n(0, scale - self.scale));
            self.scale = scale;
        }
    }

    fn add(mut self, other: &Self) -> Self {
        let scale = self.scale.max(other.scale);
        self.align(scale);
        let mut other = other.clone();
        other.align(scale);

        if self.negative == other.negative {
            self.digits = add_magnitudes(&self.digits, &other.digits);
        } else {
            match compare_magnitudes(&self.digits, &other.digits) {
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => {
                    self.digits = subtract_magnitudes(&self.digits, &other.digits);
                }
                std::cmp::Ordering::Less => {
                    self.digits = subtract_magnitudes(&other.digits, &self.digits);
                    self.negative = other.negative;
                }
            }
        }
        if self.is_zero() {
            self.negative = false;
        }
        self
    }

    fn multiply(self, other: &Self) -> Self {
        let mut digits = vec![0_u8; self.digits.len() + other.digits.len()];
        for (left_index, left) in self.digits.iter().copied().enumerate() {
            let mut carry = 0_u16;
            for (right_index, right) in other.digits.iter().copied().enumerate() {
                let index = left_index + right_index;
                let value = u16::from(digits[index]) + u16::from(left) * u16::from(right) + carry;
                digits[index] = (value % 10) as u8;
                carry = value / 10;
            }
            let mut index = left_index + other.digits.len();
            while carry > 0 {
                if index == digits.len() {
                    digits.push(0);
                }
                let value = u16::from(digits[index]) + carry;
                digits[index] = (value % 10) as u8;
                carry = value / 10;
                index += 1;
            }
        }
        while digits.len() > 1 && digits.last() == Some(&0) {
            digits.pop();
        }
        let is_zero = digits.iter().all(|digit| *digit == 0);
        Self {
            negative: !is_zero && self.negative != other.negative,
            digits,
            scale: self.scale + other.scale,
        }
    }

    fn into_decimal(mut self) -> Decimal {
        while self.digits.len() <= self.scale {
            self.digits.push(0);
        }
        let digits = self
            .digits
            .into_iter()
            .rev()
            .map(|digit| char::from(b'0' + digit))
            .collect::<String>();
        let split = digits.len() - self.scale;
        let integer = digits[..split].trim_start_matches('0');
        let integer = if integer.is_empty() { "0" } else { integer };
        let fraction = digits[split..].trim_end_matches('0');
        let mut value = if fraction.is_empty() {
            integer.to_owned()
        } else {
            format!("{integer}.{fraction}")
        };
        if self.negative && value != "0" {
            value.insert(0, '-');
        }
        Decimal(value)
    }
}

fn compare_magnitudes(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    let significant_len = |digits: &[u8]| {
        digits
            .iter()
            .rposition(|digit| *digit != 0)
            .map_or(1, |index| index + 1)
    };
    let left_len = significant_len(left);
    let right_len = significant_len(right);
    left_len.cmp(&right_len).then_with(|| {
        left[..left_len]
            .iter()
            .rev()
            .cmp(right[..right_len].iter().rev())
    })
}

fn add_magnitudes(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(left.len().max(right.len()) + 1);
    let mut carry = 0;
    for index in 0..left.len().max(right.len()) {
        let value =
            left.get(index).copied().unwrap_or(0) + right.get(index).copied().unwrap_or(0) + carry;
        output.push(value % 10);
        carry = value / 10;
    }
    if carry != 0 {
        output.push(carry);
    }
    output
}

fn subtract_magnitudes(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(left.len());
    let mut borrow = 0_i8;
    for (index, left) in left.iter().copied().enumerate() {
        let mut value = left as i8 - borrow - right.get(index).copied().unwrap_or(0) as i8;
        if value < 0 {
            value += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        output.push(value as u8);
    }
    while output.len() > 1 && output.last() == Some(&0) {
        output.pop();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{Decimal, DocumentType, Percentage};

    #[test]
    fn normalizes_signed_decimals_without_losing_string_precision() {
        assert_eq!(Decimal::new("+0012.3400").unwrap().as_str(), "12.34");
        assert_eq!(Decimal::new("-.500").unwrap().as_str(), "-0.5");
        assert_eq!(Decimal::new("-0.000").unwrap().as_str(), "0");
        assert!(Decimal::new("1e2").is_err());
    }

    #[test]
    fn exact_decimal_addition_and_subtraction_are_scale_independent() {
        let left = Decimal::new("999999999999999999.999").unwrap();
        let right = Decimal::new("0.001").unwrap();
        assert_eq!(left.add_exact(&right).as_str(), "1000000000000000000");
        assert_eq!(
            right.subtract_exact(&left).as_str(),
            "-999999999999999999.998"
        );
    }

    #[test]
    fn exact_decimal_multiplication_handles_sign_scale_and_carry() {
        let left = Decimal::new("12.34").unwrap();
        let right = Decimal::new("-2.5").unwrap();
        assert_eq!(left.multiply_exact(&right).as_str(), "-30.85");

        let left = Decimal::new("999999999999999999.9").unwrap();
        let right = Decimal::new("99").unwrap();
        assert_eq!(
            left.multiply_exact(&right).as_str(),
            "98999999999999999990.1"
        );
        assert_eq!(
            left.multiply_exact(&Decimal::new("0").unwrap()).as_str(),
            "0"
        );
    }

    #[test]
    fn vat_percentage_is_normalized_and_bounded() {
        assert_eq!(Percentage::new("0.200").unwrap().as_str(), "0.2");
        assert!(Percentage::new("-0.1").is_err());
        assert!(Percentage::new("1.01").is_err());
        assert!(Percentage::new("20").is_err());
    }

    #[test]
    fn document_types_have_stable_classifiers() {
        for (classifier, document_type) in [
            (0, DocumentType::Invoice),
            (1, DocumentType::ProformaInvoice),
            (2, DocumentType::CreditNote),
            (3, DocumentType::DebitNote),
            (4, DocumentType::AdvanceInvoice),
        ] {
            assert_eq!(document_type.classifier(), classifier);
            assert_eq!(
                DocumentType::from_classifier(classifier).unwrap(),
                document_type
            );
        }
    }
}
