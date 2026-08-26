//! Code-native decoration for a PAY by square QR image.

use xmltree::Element;

use super::LogoTheme;

pub(crate) const QR_MAX_DIMENSION: u32 = 500;

pub(crate) fn decorate(svg: &mut Element, theme: LogoTheme) {
    super::logo::decorate(
        svg,
        theme,
        theme.color.pay_hex(),
        bottom_wordmarks,
        insert_pay_icon,
    );
}

fn bottom_wordmarks(color: &str, by_square_color: &str) -> Element {
    let mut group = Element::new("g");
    super::branding::PAY_WORDMARK.insert_at(&mut group, 113.0, 541.0, 29.0, color);
    super::branding::BY_SQUARE_WORDMARK.insert_at(&mut group, 193.0, 539.0, 39.0, by_square_color);
    group
}

fn insert_pay_icon(svg: &mut Element, color: &str) {
    super::branding::PAY_ICON.insert_right_aligned(
        svg,
        super::CONTAINER_WIDTH,
        super::CONTAINER_HEIGHT - 98.0,
        98.0,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::super::{LogoColor, LogoLayout, LogoPosition};
    use super::*;

    #[test]
    fn complete_pay_svg_contains_embedded_vector_branding() {
        let svg = String::from_utf8(super::super::create_pay_svg(
            "PAY-FIXTURE",
            LogoTheme::default(),
        ))
        .unwrap();

        assert!(svg.contains(LogoColor::Dark.pay_hex()));
        assert!(svg.contains("M171.0449,558.182"));
        assert!(svg.contains("M65.0527,138.3133"));
        assert!(svg.contains("M206.6639.2853"));
        assert!(!svg.contains("<text"));
        assert!(!svg.contains("<image"));
    }

    #[test]
    fn every_manual_theme_has_expected_dimensions_color_and_frame() {
        for layout in LogoLayout::ALL {
            for position in LogoPosition::ALL {
                for color in LogoColor::ALL {
                    let theme = LogoTheme::new(layout, position, color);
                    let svg = String::from_utf8(super::super::create_pay_svg("PAY-THEME", theme))
                        .unwrap();
                    let (width, height) = theme.dimensions();

                    assert!(svg.contains(&format!("viewBox=\"0 0 {width} {height}\"")));
                    assert!(svg.contains(color.pay_hex()));
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
