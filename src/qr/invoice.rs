//! Code-native decoration for an INVOICE by square QR image.
//!
//! This module intentionally owns only the branding layer. QR generation and
//! placement stay in the parent module so the matrix can remain square and
//! unobstructed.

use std::collections::HashMap;

use xmltree::{Element, XMLNode};

use super::{InvoiceColor, InvoiceTheme, LogoLayout, LogoPosition};

/// Maximum size of the QR matrix including its renderer-provided four-module
/// quiet zone. The remaining four canvas units separate it from the frame.
pub(crate) const QR_MAX_DIMENSION: u32 = 504;

const BY_SQUARE_COLOR: &str = "#b2b4b9";

/// Add the official-color Invoice frame, wordmark, and document icon.
///
/// The caller must insert the white background and centered QR matrix first.
/// Keeping this as a decoration-only hook makes it impossible for the branding
/// to resize, translate, or cover the QR matrix.
pub(crate) fn decorate(svg: &mut Element, theme: InvoiceTheme) {
    let color = theme.color.hex();
    let by_square_color = if theme.color == InvoiceColor::Black {
        color
    } else {
        BY_SQUARE_COLOR
    };

    match theme.position {
        LogoPosition::Bottom => {
            insert_bottom_frame(svg, theme.layout, color);
            svg.children
                .push(XMLNode::Element(bottom_branding(color, by_square_color)));
        }
        LogoPosition::Top => {
            if theme.layout == LogoLayout::Print {
                let mut frame = Element::new("g");
                frame.attributes =
                    HashMap::from([("transform".to_owned(), "matrix(1 0 0 -1 0 600)".to_owned())]);
                super::insert_outline(&mut frame, color);
                svg.children.push(XMLNode::Element(frame));
            }
            let mut branding = bottom_branding(color, by_square_color);
            branding.attributes =
                HashMap::from([("transform".to_owned(), "translate(0 -498)".to_owned())]);
            svg.children.push(XMLNode::Element(branding));
        }
        LogoPosition::Right => {
            insert_right_frame(svg, theme.layout, color);

            let mut words = bottom_wordmarks(color, by_square_color);
            words.attributes =
                HashMap::from([("transform".to_owned(), "matrix(0 -1 1 0 0 512)".to_owned())]);
            svg.children.push(XMLNode::Element(words));

            let mut icon = Element::new("g");
            icon.attributes =
                HashMap::from([("transform".to_owned(), "translate(88 -498)".to_owned())]);
            insert_document_icon(&mut icon, color);
            svg.children.push(XMLNode::Element(icon));
        }
        LogoPosition::Left => {
            insert_left_frame(svg, theme.layout, color);

            let mut words = bottom_wordmarks(color, by_square_color);
            words.attributes = HashMap::from([(
                "transform".to_owned(),
                "matrix(0 1 -1 0 600 110)".to_owned(),
            )]);
            svg.children.push(XMLNode::Element(words));

            let mut icon = Element::new("g");
            icon.attributes =
                HashMap::from([("transform".to_owned(), "translate(-410 -498)".to_owned())]);
            insert_document_icon(&mut icon, color);
            svg.children.push(XMLNode::Element(icon));
        }
    }
}

fn insert_bottom_frame(svg: &mut Element, layout: LogoLayout, color: &str) {
    if layout == LogoLayout::Print {
        super::insert_outline(svg, color);
    }
}

fn insert_left_frame(svg: &mut Element, layout: LogoLayout, color: &str) {
    if layout != LogoLayout::Print {
        return;
    }

    let mut mirror = Element::new("g");
    mirror.attributes =
        HashMap::from([("transform".to_owned(), "matrix(-1 0 0 1 600 0)".to_owned())]);
    let mut right = Element::new("g");
    right.attributes =
        HashMap::from([("transform".to_owned(), "matrix(0 -1 1 0 0 512)".to_owned())]);
    super::insert_outline(&mut right, color);
    mirror.children.push(XMLNode::Element(right));
    svg.children.push(XMLNode::Element(mirror));
}

