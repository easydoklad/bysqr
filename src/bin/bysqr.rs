use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(feature = "qr-reader")]
use bysqr::qr_reader;
use bysqr::{document, qr, Document};
use clap::{Parser, Subcommand, ValueEnum};

#[path = "../preview.rs"]
#[cfg(feature = "preview")]
mod preview;
#[path = "../utils.rs"]
mod utils;

use utils::ensure_directory_for_file;

#[derive(Parser)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Encode {
        #[arg(long = "src")]
        src: String,

        #[arg(long = "format")]
        format: Option<String>,

        #[arg(long = "preview")]
        preview: bool,

        #[arg(long = "size", default_value = "512")]
        size: u32,

        #[arg(long = "quality", default_value = "90")]
        quality: u8,

        #[arg(long = "save")]
        save: Option<PathBuf>,

        #[arg(long = "overwrite")]
        overwrite: bool,

        /// INVOICE logo composition from the official logo manual.
        #[arg(long = "invoice-layout", value_enum)]
        invoice_layout: Option<InvoiceLayoutArg>,

        /// Position of the INVOICE branding around the QR matrix.
        #[arg(long = "invoice-position", value_enum)]
        invoice_position: Option<InvoicePositionArg>,

        /// Approved INVOICE branding color variation.
        #[arg(long = "invoice-color", value_enum)]
        invoice_color: Option<InvoiceColorArg>,
    },
    Decode {
        #[arg(long = "src")]
        src: String,

        #[arg(long = "format", value_enum, default_value_t = DataFormat::Json)]
        format: DataFormat,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum DataFormat {
    Json,
    Xml,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum InvoiceLayoutArg {
    Print,
    Electronic,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum InvoicePositionArg {
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum InvoiceColorArg {
    Light,
    Dark,
    Gray,
    Black,
}

#[derive(Debug)]
enum ImageFormat {
    Svg,
    Png,
    Jpeg,
}

#[derive(Debug)]
enum OutputMode {
    Save(PathBuf, ImageFormat),
    Print(ImageFormat),
}

struct EncodeOptions<'a> {
    preview_requested: bool,
    requested_format: &'a Option<String>,
    destination: &'a Option<PathBuf>,
    size: u32,
    quality: u8,
    overwrite: bool,
    invoice_theme: Option<qr::InvoiceTheme>,
}

fn cli_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn parse_image_format(format: &str) -> Result<ImageFormat, io::Error> {
    match format.to_ascii_lowercase().as_str() {
        "svg" => Ok(ImageFormat::Svg),
        "png" => Ok(ImageFormat::Png),
        "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
        _ => Err(cli_error(format!(
            "invalid output: extension {format} is not supported"
        ))),
    }
}

fn guess_output_mode(
    destination: &Option<PathBuf>,
    requested_format: &Option<String>,
) -> Result<OutputMode, io::Error> {
    if let Some(destination) = destination {
        let extension = destination
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| cli_error("invalid output: unable to determine the file format"))?;
        Ok(OutputMode::Save(
            destination.clone(),
            parse_image_format(extension)?,
        ))
    } else {
        let format = requested_format.as_deref().ok_or_else(|| {
            cli_error("missing format: --format is required when printing to standard output")
        })?;
        Ok(OutputMode::Print(parse_image_format(format)?))
    }
}

fn read_source(source: &str) -> Result<String, io::Error> {
    if Path::new(source).is_file() {
        fs::read_to_string(source)
    } else {
        Ok(source.to_owned())
    }
}

fn deserialize_document(source: &str) -> Result<Document, Box<dyn Error>> {
    Ok(document::try_deserialize(&read_source(source)?)?)
}

fn invoice_theme(
    layout: Option<InvoiceLayoutArg>,
    position: Option<InvoicePositionArg>,
    color: Option<InvoiceColorArg>,
) -> Option<qr::InvoiceTheme> {
    if layout.is_none() && position.is_none() && color.is_none() {
        return None;
    }

    let default = qr::InvoiceTheme::default();
    Some(qr::InvoiceTheme::new(
        match layout {
            Some(InvoiceLayoutArg::Print) => qr::LogoLayout::Print,
            Some(InvoiceLayoutArg::Electronic) => qr::LogoLayout::Electronic,
            None => default.layout,
        },
        match position {
            Some(InvoicePositionArg::Bottom) => qr::LogoPosition::Bottom,
            Some(InvoicePositionArg::Top) => qr::LogoPosition::Top,
            Some(InvoicePositionArg::Left) => qr::LogoPosition::Left,
            Some(InvoicePositionArg::Right) => qr::LogoPosition::Right,
            None => default.position,
        },
        match color {
            Some(InvoiceColorArg::Light) => qr::InvoiceColor::Light,
            Some(InvoiceColorArg::Dark) => qr::InvoiceColor::Dark,
            Some(InvoiceColorArg::Gray) => qr::InvoiceColor::Gray,
            Some(InvoiceColorArg::Black) => qr::InvoiceColor::Black,
            None => default.color,
        },
    ))
}

fn create_svg(
    document: &Document,
    payload: &str,
    invoice_theme: Option<qr::InvoiceTheme>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    match document {
        Document::Pay(_) if invoice_theme.is_none() => {
            Ok(qr::create_pay_svg(payload, qr::Theme::default()))
        }
        Document::Invoice(_) => Ok(qr::create_invoice_svg_with_theme(
            payload,
            invoice_theme.unwrap_or_default(),
        )),
        Document::InvoiceItems(_) if invoice_theme.is_none() => {
            Ok(qr::create_invoice_items_svg(payload))
        }
        Document::Pay(_) | Document::InvoiceItems(_) => Err(cli_error(
            "--invoice-layout, --invoice-position and --invoice-color only apply to INVOICE documents",
        )
        .into()),
        _ => Err(cli_error("this by-square document type cannot be rendered yet").into()),
    }
}

fn run_encode(source: &str, options: EncodeOptions<'_>) -> Result<(), Box<dyn Error>> {
    let document = deserialize_document(source)?;
    let encoded = document.encode()?;
    let svg_code = create_svg(&document, &encoded, options.invoice_theme)?;

    if options.preview_requested {
        #[cfg(feature = "preview")]
        {
            preview::show_svg(svg_code);
            return Ok(());
        }

        #[cfg(not(feature = "preview"))]
        {
            return Err(cli_error(
                "preview is unavailable because the binary was built without the preview feature",
            )
            .into());
        }
    }

    match guess_output_mode(options.destination, options.requested_format)? {
        OutputMode::Save(destination, output_format) => {
            if destination.exists() && !options.overwrite {
                return Err(cli_error(format!(
                    "output file {} already exists; pass --overwrite to replace it",
                    destination.display()
                ))
                .into());
            }

            let content = match output_format {
                ImageFormat::Svg => svg_code,
                ImageFormat::Png => qr::render_png(&svg_code, options.size),
                ImageFormat::Jpeg => qr::render_jpeg(&svg_code, options.size, options.quality),
            };

            ensure_directory_for_file(&destination)?;
            fs::write(destination, content)?;
        }
        OutputMode::Print(output_format) => match output_format {
            ImageFormat::Svg => println!("{}", String::from_utf8(svg_code)?),
            ImageFormat::Png => println!("{}", qr::to_base64_png(&svg_code, options.size)),
            ImageFormat::Jpeg => println!(
                "{}",
                qr::to_base64_jpeg(&svg_code, options.size, options.quality)
            ),
        },
    }

    Ok(())
}

fn run_decode(source: &str, format: &DataFormat) -> Result<(), Box<dyn Error>> {
    let document = decode_source(source)?;
    let output = match format {
        DataFormat::Json => document.to_json_pretty()?,
        DataFormat::Xml => document.to_xml()?,
    };
    println!("{output}");
    Ok(())
}

fn decode_source(source: &str) -> Result<Document, Box<dyn Error>> {
    let path = Path::new(source);
    if !path.is_file() {
        return Ok(document::decode(source.trim())?);
    }

    let bytes = fs::read(path)?;
    if image::guess_format(&bytes).is_ok() {
        #[cfg(feature = "qr-reader")]
        return Ok(qr_reader::decode_document_from_bytes(&bytes)?);

        #[cfg(not(feature = "qr-reader"))]
        return Err(cli_error(
            "QR image decoding is unavailable; rebuild with the qr-reader feature",
        )
        .into());
    }

    let payload = String::from_utf8(bytes)?;
    Ok(document::decode(payload.trim())?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Encode {
            src,
            preview,
            format,
            save,
            size,
            quality,
            overwrite,
            invoice_layout,
            invoice_position,
            invoice_color,
        }) => run_encode(
            src,
            EncodeOptions {
                preview_requested: *preview,
                requested_format: format,
                destination: save,
                size: *size,
                quality: *quality,
                overwrite: *overwrite,
                invoice_theme: invoice_theme(*invoice_layout, *invoice_position, *invoice_color),
            },
        ),
        Some(Commands::Decode { src, format }) => run_decode(src, format),
        None => Ok(()),
    }
}
