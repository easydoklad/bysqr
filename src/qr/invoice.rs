//! Code-native decoration for an INVOICE by square QR image.
//!
//! This module intentionally owns only the branding layer. QR generation and
//! placement stay in the parent module so the matrix can remain square and
//! unobstructed.

use xmltree::Element;

use super::LogoTheme;

/// Maximum size of the QR matrix including its renderer-provided four-module
/// quiet zone. The remaining four canvas units separate it from the frame.
pub(crate) const QR_MAX_DIMENSION: u32 = 504;

/// Add the orange Invoice frame, wordmark, and document icon.
///
/// The caller must insert the white background and centered QR matrix first.
/// Keeping this as a decoration-only hook makes it impossible for the branding
/// to resize, translate, or cover the QR matrix.
pub(crate) fn decorate(svg: &mut Element, theme: LogoTheme) {
    super::logo::decorate(
        svg,
        theme,
        theme.color.invoice_hex(),
        bottom_wordmarks,
        insert_document_icon,
    );
}

fn bottom_wordmarks(color: &str, by_square_color: &str) -> Element {
    let mut group = Element::new("g");
    insert_invoice_wordmark(&mut group, color);
    insert_by_square_wordmark(&mut group, by_square_color);
    group
}

fn insert_invoice_wordmark(svg: &mut Element, color: &str) {
    super::branding::INVOICE_WORDMARK.insert_at(svg, 16.0, 540.0, 29.0, color);
}

fn insert_by_square_wordmark(svg: &mut Element, color: &str) {
    super::branding::BY_SQUARE_WORDMARK.insert_at(svg, 185.0, 539.0, 39.0, color);
}

fn insert_document_icon(svg: &mut Element, color: &str) {
    super::branding::INVOICE_ICON.insert_at(svg, 414.0, 502.0, 94.0, color);
}

#[cfg(test)]
mod tests {
    use super::super::{LogoColor, LogoLayout, LogoPosition};
    use super::*;

    #[test]
    fn default_decoration_contains_frame_and_branding_groups() {
        let mut svg = Element::new("svg");

        decorate(&mut svg, LogoTheme::default());

        assert_eq!(svg.children.len(), 2);
    }

    #[test]
    fn qr_dimension_preserves_frame_clearance_around_the_quiet_zone() {
        let clearance = (super::super::CONTAINER_WIDTH as u32 - QR_MAX_DIMENSION) / 2;
        assert_eq!(clearance, 4);
    }

    #[test]
    fn complete_invoice_svg_contains_vector_branding() {
        let svg = String::from_utf8(super::super::create_invoice_svg("INVOICE-FIXTURE").unwrap())
            .unwrap();

        assert!(svg.contains(LogoColor::Dark.invoice_hex()));
        assert!(svg.contains("M1386.1976,5.6554"));
        assert!(svg.contains("M174.6769.2864"));
        assert!(!svg.contains("<text"));
        assert!(!svg.contains("<image"));
    }

    #[test]
    fn legacy_entry_point_uses_the_default_invoice_theme() {
        let legacy = super::super::create_invoice_svg("INVOICE-FIXTURE").unwrap();
        let explicit =
            super::super::create_invoice_svg_with_theme("INVOICE-FIXTURE", LogoTheme::default())
                .unwrap();
        assert_eq!(
            Element::parse(legacy.as_slice()).unwrap(),
            Element::parse(explicit.as_slice()).unwrap()
        );
    }

    #[test]
    fn every_manual_theme_has_expected_dimensions_color_and_frame() {
        for layout in LogoLayout::ALL {
            for position in LogoPosition::ALL {
                for color in LogoColor::ALL {
                    let theme = LogoTheme::new(layout, position, color);
                    let svg = String::from_utf8(
                        super::super::create_invoice_svg_with_theme("INVOICE-THEME", theme)
                            .unwrap(),
                    )
                    .unwrap();
                    let (width, height) = theme.dimensions();

                    assert!(svg.contains(&format!("viewBox=\"0 0 {width} {height}\"")));
                    assert!(svg.contains(color.invoice_hex()));
                    assert_eq!(
                        svg.contains(super::super::logo::BY_SQUARE_COLOR),
                        color != LogoColor::Black
                    );
                    assert_eq!(
                        svg.contains("stroke-width=\"8\""),
                        layout == LogoLayout::Print
                    );
                }
            }
        }
    }
}
