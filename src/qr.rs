use base64::Engine;
use jpeg_encoder::{ColorType, Encoder};
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use resvg::tiny_skia::Pixmap;
use std::collections::HashMap;
use usvg::{Options, Transform, Tree};
use xmltree::{Element, EmitterConfig};

mod branding;
mod invoice;
mod items;
mod logo;
mod pay;

pub const CONTAINER_WIDTH: f32 = 512.0;
pub const CONTAINER_HEIGHT: f32 = 600.0;

/// Approved by-square logo composition.
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

/// Approved semantic color variation from the logo manual.
///
/// PAY and INVOICE use different brand colors for the light and dark
/// variants. Gray and black are shared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogoColor {
    Light,
    Dark,
    Gray,
    Black,
}

impl LogoColor {
    pub const ALL: [Self; 4] = [Self::Light, Self::Dark, Self::Gray, Self::Black];

    pub const fn pay_hex(self) -> &'static str {
        match self {
            Self::Light => "#A1C7E9",
            Self::Dark => "#6FA4D7",
            Self::Gray => "#5F6062",
            Self::Black => "#000000",
        }
    }

    pub const fn invoice_hex(self) -> &'static str {
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

/// One logo-manual-compliant PAY or INVOICE visual theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoTheme {
    pub layout: LogoLayout,
    pub position: LogoPosition,
    pub color: LogoColor,
}

impl LogoTheme {
    pub const fn new(layout: LogoLayout, position: LogoPosition, color: LogoColor) -> Self {
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

impl Default for LogoTheme {
    fn default() -> Self {
        Self::new(LogoLayout::Print, LogoPosition::Bottom, LogoColor::Dark)
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

fn insert_by_square_text(svg: &mut Element, color: &str) {
    branding::BY_SQUARE_WORDMARK.insert_at(svg, 193.0, 539.0, 39.0, color);
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

/// Render a PAY by square payload using one approved logo-manual theme.
pub fn create_pay_svg(content: &str, theme: LogoTheme) -> Vec<u8> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .expect("unable to create QR code");

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(pay::QR_MAX_DIMENSION, pay::QR_MAX_DIMENSION)
        .quiet_zone(false)
        .build();

    let (width, height) = theme.dimensions();
    let (center_x, center_y) = theme.qr_center();
    let mut svg = create_empty_svg(width, height);
    insert_background(&mut svg, "#ffffff", width, height);
    insert_qr_content(&mut svg, &svg_image, center_x, center_y);
    pay::decorate(&mut svg, theme);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .expect("unable to write generated SVG. possible XML corruption");
    qr
}

/// Render an INVOICE by square payload with the standard orange branding.
pub fn create_invoice_svg(content: &str) -> Vec<u8> {
    create_invoice_svg_with_theme(content, LogoTheme::default())
}

/// Render an INVOICE by square payload using one approved logo-manual theme.
pub fn create_invoice_svg_with_theme(content: &str, theme: LogoTheme) -> Vec<u8> {
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
