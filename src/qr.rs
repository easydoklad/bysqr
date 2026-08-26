use base64::Engine;
use jpeg_encoder::{ColorType, Encoder};
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use resvg::tiny_skia::Pixmap;
use std::collections::HashMap;
use usvg::{Options, Transform, Tree};
use xmltree::{Element, EmitterConfig};

mod invoice;
mod items;

pub const CONTAINER_WIDTH: f32 = 512.0;
pub const CONTAINER_HEIGHT: f32 = 600.0;

/// Approved INVOICE by square logo composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogoLayout {
    /// Framed composition intended for print and general-purpose output.
    Print,
    /// Compact composition without the surrounding frame, intended for screens.
    Electronic,
}

/// Approved placement of the by-square branding around the QR matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogoPosition {
    Bottom,
    Top,
    Left,
    Right,
}

/// Approved INVOICE by square color variation from the logo manual.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvoiceColor {
    Light,
    Dark,
    Gray,
    Black,
}

impl InvoiceColor {
    pub const ALL: [Self; 4] = [Self::Light, Self::Dark, Self::Gray, Self::Black];

    pub const fn hex(self) -> &'static str {
        match self {
            Self::Light => "#FAB65B",
            Self::Dark => "#F5871F",
            Self::Gray => "#5F6062",
            Self::Black => "#000000",
        }
    }
}

impl LogoLayout {
    pub const ALL: [Self; 2] = [Self::Print, Self::Electronic];
}

impl LogoPosition {
    pub const ALL: [Self; 4] = [Self::Bottom, Self::Top, Self::Left, Self::Right];
}

/// One logo-manual-compliant INVOICE by square visual theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvoiceTheme {
    pub layout: LogoLayout,
    pub position: LogoPosition,
    pub color: InvoiceColor,
}

impl InvoiceTheme {
    pub const fn new(layout: LogoLayout, position: LogoPosition, color: InvoiceColor) -> Self {
        Self {
            layout,
            position,
            color,
        }
    }

    /// SVG canvas dimensions. Side branding swaps the portrait dimensions.
    pub const fn dimensions(self) -> (u32, u32) {
        match self.position {
            LogoPosition::Bottom | LogoPosition::Top => {
                (CONTAINER_WIDTH as u32, CONTAINER_HEIGHT as u32)
            }
            LogoPosition::Left | LogoPosition::Right => {
                (CONTAINER_HEIGHT as u32, CONTAINER_WIDTH as u32)
            }
        }
    }

    const fn qr_center(self) -> (f32, f32) {
        match self.position {
            LogoPosition::Bottom | LogoPosition::Right => {
                (CONTAINER_WIDTH / 2.0, CONTAINER_WIDTH / 2.0)
            }
            LogoPosition::Top => (
                CONTAINER_WIDTH / 2.0,
                CONTAINER_HEIGHT - CONTAINER_WIDTH / 2.0,
            ),
            LogoPosition::Left => (
                CONTAINER_HEIGHT - CONTAINER_WIDTH / 2.0,
                CONTAINER_WIDTH / 2.0,
            ),
        }
    }
}

impl Default for InvoiceTheme {
    fn default() -> Self {
        Self::new(LogoLayout::Print, LogoPosition::Bottom, InvoiceColor::Dark)
    }
}

pub struct Theme {
    background_color: String,
    outline_color: String,
    icon_color: String,
    by_square_text_color: String,
    pay_text_color: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background_color: String::from("#ffffff"),
            outline_color: String::from("#6fa4d7"),
            icon_color: String::from("#6fa4d7"),
            by_square_text_color: String::from("#b2b4b9"),
            pay_text_color: String::from("#6fa4d7"),
        }
    }
}

fn insert_background(svg: &mut Element, color: &str, width: u32, height: u32) {
    let mut path = Element::new("rect");
    path.attributes = HashMap::from([
        ("fill".to_string(), color.to_string()),
        ("width".to_string(), width.to_string()),
        ("height".to_string(), height.to_string()),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));
}

