use base64::Engine;
use jpeg_encoder::{ColorType, Encoder};
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use resvg::tiny_skia::Pixmap;
use std::collections::HashMap;
use usvg::{Options, Transform, Tree};
use xmltree::{Element, EmitterConfig};

use crate::error::{Error, Result};

mod branding;
mod invoice;
mod items;
mod logo;
mod pay;

pub const CONTAINER_WIDTH: f32 = 512.0;
pub const CONTAINER_HEIGHT: f32 = 600.0;
/// Maximum width or height accepted by the raster renderers.
///
/// This keeps caller-controlled dimensions from causing unbounded allocations.
pub const MAX_RASTER_DIMENSION: u32 = 8_192;

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

fn insert_qr_content(svg: &mut Element, qr: &str, center_x: f32, center_y: f32) -> Result<()> {
    let qr_svg = Element::parse(qr.as_bytes())
        .map_err(|error| Error::SvgRender(format!("invalid generated QR SVG: {error}")))?;

    let qr_width: f32 = qr_svg
        .attributes
        .get("width")
        .ok_or_else(|| Error::SvgRender("generated QR SVG has no width".to_string()))?
        .parse()
        .map_err(|error| Error::SvgRender(format!("invalid generated QR SVG width: {error}")))?;

    let qr_height: f32 = qr_svg
        .attributes
        .get("height")
        .ok_or_else(|| Error::SvgRender("generated QR SVG has no height".to_string()))?
        .parse()
        .map_err(|error| Error::SvgRender(format!("invalid generated QR SVG height: {error}")))?;

    let qr_path = qr_svg
        .get_child("path")
        .ok_or_else(|| Error::SvgRender("generated QR SVG has no path".to_string()))?
        .attributes
        .get("d")
        .ok_or_else(|| Error::SvgRender("generated QR SVG path has no data".to_string()))?;

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
    Ok(())
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

/// Render a PAY by square payload with the default approved theme.
pub fn create_pay_svg(content: &str) -> Result<Vec<u8>> {
    create_pay_svg_with_theme(content, LogoTheme::default())
}

/// Render a PAY by square payload using one approved logo-manual theme.
pub fn create_pay_svg_with_theme(content: &str, theme: LogoTheme) -> Result<Vec<u8>> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .map_err(|error| Error::QrEncode(error.to_string()))?;

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(pay::QR_MAX_DIMENSION, pay::QR_MAX_DIMENSION)
        .quiet_zone(true)
        .build();

    let (width, height) = theme.dimensions();
    let (center_x, center_y) = theme.qr_center();
    let mut svg = create_empty_svg(width, height);
    insert_background(&mut svg, "#ffffff", width, height);
    insert_qr_content(&mut svg, &svg_image, center_x, center_y)?;
    pay::decorate(&mut svg, theme);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .map_err(|error| Error::SvgRender(error.to_string()))?;
    Ok(qr)
}

/// Render an INVOICE by square payload with the standard orange branding.
pub fn create_invoice_svg(content: &str) -> Result<Vec<u8>> {
    create_invoice_svg_with_theme(content, LogoTheme::default())
}

/// Render an INVOICE by square payload using one approved logo-manual theme.
pub fn create_invoice_svg_with_theme(content: &str, theme: LogoTheme) -> Result<Vec<u8>> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .map_err(|error| Error::QrEncode(error.to_string()))?;

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(invoice::QR_MAX_DIMENSION, invoice::QR_MAX_DIMENSION)
        .quiet_zone(true)
        .build();

    let (width, height) = theme.dimensions();
    let (center_x, center_y) = theme.qr_center();
    let mut svg = create_empty_svg(width, height);
    insert_background(&mut svg, "#ffffff", width, height);
    insert_qr_content(&mut svg, &svg_image, center_x, center_y)?;
    invoice::decorate(&mut svg, theme);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .map_err(|error| Error::SvgRender(error.to_string()))?;
    Ok(qr)
}

/// Render an INVOICE ITEMS by square payload with the standard black branding.
pub fn create_invoice_items_svg(content: &str) -> Result<Vec<u8>> {
    let code = QrCode::with_error_correction_level(content.as_bytes(), EcLevel::L)
        .map_err(|error| Error::QrEncode(error.to_string()))?;

    let svg_image = code
        .render::<svg::Color>()
        .max_dimensions(items::QR_MAX_DIMENSION, items::QR_MAX_DIMENSION)
        .quiet_zone(true)
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
    )?;
    items::decorate(&mut svg);

    let mut qr = Vec::new();
    let emitter_options = EmitterConfig::default().write_document_declaration(false);
    svg.write_with_config(&mut qr, emitter_options)
        .map_err(|error| Error::SvgRender(error.to_string()))?;
    Ok(qr)
}

