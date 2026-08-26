use bysqr::invoice::{self, try_deserialize_invoice, DocumentType, JSON_SCHEMA};
use serde_json::{json, Value};

const MINIMAL: &str = include_str!("fixtures/invoice/schema/minimal-header-invoice.json");

fn schema() -> Value {
    serde_json::from_str(JSON_SCHEMA).expect("Invoice JSON Schema must be valid JSON")
}

fn validator() -> jsonschema::Validator {
    jsonschema::draft202012::new(&schema()).expect("Invoice JSON Schema must compile")
}

fn minimal() -> Value {
    serde_json::from_str(MINIMAL).expect("minimal Invoice fixture must be valid JSON")
}

#[test]
fn invoice_schema_is_valid_draft_2020_12_and_accepts_all_document_types() {
    let schema = schema();
    jsonschema::draft202012::meta::validate(&schema).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();

    for document_type in [
        "Invoice",
        "ProformaInvoice",
        "CreditNote",
        "DebitNote",
        "AdvanceInvoice",
    ] {
        let mut document = minimal();
        document["DocumentType"] = json!(document_type);
        let errors = validator
            .iter_errors(&document)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{document_type}: {errors:#?}");
    }
}

#[test]
fn schema_enforces_required_fields_choices_patterns_and_vat_range() {
    let validator = validator();
    let valid = minimal();
    let mut cases = Vec::new();

    let mut missing_type = valid.clone();
    missing_type.as_object_mut().unwrap().remove("DocumentType");
    cases.push(("missing DocumentType", missing_type));

    let mut unknown_type = valid.clone();
    unknown_type["DocumentType"] = json!("Receipt");
    cases.push(("unknown DocumentType", unknown_type));

    let mut numeric_decimal = valid.clone();
    numeric_decimal["TaxCategorySummaries"]["TaxCategorySummary"][0]["TaxAmount"] = json!(20);
    cases.push(("numeric canonical decimal", numeric_decimal));

    let mut percentage_points = valid.clone();
    percentage_points["TaxCategorySummaries"]["TaxCategorySummary"][0]["ClassifiedTaxCategory"] =
        json!("20");
    cases.push(("VAT outside [0,1]", percentage_points));

    let mut partial_foreign_currency = valid.clone();
    partial_foreign_currency["ForeignCurrencyCode"] = json!("USD");
    cases.push(("partial foreign currency group", partial_foreign_currency));

    let mut both_line_choices = valid.clone();
    both_line_choices["SingleInvoiceLine"] = json!({
        "ItemName": "Consulting",
        "InvoicedQuantity": "1"
    });
    cases.push(("both invoice-line choices", both_line_choices));

    let mut neither_line_choice = valid.clone();
    neither_line_choice
        .as_object_mut()
        .unwrap()
        .remove("NumberOfInvoiceLines");
    neither_line_choice
        .as_object_mut()
        .unwrap()
        .remove("InvoiceDescription");
    cases.push(("no invoice-line choice", neither_line_choice));

    let mut empty_tax_summaries = valid.clone();
    empty_tax_summaries["TaxCategorySummaries"]["TaxCategorySummary"] = json!([]);
    cases.push(("empty tax summaries", empty_tax_summaries));

    let mut invalid_currency = valid.clone();
    invalid_currency["LocalCurrencyCode"] = json!("eur");
    cases.push(("invalid currency pattern", invalid_currency));

    let mut invalid_date = valid;
    invalid_date["IssueDate"] = json!("2026/08/26");
    cases.push(("invalid canonical date", invalid_date));

    for (name, document) in cases {
        assert!(!validator.is_valid(&document), "{name} was accepted");
    }
}

#[test]
fn single_line_item_and_period_choices_are_hard_constraints() {
    let validator = validator();
    let mut single = minimal();
    single
        .as_object_mut()
        .unwrap()
        .remove("NumberOfInvoiceLines");
    single.as_object_mut().unwrap().remove("InvoiceDescription");
    single["SingleInvoiceLine"] = json!({
        "ItemName": "Consulting",
        "PeriodFromDate": "2026-08-01",
        "PeriodToDate": "2026-08-31",
        "InvoicedQuantity": "1"
    });
    assert!(validator.is_valid(&single));

    let mut both_items = single.clone();
    both_items["SingleInvoiceLine"]["ItemEANCode"] = json!("1234567890123");
    assert!(!validator.is_valid(&both_items));

    let mut incomplete_period = single;
    incomplete_period["SingleInvoiceLine"]
        .as_object_mut()
        .unwrap()
        .remove("PeriodToDate");
    assert!(!validator.is_valid(&incomplete_period));
}

#[test]
fn bsqr_max_lengths_are_advisory_and_computed_fields_are_read_only() {
    fn contains_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(object) => {
                object.contains_key(key) || object.values().any(|value| contains_key(value, key))
            }
            Value::Array(array) => array.iter().any(|value| contains_key(value, key)),
            _ => false,
        }
    }

    let schema = schema();
    assert!(!contains_key(&schema, "maxLength"));
    assert_eq!(
        schema["$defs"]["TaxCategorySummary"]["properties"]["TaxInclusiveAmount"]["readOnly"],
        json!(true)
    );
    assert_eq!(
        schema["$defs"]["MonetarySummary"]["properties"]["PayableAmount"]["readOnly"],
        json!(true)
    );
    assert_eq!(
        schema["$defs"]["SingleInvoiceLine"]["properties"]["UnitPriceTaxExclusiveAmount"]
            ["readOnly"],
        json!(true)
    );

    let mut over_advisory_limit = minimal();
    over_advisory_limit["InvoiceID"] = json!("Ž".repeat(11));
    over_advisory_limit["SupplierParty"]["PartyName"] = json!("é".repeat(21));
    let mut second_tax_summary =
        over_advisory_limit["TaxCategorySummaries"]["TaxCategorySummary"][0].clone();
    second_tax_summary["TaxExclusiveAmount"] = json!("1234567890123456");
    over_advisory_limit["TaxCategorySummaries"]["TaxCategorySummary"]
        .as_array_mut()
        .unwrap()
        .push(second_tax_summary);
    assert!(validator().is_valid(&over_advisory_limit));

    let invoice = try_deserialize_invoice(&over_advisory_limit.to_string()).unwrap();
    invoice.validate().unwrap();
    assert_eq!(
        invoice.advisory_diagnostics(),
        [
            invoice::AdvisoryDiagnostic {
                field_path: "InvoiceID".to_owned(),
                actual_character_count: 11,
                recommended_maximum: 10,
            },
            invoice::AdvisoryDiagnostic {
                field_path: "SupplierParty.PartyName".to_owned(),
                actual_character_count: 21,
                recommended_maximum: 20,
            },
            invoice::AdvisoryDiagnostic {
                field_path: "TaxCategorySummaries.TaxCategorySummary[1].TaxExclusiveAmount"
                    .to_owned(),
                actual_character_count: 16,
                recommended_maximum: 15,
            },
        ]
    );

    assert!(try_deserialize_invoice(MINIMAL)
        .unwrap()
        .advisory_diagnostics()
        .is_empty());
}