fn insert_pay_text(svg: &mut Element, color: &str) {
    // Text dimension: 69x29
    let translate_x = (CONTAINER_WIDTH - 69.0 - 330.0) as u32;
    let translate_y = (CONTAINER_HEIGHT - 29.0 - 30.0) as u32;

    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("d".to_string(), "M9.43 14.43c1.8 0 3.13-.45 3.96-1.36a5.23 5.23 0 0 0 1.27-3.72c0-.72-.1-1.37-.32-1.96a3.7 3.7 0 0 0-.96-1.51 4.15 4.15 0 0 0-1.62-.98 7.14 7.14 0 0 0-2.33-.34H5.77v9.87h3.66ZM9.43 0c1.91 0 3.56.23 4.95.7a9.24 9.24 0 0 1 3.44 1.96 7.5 7.5 0 0 1 1.98 2.96c.44 1.14.66 2.39.66 3.73 0 1.43-.23 2.74-.68 3.92a8.02 8.02 0 0 1-2.04 3.06 9.51 9.51 0 0 1-3.44 2c-1.39.46-3.01.7-4.87.7H5.77V29H0V0h9.43Zm27.49 17.99L33.9 9.1a24.37 24.37 0 0 1-1.14-3.67 29.09 29.09 0 0 1-1.13 3.71l-3 8.84h8.29ZM46.62 29h-4.49a2 2 0 0 1-1.23-.36c-.3-.25-.54-.57-.7-.96l-1.88-5.54H27.2l-1.89 5.54c-.12.33-.35.64-.67.92-.32.27-.72.4-1.21.4H18.9L29.8 0h5.92l10.89 29Zm12.24-11.15V29h-5.78V17.85L42.94 0h5.08c.5 0 .9.13 1.2.38.3.24.55.55.73.94l4.58 9.13.83 1.72a17 17 0 0 1 .67 1.6c.18-.53.38-1.07.61-1.6l.81-1.72L62 1.32c.16-.32.39-.62.7-.9.3-.28.7-.42 1.2-.42H69L58.86 17.85Z".to_string()),
        ("fill".to_string(), color.to_string()),
        ("transform".to_string(), format!("translate({},{})", translate_x, translate_y)),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));
}

