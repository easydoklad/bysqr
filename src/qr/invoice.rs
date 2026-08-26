//! Code-native decoration for an INVOICE by square QR image.
//!
//! This module intentionally owns only the branding layer. QR generation and
//! placement stay in the parent module so the matrix can remain square and
//! unobstructed.

use std::collections::HashMap;

use xmltree::{Element, XMLNode};

/// Maximum width and height of the QR matrix on the 512-unit parent canvas.
///
/// Centering a matrix no larger than this leaves at least 48 units on every
/// side. That whitespace is the QR quiet area; none of the decoration below
/// enters it.
pub(crate) const QR_MAX_DIMENSION: u32 = 416;

const ACCENT_COLOR: &str = "#f78f1e";
const BY_SQUARE_COLOR: &str = "#b2b4b9";

/// Add the official-color Invoice frame, wordmark, and document icon.
///
/// The caller must insert the white background and centered QR matrix first.
/// Keeping this as a decoration-only hook makes it impossible for the branding
/// to resize, translate, or cover the QR matrix.
pub(crate) fn decorate(svg: &mut Element) {
    super::insert_outline(svg, ACCENT_COLOR);
    insert_invoice_wordmark(svg);
    insert_by_square_wordmark(svg);
    insert_document_icon(svg);
}

fn insert_invoice_wordmark(svg: &mut Element) {
    // Hand-built, font-independent uppercase glyphs. Bounds: 155 x 29 at
    // (16, 540), aligned with the baseline of the existing by-square path.
    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("fill".to_string(), ACCENT_COLOR.to_string()),
        ("fill-rule".to_string(), "evenodd".to_string()),
        (
            "d".to_string(),
            concat!(
                // I
                "M16 540h5v29h-5z",
                // N
                "M28 569v-29h5l12 19v-19h5v29h-5l-12-19v19z",
                // V
                "M54 540h5l7 20 7-20h5l-10 29h-4z",
                // O (outer and counter)
                "M89 540h11l7 7v15l-7 7H89l-7-7v-15z",
                "M91 545l-4 4v11l4 4h7l4-4v-11l-4-4z",
                // I
                "M111 540h5v29h-5z",
                // C
                "M146 543l-4 5-4-3h-7l-3 4v11l3 4h7l4-3 4 5-6 3h-11l-6-7v-15l6-7h11z",
                // E
                "M150 540h21v5h-16v7h14v5h-14v7h16v5h-21z",
            )
            .to_string(),
        ),
    ]);
    svg.children.push(XMLNode::Element(path));
}

fn insert_by_square_wordmark(svg: &mut Element) {
    // Reuse the parent's font-independent by-square outline. The parent path
    // starts at x=193 for the shorter PAY label; shift it left to x=185 to
    // preserve the spacing seen in the Invoice reference.
    let mut group = Element::new("g");
    group.attributes = HashMap::from([("transform".to_string(), "translate(-8,0)".to_string())]);
    super::insert_by_square_text(&mut group, BY_SQUARE_COLOR);
    svg.children.push(XMLNode::Element(group));
}

fn insert_document_icon(svg: &mut Element) {
    // Bounds: x=414..508, y=502..596. This stays entirely below and to the
    // right of the open ends of the frame and outside the QR quiet area.
    let mut group = Element::new("g");

    let mut background = Element::new("rect");
    background.attributes = HashMap::from([
        ("x".to_string(), "414".to_string()),
        ("y".to_string(), "502".to_string()),
        ("width".to_string(), "94".to_string()),
        ("height".to_string(), "94".to_string()),
        ("rx".to_string(), "17".to_string()),
        ("fill".to_string(), ACCENT_COLOR.to_string()),
    ]);
    group.children.push(XMLNode::Element(background));

    let mut document = Element::new("path");
    document.attributes = HashMap::from([
        (
            "d".to_string(),
            "M440 516h25l14 14v50h-39zM465 516v14h14".to_string(),
        ),
        ("fill".to_string(), "none".to_string()),
        ("stroke".to_string(), "#ffffff".to_string()),
        ("stroke-width".to_string(), "4".to_string()),
        ("stroke-linejoin".to_string(), "miter".to_string()),
    ]);
    group.children.push(XMLNode::Element(document));

    let mut lines = Element::new("path");
    lines.attributes = HashMap::from([
        (
            "d".to_string(),
            "M447 541h25M447 551h25M447 561h25".to_string(),
        ),
        ("fill".to_string(), "none".to_string()),
        ("stroke".to_string(), "#ffffff".to_string()),
        ("stroke-width".to_string(), "3".to_string()),
    ]);
    group.children.push(XMLNode::Element(lines));

    svg.children.push(XMLNode::Element(group));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoration_has_four_non_qr_layers() {
        let mut svg = Element::new("svg");

        decorate(&mut svg);

        assert_eq!(svg.children.len(), 4);
    }

    #[test]
    fn recommended_qr_dimension_preserves_large_canvas_clearance() {
        let clearance = (super::super::CONTAINER_WIDTH as u32 - QR_MAX_DIMENSION) / 2;
        assert_eq!(clearance, 48);
    }

    #[test]
    fn complete_invoice_svg_contains_vector_branding() {
        let svg = String::from_utf8(super::super::create_invoice_svg("INVOICE-FIXTURE")).unwrap();

        assert!(svg.contains(ACCENT_COLOR));
        assert!(svg.contains("M440 516h25l14 14v50h-39z"));
        assert!(!svg.contains("<text"));
    }
}