#[test]
fn rust_model_accepts_numeric_input_but_emits_lossless_decimal_strings() {
    let mut source = minimal();
    source["TaxCategorySummaries"]["TaxCategorySummary"][0]["TaxExclusiveAmount"] = json!(100.5);
    source["TaxCategorySummaries"]["TaxCategorySummary"][0]["TaxAmount"] = json!(20.1);
    source["MonetarySummary"]["PayableRoundingAmount"] = json!(-0.05);

    let invoice = try_deserialize_invoice(&source.to_string()).unwrap();
    let canonical = serde_json::to_value(&invoice).unwrap();
    assert_eq!(
        canonical["TaxCategorySummaries"]["TaxCategorySummary"][0]["TaxExclusiveAmount"],
        json!("100.5")
    );
    assert_eq!(
        canonical["MonetarySummary"]["PayableRoundingAmount"],
        json!("-0.05")
    );
    assert!(validator().is_valid(&canonical));
}

#[test]
fn xml_xsi_type_round_trips_the_same_semantic_document_type() {
    let base = try_deserialize_invoice(MINIMAL).unwrap();
    for document_type in [
        DocumentType::Invoice,
        DocumentType::ProformaInvoice,
        DocumentType::CreditNote,
        DocumentType::DebitNote,
        DocumentType::AdvanceInvoice,
    ] {
        let mut invoice = base.clone();
        invoice.document_type = document_type;
        let xml = invoice.to_xml_string().unwrap();
        assert!(xml.contains(&format!("xsi:type=\"{}\"", document_type.as_xsi_type())));
        let decoded = try_deserialize_invoice(&xml).unwrap();
        assert_eq!(decoded, invoice);

        let qualified = xml.replace(
            &format!("xsi:type=\"{}\"", document_type.as_xsi_type()),
            &format!("xsi:type=\"bsqr:{}\"", document_type.as_xsi_type()),
        );
        assert_eq!(try_deserialize_invoice(&qualified).unwrap(), invoice);
    }
}

#[test]
fn exact_calculation_api_ignores_optional_computed_transport_fields() {
    let invoice = try_deserialize_invoice(MINIMAL).unwrap();
    let calculated = invoice.calculate_totals();
    let tax = &calculated.tax_category_summaries[0];

    assert_eq!(tax.tax_inclusive_amount.as_str(), "120");
    assert_eq!(tax.difference_tax_exclusive_amount.as_str(), "100");
    assert_eq!(calculated.monetary_summary.tax_amount.as_str(), "20");
    assert_eq!(calculated.monetary_summary.payable_amount.as_str(), "120");
}
