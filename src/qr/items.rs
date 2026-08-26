//! Code-native decoration for an ITEMS by square QR image.

use xmltree::Element;

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
    super::branding::ITEMS_WORDMARK.insert_right_aligned(svg, 179.0, 540.0, 29.0, ACCENT_COLOR);
}

fn insert_document_icon(svg: &mut Element) {
    super::branding::INVOICE_ICON.insert_at(svg, 414.0, 502.0, 94.0, ACCENT_COLOR);
}

#[cfg(test)]
mod tests {
    #[test]
    fn complete_items_svg_contains_vector_branding() {
        let svg =
            String::from_utf8(super::super::create_invoice_items_svg("ITEMS-FIXTURE")).unwrap();
        assert!(svg.contains("M104.382 0C106.433"));
        assert!(svg.contains("M174.6769.2864"));
        assert!(!svg.contains("<text"));
        assert!(!svg.contains("<image"));
    }
}