fn insert_by_square_text(svg: &mut Element, color: &str) {
    // Text dimension: 189x39
    let translate_x = (CONTAINER_WIDTH - 189.0 - 130.0) as u32;
    let translate_y = (CONTAINER_HEIGHT - 39.0 - 22.0) as u32;

    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("d".to_string(), "M14.75 20.4c0-.82-.07-1.64-.2-2.45a6.8 6.8 0 0 0-.68-2.08 4.1 4.1 0 0 0-1.26-1.46 3.33 3.33 0 0 0-1.96-.55 4.18 4.18 0 0 0-2.28.7c-.44.28-.84.6-1.22.97-.47.47-.91.96-1.31 1.49v6.58a12.2 12.2 0 0 0 2.35 2.3c.67.5 1.5.77 2.34.79.69.02 1.36-.18 1.92-.56.54-.39.99-.88 1.31-1.44.36-.63.62-1.32.76-2.03.15-.74.23-1.5.23-2.26Zm6.04-.4c.02 1.6-.18 3.2-.6 4.75-.35 1.3-.93 2.51-1.74 3.6a7.65 7.65 0 0 1-2.79 2.29 9.49 9.49 0 0 1-5.7.61 6.8 6.8 0 0 1-1.73-.61 9.48 9.48 0 0 1-1.62-1.07c-.58-.48-1.13-1-1.63-1.55v2.15c0 .15-.04.3-.13.41a.93.93 0 0 1-.42.28c-.25.09-.5.14-.77.16a13.4 13.4 0 0 1-2.38 0 3 3 0 0 1-.76-.16.84.84 0 0 1-.4-.28.7.7 0 0 1-.12-.4V.94C0 .81.05.67.14.55.26.41.43.31.6.25c.3-.09.6-.15.9-.18a15 15 0 0 1 2.82 0c.3.03.6.09.9.19.18.05.34.15.47.3.09.1.14.25.13.4V11.9c.5-.47 1.02-.9 1.58-1.3.5-.33 1.02-.62 1.57-.87a7.4 7.4 0 0 1 1.62-.5 9.6 9.6 0 0 1 1.77-.15 8 8 0 0 1 3.88.87 7.4 7.4 0 0 1 2.6 2.37 10.2 10.2 0 0 1 1.48 3.48c.31 1.37.47 2.78.46 4.18m16.38 10.95-2.5 7.02c-.14.37-.52.64-1.13.8-.9.2-1.8.27-2.71.24-.5.01-1-.01-1.48-.07-.3-.03-.58-.12-.84-.26a.6.6 0 0 1-.2-.19.57.57 0 0 1-.1-.25c0-.23.04-.45.14-.65l2.76-6.64a1.87 1.87 0 0 1-.9-1L23.08 11.7a3.62 3.62 0 0 1-.3-1.19.78.78 0 0 1 .27-.64c.28-.2.6-.3.94-.33.58-.06 1.16-.1 1.75-.09.66 0 1.19.02 1.57.04.3 0 .61.06.9.16.21.09.38.23.5.42.13.25.23.51.32.78l4.86 13.27h.07L38.4 10.6c.04-.29.17-.55.36-.77.22-.15.47-.25.73-.28a12 12 0 0 1 1.78-.09c.56 0 1.11.03 1.66.1.35.02.68.14.97.33a.8.8 0 0 1 .31.65c0 .33-.07.66-.18.97l-6.88 19.42Zm35.75-6.36a6 6 0 0 1-2.52 5.13 8.57 8.57 0 0 1-2.85 1.3 14.82 14.82 0 0 1-7.67-.15 9.54 9.54 0 0 1-1.38-.52c-.3-.13-.57-.3-.81-.5a1.5 1.5 0 0 1-.4-.73 9.1 9.1 0 0 1-.1-2.51c.03-.22.08-.43.15-.63a.59.59 0 0 1 .23-.3.7.7 0 0 1 .35-.09c.27.04.53.13.75.28a15.95 15.95 0 0 0 2.96 1.24 7.94 7.94 0 0 0 3.64.13c.38-.1.74-.24 1.06-.45.3-.18.53-.43.69-.73.16-.32.23-.67.23-1.02 0-.42-.15-.83-.43-1.15a3.8 3.8 0 0 0-1.14-.83 13.4 13.4 0 0 0-1.6-.66 35.3 35.3 0 0 1-1.82-.7c-.63-.25-1.24-.53-1.83-.86a6.96 6.96 0 0 1-1.6-1.2 5.42 5.42 0 0 1-1.56-4.11 5.73 5.73 0 0 1 2.25-4.64 8.03 8.03 0 0 1 2.64-1.31 13.04 13.04 0 0 1 8.3.43c.26.1.5.24.74.4.12.09.23.2.3.33.07.13.12.26.15.4a6.26 6.26 0 0 1 .11 1.44c0 .4-.01.72-.03.97-.02.2-.06.4-.12.6a.53.53 0 0 1-.22.3.65.65 0 0 1-.32.08 1.61 1.61 0 0 1-.65-.23 12.02 12.02 0 0 0-2.63-1 7.64 7.64 0 0 0-3.34-.08 2.5 2.5 0 0 0-.95.44 1.9 1.9 0 0 0-.74 1.51c-.02.43.14.84.44 1.16.33.34.72.62 1.15.82.53.26 1.08.48 1.64.67a46 46 0 0 1 1.85.68c.63.24 1.25.53 1.85.85a7 7 0 0 1 1.63 1.2 5.55 5.55 0 0 1 1.6 4.04m18.18-7.64c-.7-.87-1.5-1.64-2.4-2.31a4.13 4.13 0 0 0-2.4-.8c-.7-.02-1.38.17-1.95.55-.56.37-1.01.87-1.32 1.45-.35.64-.6 1.33-.74 2.03a10.8 10.8 0 0 0-.25 2.3c0 .82.07 1.63.21 2.44.12.72.35 1.43.69 2.1.29.57.72 1.08 1.26 1.46a3.4 3.4 0 0 0 1.99.56A4.1 4.1 0 0 0 88.5 26c.43-.3.84-.62 1.21-1 .42-.4.87-.9 1.37-1.49v-6.58Zm5.79 21.1a.6.6 0 0 1-.14.4.95.95 0 0 1-.47.3c-.28.1-.58.17-.88.2-.94.09-1.88.09-2.82 0-.3-.03-.6-.1-.9-.2a.96.96 0 0 1-.45-.3.67.67 0 0 1-.13-.4v-9.41c-.52.45-1.06.88-1.62 1.28-.5.34-1.02.64-1.58.88a7.4 7.4 0 0 1-1.62.5 9.6 9.6 0 0 1-1.76.16 7.4 7.4 0 0 1-6.47-3.24c-.7-1.07-1.2-2.25-1.47-3.48a18.39 18.39 0 0 1-.46-4.19c-.02-1.6.18-3.2.6-4.76.35-1.3.94-2.51 1.76-3.6a7.79 7.79 0 0 1 2.85-2.28 9.06 9.06 0 0 1 5.56-.64c.55.12 1.08.32 1.57.58a11 11 0 0 1 1.63 1.07 25 25 0 0 1 1.87 1.6v-2.15c0-.14.04-.27.12-.38a.92.92 0 0 1 .42-.28c.24-.09.5-.14.75-.17a10.56 10.56 0 0 1 3.13.17c.16.05.3.15.4.28.07.11.11.25.1.38v27.69Zm25.44-7.86c0 .14-.03.28-.11.4a.85.85 0 0 1-.4.27c-.25.09-.51.14-.77.17a12.78 12.78 0 0 1-2.43 0 3.05 3.05 0 0 1-.76-.17.85.85 0 0 1-.39-.28.66.66 0 0 1-.11-.39v-2.29c-1 1.08-2.18 1.98-3.49 2.67a8.09 8.09 0 0 1-3.67.89 8.55 8.55 0 0 1-3.58-.67 6.13 6.13 0 0 1-2.33-1.82 7.2 7.2 0 0 1-1.28-2.68c-.28-1.24-.4-2.51-.38-3.78V10.35c0-.14.04-.27.13-.38a.94.94 0 0 1 .46-.28c.3-.09.6-.15.9-.17a16.7 16.7 0 0 1 2.82 0c.3.02.6.08.89.17.18.05.35.14.47.28.1.1.14.24.14.38v11.2c-.02.8.05 1.6.21 2.38a4 4 0 0 0 .64 1.41c.28.39.65.7 1.09.91.47.23 1 .34 1.53.32a3.9 3.9 0 0 0 2.24-.77c.9-.66 1.7-1.42 2.4-2.27V10.35c0-.14.04-.27.12-.38a.94.94 0 0 1 .47-.28c.29-.09.59-.15.89-.17.94-.07 1.88-.07 2.82 0 .3.02.6.08.88.17.18.05.34.14.47.28.09.1.14.24.14.38v19.82Zm17.74-8.42h-2.22c-.81-.01-1.62.06-2.41.2-.58.1-1.13.3-1.62.6a2.52 2.52 0 0 0-1.2 2.27 2.4 2.4 0 0 0 .81 1.95c.64.5 1.46.76 2.29.71.8.01 1.57-.2 2.25-.6a9.28 9.28 0 0 0 2.1-1.75v-3.38Zm5.72 8.49a.58.58 0 0 1-.24.49c-.21.14-.46.23-.72.25-.5.06-.98.09-1.47.08a9.62 9.62 0 0 1-1.52-.08 1.34 1.34 0 0 1-.67-.25.64.64 0 0 1-.2-.5v-1.57a8.75 8.75 0 0 1-6.53 2.78c-1.02.01-2.04-.13-3.02-.41a6.88 6.88 0 0 1-2.38-1.22 5.5 5.5 0 0 1-1.57-2.01 6.6 6.6 0 0 1-.55-2.8 6.13 6.13 0 0 1 .7-3.01 5.6 5.6 0 0 1 2.1-2.11 10.76 10.76 0 0 1 3.5-1.24c1.59-.28 3.21-.42 4.83-.4h2.02v-1.2c0-.55-.06-1.1-.2-1.63a2.7 2.7 0 0 0-.65-1.17 2.7 2.7 0 0 0-1.2-.68 9.99 9.99 0 0 0-4.47.09 15.44 15.44 0 0 0-3.5 1.37c-.29.18-.62.28-.96.31a.7.7 0 0 1-.45-.15 1.12 1.12 0 0 1-.32-.45c-.1-.23-.16-.47-.2-.7a5.7 5.7 0 0 1 .05-2.03c.08-.27.23-.52.44-.72.33-.3.72-.54 1.13-.71.61-.3 1.25-.53 1.9-.73a18.15 18.15 0 0 1 9.18-.3c1.04.26 2.01.73 2.84 1.4.75.66 1.31 1.5 1.62 2.44.36 1.14.53 2.33.5 3.53v13.33Zm19.08-18.04c0 .53-.02.97-.05 1.31-.02.27-.06.54-.14.8a.78.78 0 0 1-.24.4.62.62 0 0 1-.38.11c-.15 0-.29-.03-.42-.08a20.76 20.76 0 0 0-1.16-.33c-.25-.05-.5-.08-.76-.08-.34 0-.67.07-.97.19-.37.15-.7.35-1.01.59-.4.32-.77.67-1.1 1.06-.43.53-.84 1.08-1.2 1.65v12.35a.6.6 0 0 1-.15.4 1 1 0 0 1-.47.27 4.3 4.3 0 0 1-.9.17c-.93.07-1.88.07-2.82 0-.3-.03-.6-.08-.89-.17a1.03 1.03 0 0 1-.47-.28.59.59 0 0 1-.14-.39V10.35c0-.13.04-.27.11-.38a.85.85 0 0 1 .42-.28c.25-.09.51-.14.78-.17a11.21 11.21 0 0 1 2.42 0c.25.02.5.08.75.17a.8.8 0 0 1 .38.28c.08.11.12.25.12.38v2.47c.46-.65.97-1.26 1.52-1.83a8.5 8.5 0 0 1 1.37-1.15c.4-.26.83-.46 1.3-.58a5.18 5.18 0 0 1 1.94-.14 6.61 6.61 0 0 1 1.37.3c.15.04.28.11.4.2.08.07.15.15.19.25a4.97 4.97 0 0 1 .17 1.07c.02.3.03.73.03 1.26m16.48 5.89a5.58 5.58 0 0 0-.98-3.7c-.7-.9-1.79-1.35-3.26-1.35a4.36 4.36 0 0 0-1.93.4c-.53.26-1 .62-1.36 1.08a4.87 4.87 0 0 0-.84 1.6 7.7 7.7 0 0 0-.33 1.97h8.7Zm5.65 1.6c.05.54-.12 1.08-.47 1.5a1.7 1.7 0 0 1-1.31.5h-12.57c-.01.77.1 1.55.3 2.3.2.65.54 1.25 1 1.75.49.5 1.1.88 1.77 1.1.83.27 1.7.4 2.58.38a15.05 15.05 0 0 0 4.69-.68c.56-.17 1.03-.32 1.4-.47.28-.12.59-.19.9-.2.12-.01.24.02.35.07.1.07.18.17.23.28.07.18.11.37.13.57a10.57 10.57 0 0 1-.06 2.34 1.36 1.36 0 0 1-.38.73 2.6 2.6 0 0 1-.83.42c-.55.22-1.13.4-1.7.53-.8.18-1.6.33-2.4.44-.94.13-1.9.2-2.87.19-1.63.03-3.26-.2-4.82-.67a8.62 8.62 0 0 1-3.46-2.02 8.33 8.33 0 0 1-2.08-3.42c-.48-1.57-.71-3.2-.68-4.84a14.8 14.8 0 0 1 .72-4.77 9.9 9.9 0 0 1 2.08-3.6 8.97 8.97 0 0 1 3.32-2.25c1.4-.54 2.9-.8 4.41-.78a12 12 0 0 1 4.44.73 8.04 8.04 0 0 1 3.02 2.04 8.1 8.1 0 0 1 1.74 3.07c.38 1.26.56 2.56.55 3.87v.89Z".to_string()),
        ("fill".to_string(), color.to_string()),
        ("transform".to_string(), format!("translate({},{})", translate_x, translate_y)),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));
}

