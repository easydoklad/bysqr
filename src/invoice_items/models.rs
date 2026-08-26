use serde::{Deserialize, Deserializer, Serialize};

use crate::invoice::{
    AdvisoryDiagnostic, Date, Decimal, InvoiceModelError, ModelResult, Percentage,
};

const BYSQUARE_NAMESPACE: &str = "http://www.bysquare.com/bysquare";
const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// One INVOICE ITEMS by square document (one independently encoded QR block).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename = "InvoiceItems", rename_all = "PascalCase")]
pub struct InvoiceItems {
    #[serde(rename = "InvoiceID")]
    pub invoice_id: String,
    #[serde(rename = "FirstInvoiceLineID")]
    pub first_invoice_line_id: String,
    pub invoice_lines: InvoiceLines,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InvoiceItemsWire {
    #[serde(rename = "InvoiceID")]
    invoice_id: String,
    #[serde(rename = "FirstInvoiceLineID")]
    first_invoice_line_id: String,
    invoice_lines: InvoiceLines,
}

impl<'de> Deserialize<'de> for InvoiceItems {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = InvoiceItemsWire::deserialize(deserializer)?;
        Self::new(
            wire.invoice_id,
            wire.first_invoice_line_id,
            wire.invoice_lines,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl InvoiceItems {
    pub fn new(
        invoice_id: impl Into<String>,
        first_invoice_line_id: impl Into<String>,
        invoice_lines: InvoiceLines,
    ) -> ModelResult<Self> {
        let document = Self {
            invoice_id: invoice_id.into(),
            first_invoice_line_id: first_invoice_line_id.into(),
            invoice_lines,
        };
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> ModelResult<()> {
        if self.invoice_lines.invoice_line.is_empty() {
            return Err(InvoiceModelError::invalid(
                "InvoiceLines",
                "must contain at least one InvoiceLine",
            ));
        }
        for line in &self.invoice_lines.invoice_line {
            line.validate()?;
        }
        Ok(())
    }

    /// Reports advisory `bsqr:maxLength` overflows without rejecting data.
    pub fn advisory_diagnostics(&self) -> Vec<AdvisoryDiagnostic> {
        let mut diagnostics = Vec::new();
        diagnose(
            &mut diagnostics,
            "InvoiceID".to_owned(),
            &self.invoice_id,
            10,
        );
        diagnose(
            &mut diagnostics,
            "FirstInvoiceLineID".to_owned(),
            &self.first_invoice_line_id,
            10,
        );
        for (index, line) in self.invoice_lines.invoice_line.iter().enumerate() {
            let base = format!("InvoiceLines.InvoiceLine[{index}]");
            if let Some(reference) = &line.order_reference {
                diagnose_optional(
                    &mut diagnostics,
                    format!("{base}.OrderReference.OrderID"),
                    reference.order_id.as_deref(),
                    10,
                );
                diagnose_optional(
                    &mut diagnostics,
                    format!("{base}.OrderReference.OrderLineID"),
                    reference.order_line_id.as_deref(),
                    10,
                );
            }
            if let Some(reference) = &line.delivery_note_reference {
                diagnose_optional(
                    &mut diagnostics,
                    format!("{base}.DeliveryNoteReference.DeliveryNoteID"),
                    reference.delivery_note_id.as_deref(),
                    10,
                );
                diagnose_optional(
                    &mut diagnostics,
                    format!("{base}.DeliveryNoteReference.DeliveryNoteLineID"),
                    reference.delivery_note_line_id.as_deref(),
                    10,
                );
            }
            diagnose_optional(
                &mut diagnostics,
                format!("{base}.ItemName"),
                line.item_name.as_deref(),
                30,
            );
            diagnose_optional(
                &mut diagnostics,
                format!("{base}.ItemEANCode"),
                line.item_ean_code.as_deref(),
                30,
            );
            if let Some(value) = &line.period_from_date {
                diagnose(
                    &mut diagnostics,
                    format!("{base}.PeriodFromDate"),
                    &value.as_str().replace('-', ""),
                    8,
                );
            }
            if let Some(value) = &line.period_to_date {
                diagnose(
                    &mut diagnostics,
                    format!("{base}.PeriodToDate"),
                    &value.as_str().replace('-', ""),
                    8,
                );
            }
            for (name, value) in [
                ("InvoicedQuantity", line.invoiced_quantity.as_str()),
                (
                    "UnitPriceTaxExclusiveAmount",
                    line.unit_price_tax_exclusive_amount.as_str(),
                ),
                ("UnitPriceTaxAmount", line.unit_price_tax_amount.as_str()),
                (
                    "ClassifiedTaxCategory",
                    line.classified_tax_category.as_str(),
                ),
            ] {
                diagnose(&mut diagnostics, format!("{base}.{name}"), value, 15);
            }
        }
        diagnostics
    }

    pub fn from_xml_str(source: &str) -> ModelResult<Self> {
        validate_xml_root(source)?;
        quick_xml::de::from_str(source)
            .map_err(|error| InvoiceModelError::invalid("InvoiceItems XML", error.to_string()))
    }

    pub fn to_xml_string(&self) -> ModelResult<String> {
        self.validate()?;
        let body = quick_xml::se::to_string(self)
            .map_err(|error| InvoiceModelError::invalid("InvoiceItems XML", error.to_string()))?;
        let end = body.find('>').ok_or_else(|| {
            InvoiceModelError::invalid("InvoiceItems XML", "serialized root element has no end")
        })?;
        if !body[..end].starts_with("<InvoiceItems") {
            return Err(InvoiceModelError::invalid(
                "InvoiceItems XML",
                "serialized root element is not InvoiceItems",
            ));
        }
        Ok(format!(
            "{} xmlns=\"{}\" xmlns:xsi=\"{}\" xsi:type=\"InvoiceItems\"{}",
            &body[..end],
            BYSQUARE_NAMESPACE,
            XSI_NAMESPACE,
            &body[end..]
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct InvoiceLines {
    #[serde(default, rename = "InvoiceLine")]
    pub invoice_line: Vec<InvoiceLine>,
}

impl InvoiceLines {
    pub fn new(invoice_line: Vec<InvoiceLine>) -> Self {
        Self { invoice_line }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct InvoiceLine {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_reference: Option<OrderReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_note_reference: Option<DeliveryNoteReference>,
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
    pub unit_price_tax_exclusive_amount: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_price_tax_inclusive_amount: Option<Decimal>,
    pub unit_price_tax_amount: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_tax_exclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_tax_inclusive_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_tax_amount: Option<Decimal>,
    pub classified_tax_category: Percentage,
}

impl InvoiceLine {
    pub fn validate(&self) -> ModelResult<()> {
        if self.item_name.is_some() == self.item_ean_code.is_some() {
            return Err(InvoiceModelError::invalid(
                "InvoiceLine",
                "exactly one of ItemName and ItemEANCode is required",
            ));
        }
        if self.period_from_date.is_some() != self.period_to_date.is_some() {
            return Err(InvoiceModelError::invalid(
                "InvoiceLine",
                "PeriodFromDate and PeriodToDate must occur together",
            ));
        }
        Ok(())
    }

    /// Calculates all fields marked `bsqr:computed=true` in the XSD.
    pub fn calculate(&self) -> InvoiceLineCalculation {
        let unit_price_tax_inclusive_amount = self
            .unit_price_tax_exclusive_amount
            .add_exact(&self.unit_price_tax_amount);
        InvoiceLineCalculation {
            unit_price_tax_inclusive_amount: unit_price_tax_inclusive_amount.clone(),
            line_tax_exclusive_amount: self
                .unit_price_tax_exclusive_amount
                .multiply_exact(&self.invoiced_quantity),
            line_tax_inclusive_amount: unit_price_tax_inclusive_amount
                .multiply_exact(&self.invoiced_quantity),
            line_tax_amount: self
                .unit_price_tax_amount
                .multiply_exact(&self.invoiced_quantity),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OrderReference {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "OrderID")]
    pub order_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "OrderLineID"
    )]
    pub order_line_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeliveryNoteReference {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "DeliveryNoteID"
    )]
    pub delivery_note_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "DeliveryNoteLineID"
    )]
    pub delivery_note_line_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceLineCalculation {
    pub unit_price_tax_inclusive_amount: Decimal,
    pub line_tax_exclusive_amount: Decimal,
    pub line_tax_inclusive_amount: Decimal,
    pub line_tax_amount: Decimal,
}

/// Deserialize canonical INVOICE ITEMS JSON or XML.
pub fn try_deserialize_invoice_items(source: &str) -> ModelResult<InvoiceItems> {
    let source = source.trim_start();
    if source.starts_with('<') {
        InvoiceItems::from_xml_str(source)
    } else if source.starts_with('{') {
        serde_json::from_str(source)
            .map_err(|error| InvoiceModelError::invalid("InvoiceItems JSON", error.to_string()))
    } else {
        Err(InvoiceModelError::invalid(
            "InvoiceItems",
            "expected an XML document or JSON object",
        ))
    }
}

fn validate_xml_root(source: &str) -> ModelResult<()> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(source);
    loop {
        match reader.read_event() {
            Ok(Event::Start(root)) | Ok(Event::Empty(root)) => {
                if root.local_name().as_ref() != b"InvoiceItems" {
                    return Err(InvoiceModelError::invalid(
                        "InvoiceItems XML",
                        "root element must be InvoiceItems",
                    ));
                }
                for attribute in root.attributes() {
                    let attribute = attribute.map_err(|error| {
                        InvoiceModelError::invalid("InvoiceItems XML", error.to_string())
                    })?;
                    let key = attribute.key.as_ref();
                    if key == b"type" || key.ends_with(b":type") {
                        let value = attribute
                            .decode_and_unescape_value(reader.decoder())
                            .map_err(|error| {
                                InvoiceModelError::invalid("InvoiceItems XML", error.to_string())
                            })?;
                        let local = value.rsplit_once(':').map_or(value.as_ref(), |(_, v)| v);
                        if local != "InvoiceItems" {
                            return Err(InvoiceModelError::invalid(
                                "InvoiceItems XML",
                                format!("unknown xsi:type {value:?}"),
                            ));
                        }
                    }
                }
                return Ok(());
            }
            Ok(Event::Eof) => {
                return Err(InvoiceModelError::invalid(
                    "InvoiceItems XML",
                    "document has no root element",
                ));
            }
            Ok(_) => {}
            Err(error) => {
                return Err(InvoiceModelError::invalid(
                    "InvoiceItems XML",
                    error.to_string(),
                ));
            }
        }
    }
}

fn diagnose(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: String,
    value: &str,
    recommended_maximum: usize,
) {
    let actual_character_count = value.chars().count();
    if actual_character_count > recommended_maximum {
        diagnostics.push(AdvisoryDiagnostic {
            field_path,
            actual_character_count,
            recommended_maximum,
        });
    }
}

fn diagnose_optional(
    diagnostics: &mut Vec<AdvisoryDiagnostic>,
    field_path: String,
    value: Option<&str>,
    recommended_maximum: usize,
) {
    if let Some(value) = value {
        diagnose(diagnostics, field_path, value, recommended_maximum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> InvoiceLine {
        InvoiceLine {
            order_reference: None,
            delivery_note_reference: None,
            item_name: Some("Service".to_owned()),
            item_ean_code: None,
            period_from_date: None,
            period_to_date: None,
            invoiced_quantity: Decimal::new("2.5").unwrap(),
            unit_price_tax_exclusive_amount: Decimal::new("10.2").unwrap(),
            unit_price_tax_inclusive_amount: None,
            unit_price_tax_amount: Decimal::new("2.04").unwrap(),
            line_tax_exclusive_amount: None,
            line_tax_inclusive_amount: None,
            line_tax_amount: None,
            classified_tax_category: Percentage::new("0.2").unwrap(),
        }
    }

    #[test]
    fn calculates_computed_line_values_exactly() {
        let calculated = line().calculate();
        assert_eq!(calculated.unit_price_tax_inclusive_amount.as_str(), "12.24");
        assert_eq!(calculated.line_tax_exclusive_amount.as_str(), "25.5");
        assert_eq!(calculated.line_tax_inclusive_amount.as_str(), "30.6");
        assert_eq!(calculated.line_tax_amount.as_str(), "5.1");
    }

    #[test]
    fn validates_choices_and_period_pair() {
        let mut invalid = line();
        invalid.item_ean_code = Some("8580000000000".to_owned());
        assert_eq!(invalid.validate().unwrap_err().field(), "InvoiceLine");

        let mut invalid = line();
        invalid.period_from_date = Some(Date::new("2026-08-01").unwrap());
        assert_eq!(invalid.validate().unwrap_err().field(), "InvoiceLine");
    }

    #[test]
    fn json_and_xml_round_trip() {
        let document = InvoiceItems::new("INV-1", "1", InvoiceLines::new(vec![line()])).unwrap();
        let json = serde_json::to_string(&document).unwrap();
        assert_eq!(
            serde_json::from_str::<InvoiceItems>(&json).unwrap(),
            document
        );

        let xml = document.to_xml_string().unwrap();
        assert!(xml.contains("xsi:type=\"InvoiceItems\""));
        assert_eq!(InvoiceItems::from_xml_str(&xml).unwrap(), document);
    }

    #[test]
    fn accepts_deployed_xml_order_and_non_xsd_type_hint() {
        let xml = r#"<InvoiceItems type="InvoiceItems">
          <InvoiceLines><InvoiceLine>
            <Type>item</Type><ItemName>Service</ItemName>
            <ClassifiedTaxCategory>0.2</ClassifiedTaxCategory>
            <UnitPriceTaxExclusiveAmount>10</UnitPriceTaxExclusiveAmount>
            <UnitPriceTaxAmount>2</UnitPriceTaxAmount><InvoicedQuantity>1</InvoicedQuantity>
            <OrderReference/><DeliveryNoteReference/>
          </InvoiceLine></InvoiceLines>
          <FirstInvoiceLineID>1</FirstInvoiceLineID><InvoiceID>INV-1</InvoiceID>
        </InvoiceItems>"#;
        let document = InvoiceItems::from_xml_str(xml).unwrap();
        assert_eq!(document.invoice_id, "INV-1");
        assert_eq!(document.invoice_lines.invoice_line.len(), 1);
        assert_eq!(
            document.invoice_lines.invoice_line[0].item_name.as_deref(),
            Some("Service")
        );
    }
}