fn insert_right_frame(svg: &mut Element, layout: LogoLayout, color: &str) {
    if layout != LogoLayout::Print {
        return;
    }

    let mut frame = Element::new("g");
    frame.attributes =
        HashMap::from([("transform".to_owned(), "matrix(0 -1 1 0 0 512)".to_owned())]);
    super::insert_outline(&mut frame, color);
    svg.children.push(XMLNode::Element(frame));
}

fn bottom_branding(color: &str, by_square_color: &str) -> Element {
    let mut group = bottom_wordmarks(color, by_square_color);
    insert_document_icon(&mut group, color);
    group
}

fn bottom_wordmarks(color: &str, by_square_color: &str) -> Element {
    let mut group = Element::new("g");
    insert_invoice_wordmark(&mut group, color);
    insert_by_square_wordmark(&mut group, by_square_color);
    group
}

fn insert_invoice_wordmark(svg: &mut Element, color: &str) {
    // Hand-built, font-independent uppercase glyphs. Bounds: 155 x 29 at
    // (16, 540), aligned with the baseline of the existing by-square path.
    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("fill".to_string(), color.to_string()),
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

fn insert_by_square_wordmark(svg: &mut Element, color: &str) {
    // Reuse the parent's font-independent by-square outline. The parent path
    // starts at x=193 for the shorter PAY label; shift it left to x=185 to
    // preserve the spacing seen in the Invoice reference.
    let mut group = Element::new("g");
    group.attributes = HashMap::from([("transform".to_string(), "translate(-8,0)".to_string())]);
    super::insert_by_square_text(&mut group, color);
    svg.children.push(XMLNode::Element(group));
}

fn insert_document_icon(svg: &mut Element, color: &str) {
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
        ("fill".to_string(), color.to_string()),
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
    fn default_decoration_contains_frame_and_branding_groups() {
        let mut svg = Element::new("svg");

        decorate(&mut svg, InvoiceTheme::default());

        assert_eq!(svg.children.len(), 2);
    }

    #[test]
    fn qr_dimension_preserves_frame_clearance_around_the_quiet_zone() {
        let clearance = (super::super::CONTAINER_WIDTH as u32 - QR_MAX_DIMENSION) / 2;
        assert_eq!(clearance, 4);
    }

    #[test]
    fn complete_invoice_svg_contains_vector_branding() {
        let svg = String::from_utf8(super::super::create_invoice_svg("INVOICE-FIXTURE")).unwrap();

        assert!(svg.contains(InvoiceColor::Dark.hex()));
        assert!(svg.contains("M440 516h25l14 14v50h-39z"));
        assert!(!svg.contains("<text"));
    }

    #[test]
    fn legacy_entry_point_uses_the_default_invoice_theme() {
        let legacy = super::super::create_invoice_svg("INVOICE-FIXTURE");
        let explicit =
            super::super::create_invoice_svg_with_theme("INVOICE-FIXTURE", InvoiceTheme::default());
        assert_eq!(
            Element::parse(legacy.as_slice()).unwrap(),
            Element::parse(explicit.as_slice()).unwrap()
        );
    }

    #[test]
    fn every_manual_theme_has_expected_dimensions_color_and_frame() {
        for layout in LogoLayout::ALL {
            for position in LogoPosition::ALL {
                for color in InvoiceColor::ALL {
                    let theme = InvoiceTheme::new(layout, position, color);
                    let svg = String::from_utf8(super::super::create_invoice_svg_with_theme(
                        "INVOICE-THEME",
                        theme,
                    ))
                    .unwrap();
                    let (width, height) = theme.dimensions();

                    assert!(svg.contains(&format!("viewBox=\"0 0 {width} {height}\"")));
                    assert!(svg.contains(color.hex()));
                    assert_eq!(svg.contains(BY_SQUARE_COLOR), color != InvoiceColor::Black);
                    assert_eq!(
                        svg.contains("stroke-width=\"8\""),
                        layout == LogoLayout::Print
                    );
                }
            }
        }
    }
}
