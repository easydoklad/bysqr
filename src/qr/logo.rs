//! Shared placement rules for logo-manual-compliant PAY and INVOICE artwork.

use std::collections::HashMap;

use xmltree::{Element, XMLNode};

use super::{LogoColor, LogoLayout, LogoPosition, LogoTheme};

pub(crate) const BY_SQUARE_COLOR: &str = "#b2b4b9";

pub(crate) fn decorate(
    svg: &mut Element,
    theme: LogoTheme,
    accent: &str,
    bottom_wordmarks: fn(&str, &str) -> Element,
    insert_icon: fn(&mut Element, &str),
) {
    let by_square_color = if theme.color == LogoColor::Black {
        accent
    } else {
        BY_SQUARE_COLOR
    };

    match theme.position {
        LogoPosition::Bottom => {
            insert_bottom_frame(svg, theme.layout, accent);
            svg.children.push(XMLNode::Element(bottom_branding(
                accent,
                by_square_color,
                bottom_wordmarks,
                insert_icon,
            )));
        }
        LogoPosition::Top => {
            if theme.layout == LogoLayout::Print {
                let mut frame = Element::new("g");
                frame.attributes =
                    HashMap::from([("transform".to_owned(), "matrix(1 0 0 -1 0 600)".to_owned())]);
                super::insert_outline(&mut frame, accent);
                svg.children.push(XMLNode::Element(frame));
            }
            let mut branding =
                bottom_branding(accent, by_square_color, bottom_wordmarks, insert_icon);
            branding.attributes =
                HashMap::from([("transform".to_owned(), "translate(0 -498)".to_owned())]);
            svg.children.push(XMLNode::Element(branding));
        }
        LogoPosition::Right => {
            insert_right_frame(svg, theme.layout, accent);

            let mut words = bottom_wordmarks(accent, by_square_color);
            words.attributes =
                HashMap::from([("transform".to_owned(), "matrix(0 -1 1 0 0 512)".to_owned())]);
            svg.children.push(XMLNode::Element(words));

            let mut icon = Element::new("g");
            icon.attributes =
                HashMap::from([("transform".to_owned(), "translate(88 -498)".to_owned())]);
            insert_icon(&mut icon, accent);
            svg.children.push(XMLNode::Element(icon));
        }
        LogoPosition::Left => {
            insert_left_frame(svg, theme.layout, accent);

            let mut words = bottom_wordmarks(accent, by_square_color);
            words.attributes = HashMap::from([(
                "transform".to_owned(),
                "matrix(0 1 -1 0 600 110)".to_owned(),
            )]);
            svg.children.push(XMLNode::Element(words));

            let mut icon = Element::new("g");
            icon.attributes =
                HashMap::from([("transform".to_owned(), "translate(-410 -498)".to_owned())]);
            insert_icon(&mut icon, accent);
            svg.children.push(XMLNode::Element(icon));
        }
    }
}

fn bottom_branding(
    accent: &str,
    by_square_color: &str,
    bottom_wordmarks: fn(&str, &str) -> Element,
    insert_icon: fn(&mut Element, &str),
) -> Element {
    let mut group = bottom_wordmarks(accent, by_square_color);
    insert_icon(&mut group, accent);
    group
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
