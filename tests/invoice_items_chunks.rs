use bysqr::{
    invoice::{self, Decimal, Percentage},
    invoice_items::{
        chunk_invoice_items_list, decode_chunks, encode_chunks, encode_invoice_items_list,
        reassemble_invoice_lines, InvoiceItemsList, InvoiceLine, InvoiceLines, JSON_SCHEMA_LIST,
    },
};

fn line(index: usize) -> InvoiceLine {
    InvoiceLine {
        order_reference: None,
        delivery_note_reference: None,
        item_name: Some(format!("Item {index}")),
        item_ean_code: None,
        period_from_date: None,
        period_to_date: None,
        invoiced_quantity: Decimal::new("1").unwrap(),
        unit_price_tax_exclusive_amount: Decimal::new("10").unwrap(),
        unit_price_tax_inclusive_amount: None,
        unit_price_tax_amount: Decimal::new("2").unwrap(),
        line_tax_exclusive_amount: None,
        line_tax_inclusive_amount: None,
        line_tax_amount: None,
        classified_tax_category: Percentage::new("0.2").unwrap(),
    }
}

fn list(invoice_id: &str, count: usize) -> InvoiceItemsList {
    InvoiceItemsList::new(
        invoice_id,
        InvoiceLines::new((1..=count).map(line).collect()),
    )
    .unwrap()
}

#[test]
fn aggregate_chunk_boundaries_have_exact_first_line_ids_and_unchanged_payloads() {
    for (count, expected_sizes, expected_first_ids) in [
        (1, vec![1], vec!["1"]),
        (4, vec![4], vec!["1"]),
        (5, vec![4, 1], vec!["1", "5"]),
        (9, vec![4, 4, 1], vec!["1", "5", "9"]),
    ] {
        let document = list("INV-BOUND", count);
        let blocks = chunk_invoice_items_list(&document).unwrap();

        assert_eq!(
            blocks
                .iter()
                .map(|block| block.invoice_lines.invoice_line.len())
                .collect::<Vec<_>>(),
            expected_sizes,
            "wrong block sizes for {count} lines"
        );
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.first_invoice_line_id.as_str())
                .collect::<Vec<_>>(),
            expected_first_ids,
            "wrong first IDs for {count} lines"
        );

        let aggregate_payloads = document.encode_chunks().unwrap();
        assert_eq!(
            aggregate_payloads,
            encode_invoice_items_list(&document).unwrap()
        );
        assert_eq!(
            aggregate_payloads,
            encode_chunks(
                document.invoice_id.clone(),
                document.invoice_lines.invoice_line.clone()
            )
            .unwrap(),
            "aggregate API changed the established block payload for {count} lines"
        );
        assert_eq!(
            decode_chunks(aggregate_payloads.iter().rev()).unwrap(),
            document,
            "out-of-order round trip failed for {count} lines"
        );
    }
}

#[test]
fn reassembly_rejects_gaps_overlaps_duplicates_and_mixed_invoice_ids() {
    let blocks = chunk_invoice_items_list(&list("INV-A", 9)).unwrap();

    let mut gap = blocks.clone();
    gap[1].first_invoice_line_id = "6".to_owned();
    assert_eq!(
        reassemble_invoice_lines(gap).unwrap_err().field(),
        "FirstInvoiceLineID"
    );

    let mut overlap = blocks.clone();
    overlap[1].first_invoice_line_id = "4".to_owned();
    assert_eq!(
        reassemble_invoice_lines(overlap).unwrap_err().field(),
        "FirstInvoiceLineID"
    );

    let mut duplicate = blocks.clone();
    duplicate.push(blocks[0].clone());
    assert_eq!(
        reassemble_invoice_lines(duplicate).unwrap_err().field(),
        "FirstInvoiceLineID"
    );

    let mut mixed = blocks;
    mixed[2].invoice_id = "INV-B".to_owned();
    assert_eq!(
        reassemble_invoice_lines(mixed).unwrap_err().field(),
        "InvoiceID"
    );
}

#[test]
fn reassembly_rejects_malformed_first_line_ids() {
    for malformed in ["", "0", "-1", "one", "１２", "184467440737095516160"] {
        let mut block = chunk_invoice_items_list(&list("INV-A", 1))
            .unwrap()
            .pop()
            .unwrap();
        block.first_invoice_line_id = malformed.to_owned();
        let error = reassemble_invoice_lines([block]).unwrap_err();
        assert_eq!(
            error.field(),
            "FirstInvoiceLineID",
            "accepted {malformed:?}"
        );
    }
}

#[test]
fn aggregate_validates_parent_invoice_id_and_declared_line_count() {
    let parent = invoice::try_deserialize_invoice(include_str!(
        "fixtures/invoice/valid-interoperability-offline-multiple-lines.json"
    ))
    .unwrap();
    let document = list("INV-MULTI-2026", 3);
    document.validate_against_invoice(&parent).unwrap();

    let wrong_id = list("OTHER", 3);
    assert_eq!(
        wrong_id
            .validate_against_invoice(&parent)
            .unwrap_err()
            .field(),
        "InvoiceID"
    );

    let wrong_count = list("INV-MULTI-2026", 2);
    assert_eq!(
        wrong_count
            .validate_against_invoice(&parent)
            .unwrap_err()
            .field(),
        "InvoiceLines"
    );

    let mut missing_count = parent;
    missing_count.data.number_of_invoice_lines = None;
    assert_eq!(
        document
            .validate_against_invoice(&missing_count)
            .unwrap_err()
            .field(),
        "NumberOfInvoiceLines"
    );
}

#[test]
fn standalone_aggregate_schema_is_exported() {
    let schema: serde_json::Value = serde_json::from_str(JSON_SCHEMA_LIST).unwrap();
    assert_eq!(schema["title"], "Invoice items list");
    assert!(jsonschema::draft202012::new(&schema).is_ok());
}