fn insert_pay_icon(svg: &mut Element, color: &str) {
    let icon_size = 98.0;
    let translate_x = (CONTAINER_WIDTH - icon_size) as u32;
    let translate_y = (CONTAINER_HEIGHT - icon_size) as u32;

    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("d".to_string(), "m80.8 45.55-63.24 9.23 2.7 18.55 63.25-9.22-2.71-18.56ZM22.39 58.38l30.35-4.43.5 3.38-30.36 4.42-.5-3.37Zm31.34 2.32-30.36 4.43.5 3.37 30.35-4.43-.5-3.37Z".to_string()),
        ("clip-rule".to_string(), "evenodd".to_string()),
        ("fill-rule".to_string(), "evenodd".to_string()),
        ("fill".to_string(), color.to_string()),
        ("transform".to_string(), format!("translate({},{})", translate_x, translate_y)),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));

    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        (
            "d".to_string(),
            "m15.46 40.44 63.25-9.22.74 5.06L16.2 45.5l-.73-5.06Z".to_string(),
        ),
        ("fill".to_string(), color.to_string()),
        (
            "transform".to_string(),
            format!("translate({},{})", translate_x, translate_y),
        ),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));

    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("d".to_string(), "M18.75 0A18.75 18.75 0 0 0 0 18.75v60.5C0 89.61 8.4 98 18.75 98h60.5C89.61 98 98 89.6 98 79.25v-60.5C98 8.39 89.6 0 79.25 0h-60.5Zm-4.99 34.66a4.26 4.26 0 0 0-3.6 4.84l5.17 35.41a4.26 4.26 0 0 0 4.83 3.6l64.93-9.47a4.26 4.26 0 0 0 3.6-4.83l-5.17-35.42a4.26 4.26 0 0 0-4.83-3.6l-64.93 9.47Z".to_string()),
        ("clip-rule".to_string(), "evenodd".to_string()),
        ("fill-rule".to_string(), "evenodd".to_string()),
        ("fill".to_string(), color.to_string()),
        ("transform".to_string(), format!("translate({},{})", translate_x, translate_y)),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));
}

