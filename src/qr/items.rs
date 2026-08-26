//! Code-native decoration for an ITEMS by square QR image.

use std::collections::HashMap;

use xmltree::{Element, XMLNode};

pub(crate) const QR_MAX_DIMENSION: u32 = 416;

const ACCENT_COLOR: &str = "#000000";
const BY_SQUARE_COLOR: &str = "#b2b4b9";

pub(crate) fn decorate(svg: &mut Element) {
    super::insert_outline(svg, ACCENT_COLOR);
    insert_items_wordmark(svg);
    super::insert_by_square_text(svg, BY_SQUARE_COLOR);
    insert_document_icon(svg);
}

fn insert_items_wordmark(svg: &mut Element) {
    // Font-independent uppercase ITEMS glyphs, aligned with the shared
    // by-square vector wordmark on the 512 × 600 canvas.
    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("fill".to_owned(), ACCENT_COLOR.to_owned()),
        ("fill-rule".to_owned(), "evenodd".to_owned()),
        (
            "d".to_owned(),
            concat!(
                // I
                "M52 540h5v29h-5z",
                // T
                "M63 540h25v5H78v24h-5v-24H63z",
                // E
                "M94 540h21v5H99v7h14v5H99v7h16v5H94z",
                // M
                "M121 569v-29h6l8 17 8-17h6v29h-5v-19l-7 15h-4l-7-15v19z",
                // S
                "M178 543l-3 5-5-3h-7l-3 3 3 4 10 2 6 6v3l-6 6h-12l-7-4 3-5 6 4h8l3-3-3-3-10-2-6-6v-4l6-6h11z",
            )
            .to_owned(),
        ),
    ]);
    svg.children.push(XMLNode::Element(path));
}

fn insert_document_icon(svg: &mut Element) {
    let mut group = Element::new("g");

    let mut background = Element::new("rect");
    background.attributes = HashMap::from([
        ("x".to_owned(), "414".to_owned()),
        ("y".to_owned(), "502".to_owned()),
        ("width".to_owned(), "94".to_owned()),
        ("height".to_owned(), "94".to_owned()),
        ("rx".to_owned(), "17".to_owned()),
        ("fill".to_owned(), ACCENT_COLOR.to_owned()),
    ]);
    group.children.push(XMLNode::Element(background));

    let mut document = Element::new("path");
    document.attributes = HashMap::from([
        (
            "d".to_owned(),
            "M440 516h25l14 14v50h-39zM465 516v14h14".to_owned(),
        ),
        ("fill".to_owned(), "none".to_owned()),
        ("stroke".to_owned(), "#ffffff".to_owned()),
        ("stroke-width".to_owned(), "4".to_owned()),
        ("stroke-linejoin".to_owned(), "miter".to_owned()),
    ]);
    group.children.push(XMLNode::Element(document));

    let mut lines = Element::new("path");
    lines.attributes = HashMap::from([
        (
            "d".to_owned(),
            "M447 541h25M447 551h25M447 561h25".to_owned(),
        ),
        ("fill".to_owned(), "none".to_owned()),
        ("stroke".to_owned(), "#ffffff".to_owned()),
        ("stroke-width".to_owned(), "3".to_owned()),
    ]);
    group.children.push(XMLNode::Element(lines));
    svg.children.push(XMLNode::Element(group));
}

#[cfg(test)]
mod tests {
    #[test]
    fn complete_items_svg_contains_vector_branding() {
        let svg =
            String::from_utf8(super::super::create_invoice_items_svg("ITEMS-FIXTURE")).unwrap();
        assert!(svg.contains("M52 540h5v29h-5z"));
        assert!(svg.contains("M440 516h25l14 14v50h-39z"));
        assert!(!svg.contains("<text"));
    }
}