pub fn map_svg(svg: &[u8], size: u32) -> Result<Pixmap> {
    validate_raster_dimension("size", size)?;
    let svg_tree = Tree::from_data(svg, &Options::default())
        .map_err(|error| Error::SvgRender(error.to_string()))?;
    let source_size = svg_tree.size();
    let scale = size as f32 / source_size.width();
    let width: u32 = size;
    let height = (source_size.height() * scale).round() as u32;
    validate_raster_dimension("calculated height", height)?;

    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| Error::ImageEncode {
        format: "raster",
        message: format!("unable to allocate {width} × {height} pixel buffer"),
    })?;
    resvg::render(
        &svg_tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

pub fn render_png(svg: &[u8], size: u32) -> Result<Vec<u8>> {
    let pixmap = map_svg(svg, size)?;

    pixmap.encode_png().map_err(|error| Error::ImageEncode {
        format: "PNG",
        message: error.to_string(),
    })
}

pub fn to_base64_png(svg: &[u8], size: u32) -> Result<String> {
    let buf = render_png(svg, size)?;
    let base64_content = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/png;base64,{base64_content}"))
}

pub fn render_jpeg(svg: &[u8], size: u32, quality: u8) -> Result<Vec<u8>> {
    if !(1..=100).contains(&quality) {
        return Err(Error::invalid("quality", "must be between 1 and 100"));
    }

    let pixmap = map_svg(svg, size)?;
    let (width, height) = (pixmap.width(), pixmap.height());
    let width_u16 = u16::try_from(width)
        .map_err(|_| Error::invalid("size", "JPEG width exceeds the 16-bit encoder limit"))?;
    let height_u16 = u16::try_from(height).map_err(|_| {
        Error::invalid(
            "calculated height",
            "JPEG height exceeds the 16-bit encoder limit",
        )
    })?;
    let mut buf = Vec::with_capacity(width as usize * height as usize * 3);

    for pixel in pixmap.pixels() {
        buf.push(pixel.red());
        buf.push(pixel.green());
        buf.push(pixel.blue());
    }

    let mut jpeg_buffer = Vec::new();
    let encoder = Encoder::new(&mut jpeg_buffer, quality);
    encoder
        .encode(&buf, width_u16, height_u16, ColorType::Rgb)
        .map_err(|error| Error::ImageEncode {
            format: "JPEG",
            message: error.to_string(),
        })?;
    Ok(jpeg_buffer)
}

pub fn to_base64_jpeg(svg: &[u8], size: u32, quality: u8) -> Result<String> {
    let buf = render_jpeg(svg, size, quality)?;
    let content = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/jpeg;base64,{content}"))
}

fn validate_raster_dimension(field: &'static str, value: u32) -> Result<()> {
    if value == 0 {
        return Err(Error::invalid(field, "must be greater than zero"));
    }
    if value > MAX_RASTER_DIMENSION {
        return Err(Error::invalid(
            field,
            format!("must not exceed {MAX_RASTER_DIMENSION} pixels"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="20"/></svg>"#;

    #[test]
    fn reports_qr_capacity_errors_instead_of_panicking() {
        let content = "x".repeat(4_000);
        assert!(matches!(create_pay_svg(&content), Err(Error::QrEncode(_))));
        assert!(matches!(
            create_invoice_svg(&content),
            Err(Error::QrEncode(_))
        ));
        assert!(matches!(
            create_invoice_items_svg(&content),
            Err(Error::QrEncode(_))
        ));
    }

    #[test]
    fn rejects_invalid_svg_and_unsafe_raster_dimensions() {
        assert!(matches!(map_svg(b"not svg", 512), Err(Error::SvgRender(_))));
        assert!(matches!(
            map_svg(SIMPLE_SVG, 0),
            Err(Error::InvalidInput { field: "size", .. })
        ));
        assert!(matches!(
            map_svg(SIMPLE_SVG, MAX_RASTER_DIMENSION + 1),
            Err(Error::InvalidInput { field: "size", .. })
        ));
        assert!(matches!(
            map_svg(SIMPLE_SVG, MAX_RASTER_DIMENSION),
            Err(Error::InvalidInput {
                field: "calculated height",
                ..
            })
        ));
    }

    #[test]
    fn validates_jpeg_quality() {
        for quality in [0, 101] {
            assert!(matches!(
                render_jpeg(SIMPLE_SVG, 32, quality),
                Err(Error::InvalidInput {
                    field: "quality",
                    ..
                })
            ));
        }
    }

    #[test]
    fn raster_and_data_url_renderers_are_fallible_and_usable() {
        let pixmap = map_svg(SIMPLE_SVG, 32).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (32, 64));
        assert!(render_png(SIMPLE_SVG, 32).unwrap().starts_with(b"\x89PNG"));
        assert!(render_jpeg(SIMPLE_SVG, 32, 90)
            .unwrap()
            .starts_with(&[0xff, 0xd8]));
        assert!(to_base64_png(SIMPLE_SVG, 32)
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(to_base64_jpeg(SIMPLE_SVG, 32, 90)
            .unwrap()
            .starts_with("data:image/jpeg;base64,"));
    }
}