fn insert_outline(svg: &mut Element, color: &str) {
    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("d".to_string(), "M508 481V7.99999C508 5.79085 506.209 4 504 4H8C5.79086 4 4 5.79085 4 7.99999V503C4 505.209 5.79086 507 8 507H377".to_string()),
        ("stroke".to_string(), color.to_string()),
        ("fill".to_string(), "none".to_string()),
        ("stroke-width".to_string(), "8".to_string()),
        ("stroke-linecap".to_string(), "round".to_string()),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));
}

fn insert_qr_content(svg: &mut Element, qr: &str, center_x: f32, center_y: f32) {
    let qr_svg =
        Element::parse(qr.as_bytes()).expect("unable to parse SVG content from QR encoder");

    let qr_width: f32 = qr_svg
        .attributes
        .get("width")
        .expect("unable to determine SVG content width")
        .parse()
        .expect("unable to parse SVG content width as number");

    let qr_height: f32 = qr_svg
        .attributes
        .get("height")
        .expect("unable to determine SVG content height")
        .parse()
        .expect("unable to parse SVG content height as number");

    let qr_path = qr_svg
        .get_child("path")
        .expect("QR code does not have path element")
        .attributes
        .get("d")
        .expect("unable to find d attribute within QR code");

    let translate_x = center_x - (qr_width / 2.0);
    let translate_y = center_y - (qr_height / 2.0);

    let mut path = Element::new("path");
    path.attributes = HashMap::from([
        ("d".to_string(), qr_path.clone()),
        (
            "transform".to_string(),
            format!("translate({},{})", translate_x, translate_y),
        ),
    ]);
    svg.children.push(xmltree::XMLNode::Element(path));
}

fn create_empty_svg(width: u32, height: u32) -> Element {
    let mut final_svg = Element::new("svg");
    final_svg.attributes = HashMap::from([
        (
            "xmlns".to_string(),
            "http://www.w3.org/2000/svg".to_string(),
        ),
        ("width".to_string(), width.to_string()),
        ("height".to_string(), height.to_string()),
        ("viewBox".to_string(), format!("0 0 {width} {height}")),
    ]);
    final_svg
}

pub fn create_pay_svg(content: &str, theme: Theme) -> Vec<u8> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .expect("unable to create QR code");

    let qr_size = (CONTAINER_WIDTH - 12.0) as u32;

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(qr_size, qr_size)
        .quiet_zone(false)
        .build();

    let mut svg = create_empty_svg(CONTAINER_WIDTH as u32, CONTAINER_HEIGHT as u32);
    insert_background(
        &mut svg,
        &theme.background_color,
        CONTAINER_WIDTH as u32,
        CONTAINER_HEIGHT as u32,
    );
    insert_qr_content(
        &mut svg,
        &svg_image,
        CONTAINER_WIDTH / 2.0,
        CONTAINER_WIDTH / 2.0,
    );
    insert_outline(&mut svg, &theme.outline_color);
    insert_pay_icon(&mut svg, &theme.icon_color);
    insert_by_square_text(&mut svg, &theme.by_square_text_color);
    insert_pay_text(&mut svg, &theme.pay_text_color);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .expect("unable to write generated SVG. possible XML corruption");
    qr
}

/// Render an INVOICE by square payload with the standard orange branding.
pub fn create_invoice_svg(content: &str) -> Vec<u8> {
    create_invoice_svg_with_theme(content, InvoiceTheme::default())
}

/// Render an INVOICE by square payload using one approved logo-manual theme.
pub fn create_invoice_svg_with_theme(content: &str, theme: InvoiceTheme) -> Vec<u8> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .expect("unable to create QR code");

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(invoice::QR_MAX_DIMENSION, invoice::QR_MAX_DIMENSION)
        .quiet_zone(true)
        .build();

    let (width, height) = theme.dimensions();
    let (center_x, center_y) = theme.qr_center();
    let mut svg = create_empty_svg(width, height);
    insert_background(&mut svg, "#ffffff", width, height);
    insert_qr_content(&mut svg, &svg_image, center_x, center_y);
    invoice::decorate(&mut svg, theme);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .expect("unable to write generated SVG. possible XML corruption");
    qr
}

/// Render an INVOICE ITEMS by square payload with the standard black branding.
pub fn create_invoice_items_svg(content: &str) -> Vec<u8> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .expect("unable to create QR code");

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(items::QR_MAX_DIMENSION, items::QR_MAX_DIMENSION)
        .quiet_zone(false)
        .build();

    let mut svg = create_empty_svg(CONTAINER_WIDTH as u32, CONTAINER_HEIGHT as u32);
    insert_background(
        &mut svg,
        "#ffffff",
        CONTAINER_WIDTH as u32,
        CONTAINER_HEIGHT as u32,
    );
    insert_qr_content(
        &mut svg,
        &svg_image,
        CONTAINER_WIDTH / 2.0,
        CONTAINER_WIDTH / 2.0,
    );
    items::decorate(&mut svg);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .expect("unable to write generated SVG. possible XML corruption");
    qr
}

pub fn map_svg(svg: &[u8], size: u32) -> Pixmap {
    let svg_tree = Tree::from_data(svg, &Options::default()).unwrap();
    let source_size = svg_tree.size();
    let scale = size as f32 / source_size.width();
    let width: u32 = size;
    let height = (source_size.height() * scale).round() as u32;

    let mut pixmap = Pixmap::new(width, height).expect("unable to create pixmap");
    resvg::render(
        &svg_tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
}

pub fn render_png(svg: &[u8], size: u32) -> Vec<u8> {
    let pixmap = map_svg(svg, size);

    pixmap.encode_png().expect("unable to save image")
}

pub fn to_base64_png(svg: &[u8], size: u32) -> String {
    let buf = render_png(svg, size);
    let base64_content = base64::engine::general_purpose::STANDARD.encode(&buf);
    format!("data:image/png;base64,{}", base64_content)
}

pub fn render_jpeg(svg: &[u8], size: u32, quality: u8) -> Vec<u8> {
    let pixmap = map_svg(svg, size);
    let (width, height) = (pixmap.width(), pixmap.height());
    let mut buf = Vec::with_capacity((width * height * 3) as usize);

    for pixel in pixmap.pixels() {
        buf.push(pixel.red());
        buf.push(pixel.green());
        buf.push(pixel.blue());
    }

    let mut jpeg_buffer = Vec::new();
    let encoder = Encoder::new(&mut jpeg_buffer, quality);
    encoder
        .encode(&buf, width as u16, height as u16, ColorType::Rgb)
        .ok()
        .unwrap();
    jpeg_buffer
}

pub fn to_base64_jpeg(svg: &[u8], size: u32, quality: u8) -> String {
    let buf = render_jpeg(svg, size, quality);
    let content = base64::engine::general_purpose::STANDARD.encode(&buf);
    format!("data:image/jpeg;base64,{}", content)
}
