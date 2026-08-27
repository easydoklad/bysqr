use std::{
    error::Error,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[cfg(feature = "qr-reader")]
use bysqr::qr_reader;
use bysqr::{
    document,
    invoice::{self, Invoice},
    invoice_items::{self, InvoiceItemsList},
    qr, Document,
};
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

        /// PAY or INVOICE logo composition from the by-square logo manual.
        #[arg(long = "logo-layout", value_enum)]
        logo_layout: Option<LogoLayoutArg>,

        /// Position of the PAY or INVOICE branding around the QR matrix.
        #[arg(long = "logo-position", value_enum)]
        logo_position: Option<LogoPositionArg>,

        /// Family-specific PAY or INVOICE color variation.
        #[arg(long = "logo-color", value_enum)]
        logo_color: Option<LogoColorArg>,
    },
    Decode {
        #[arg(long = "src")]
        src: String,

        #[arg(long = "format", value_enum, default_value_t = DataFormat::Json)]
        format: DataFormat,
    },
    EncodeItems {
        #[arg(long = "src")]
        src: String,

        #[arg(long = "format", value_enum)]
        format: ImageFormat,

        #[arg(long = "size", default_value = "512")]
        size: u32,

        #[arg(long = "quality", default_value = "90")]
        quality: u8,

        /// Directory for deterministic invoice-items-NNN output files.
        #[arg(long = "save")]
        save: Option<PathBuf>,

        #[arg(long = "overwrite")]
        overwrite: bool,

        /// Canonical parent INVOICE JSON/XML, inline or as a file path.
        #[arg(long = "invoice-src")]
        invoice_src: Option<String>,
    },
    DecodeItems {
        /// JSON array of textual INVOICE ITEMS QR payloads.
        #[arg(long = "src")]
        src: String,

        #[arg(long = "format", value_enum, default_value_t = DataFormat::Json)]
        format: DataFormat,

        /// Canonical parent INVOICE JSON/XML, inline or as a file path.
        #[arg(long = "invoice-src")]
        invoice_src: Option<String>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum DataFormat {
    Json,
    Xml,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogoLayoutArg {
    Print,
    Electronic,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogoPositionArg {
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogoColorArg {
    Light,
    Dark,
    Gray,
    Black,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ImageFormat {
    Svg,
    Png,
    #[value(alias = "jpg")]
    Jpeg,
}

impl ImageFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Jpeg => "jpeg",
        }
    }
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
    logo_theme: Option<qr::LogoTheme>,
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
    if source == "-" {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        Ok(input)
    } else if Path::new(source).is_file() {
        fs::read_to_string(source)
    } else {
        Ok(source.to_owned())
    }
}

fn deserialize_document(source: &str) -> Result<Document, Box<dyn Error>> {
    Ok(document::try_deserialize(&read_source(source)?)?)
}

fn deserialize_invoice_items_list(source: &str) -> Result<InvoiceItemsList, Box<dyn Error>> {
    let input = read_source(source)?;
    let input = input.trim_start();

    if input.starts_with('<') {
        return InvoiceItemsList::from_xml_str(input).map_err(|error| {
            cli_error(format!(
                "unable to deserialize InvoiceItemsList XML: {error}"
            ))
            .into()
        });
    }
    if input.starts_with('{') {
        return serde_json::from_str(input).map_err(|error| {
            cli_error(format!(
                "unable to deserialize InvoiceItemsList JSON: {error}"
            ))
            .into()
        });
    }

    Err(cli_error("expected an InvoiceItemsList XML document or JSON object").into())
}

fn deserialize_invoice(source: &str) -> Result<Invoice, Box<dyn Error>> {
    if source == "-" {
        return Err(
            cli_error("--invoice-src - is not supported; standard input belongs to --src").into(),
        );
    }

    invoice::try_deserialize_invoice(&read_source(source)?)
        .map_err(|error| cli_error(format!("unable to deserialize parent Invoice: {error}")).into())
}

fn validate_against_invoice(
    items: &InvoiceItemsList,
    invoice_source: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if let Some(source) = invoice_source {
        items.validate_against_invoice(&deserialize_invoice(source)?)?;
    }
    Ok(())
}

fn logo_theme(
    layout: Option<LogoLayoutArg>,
    position: Option<LogoPositionArg>,
    color: Option<LogoColorArg>,
) -> Option<qr::LogoTheme> {
    if layout.is_none() && position.is_none() && color.is_none() {
        return None;
    }

    let default = qr::LogoTheme::default();
    Some(qr::LogoTheme::new(
        match layout {
            Some(LogoLayoutArg::Print) => qr::LogoLayout::Print,
            Some(LogoLayoutArg::Electronic) => qr::LogoLayout::Electronic,
            None => default.layout,
        },
        match position {
            Some(LogoPositionArg::Bottom) => qr::LogoPosition::Bottom,
            Some(LogoPositionArg::Top) => qr::LogoPosition::Top,
            Some(LogoPositionArg::Left) => qr::LogoPosition::Left,
            Some(LogoPositionArg::Right) => qr::LogoPosition::Right,
            None => default.position,
        },
        match color {
            Some(LogoColorArg::Light) => qr::LogoColor::Light,
            Some(LogoColorArg::Dark) => qr::LogoColor::Dark,
            Some(LogoColorArg::Gray) => qr::LogoColor::Gray,
            Some(LogoColorArg::Black) => qr::LogoColor::Black,
            None => default.color,
        },
    ))
}

fn create_svg(
    document: &Document,
    payload: &str,
    logo_theme: Option<qr::LogoTheme>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    match document {
        Document::Pay(_) => Ok(qr::create_pay_svg_with_theme(
            payload,
            logo_theme.unwrap_or_default(),
        )?),
        Document::Invoice(_) => Ok(qr::create_invoice_svg_with_theme(
            payload,
            logo_theme.unwrap_or_default(),
        )?),
        Document::InvoiceItems(_) if logo_theme.is_none() => {
            Ok(qr::create_invoice_items_svg(payload)?)
        }
        Document::InvoiceItems(_) => Err(cli_error(
            "--logo-layout, --logo-position and --logo-color only apply to PAY and INVOICE documents",
        )
        .into()),
        _ => Err(cli_error("this by-square document type cannot be rendered yet").into()),
    }
}

fn run_encode(source: &str, options: EncodeOptions<'_>) -> Result<(), Box<dyn Error>> {
    let document = deserialize_document(source)?;
    let encoded = document.encode()?;
    let svg_code = create_svg(&document, &encoded, options.logo_theme)?;

    if options.preview_requested {
        #[cfg(feature = "preview")]
        {
            preview::show_svg(svg_code)?;
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
                ImageFormat::Png => qr::render_png(&svg_code, options.size)?,
                ImageFormat::Jpeg => qr::render_jpeg(&svg_code, options.size, options.quality)?,
            };

            ensure_directory_for_file(&destination)?;
            fs::write(destination, content)?;
        }
        OutputMode::Print(output_format) => match output_format {
            ImageFormat::Svg => println!("{}", String::from_utf8(svg_code)?),
            ImageFormat::Png => println!("{}", qr::to_base64_png(&svg_code, options.size)?),
            ImageFormat::Jpeg => println!(
                "{}",
                qr::to_base64_jpeg(&svg_code, options.size, options.quality)?
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

fn render_items_for_stdout(
    payloads: &[String],
    format: ImageFormat,
    size: u32,
    quality: u8,
) -> Result<Vec<String>, Box<dyn Error>> {
    payloads
        .iter()
        .map(|payload| {
            let svg = qr::create_invoice_items_svg(payload)?;
            match format {
                ImageFormat::Svg => Ok(String::from_utf8(svg)?),
                ImageFormat::Png => Ok(qr::to_base64_png(&svg, size)?),
                ImageFormat::Jpeg => Ok(qr::to_base64_jpeg(&svg, size, quality)?),
            }
        })
        .collect()
}

fn render_items_for_files(
    payloads: &[String],
    format: ImageFormat,
    size: u32,
    quality: u8,
) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    payloads
        .iter()
        .map(|payload| {
            let svg = qr::create_invoice_items_svg(payload)?;
            match format {
                ImageFormat::Svg => Ok(svg),
                ImageFormat::Png => Ok(qr::render_png(&svg, size)?),
                ImageFormat::Jpeg => Ok(qr::render_jpeg(&svg, size, quality)?),
            }
        })
        .collect()
}

fn is_generated_items_output(file_name: &str, extension: &str) -> bool {
    let Some(index) = file_name
        .strip_prefix("invoice-items-")
        .and_then(|name| name.strip_suffix(&format!(".{extension}")))
    else {
        return false;
    };

    let Ok(index_value) = index.parse::<usize>() else {
        return false;
    };
    index_value > 0 && format!("{index_value:03}") == index
}

fn save_items_outputs(
    directory: &Path,
    format: ImageFormat,
    contents: &[Vec<u8>],
    overwrite: bool,
) -> Result<(), Box<dyn Error>> {
    if directory.exists() && !directory.is_dir() {
        return Err(cli_error(format!(
            "batch output path {} must be a directory",
            directory.display()
        ))
        .into());
    }

    let destinations = (1..=contents.len())
        .map(|index| directory.join(format!("invoice-items-{index:03}.{}", format.extension())))
        .collect::<Vec<_>>();

    let existing_outputs = if directory.is_dir() {
        fs::read_dir(directory)?
            .filter_map(|entry| match entry {
                Ok(entry)
                    if entry.file_name().to_str().is_some_and(|name| {
                        is_generated_items_output(name, format.extension())
                    }) =>
                {
                    Some(Ok(entry.path()))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, io::Error>>()?
    } else {
        Vec::new()
    };

    if !overwrite {
        if let Some(existing) = existing_outputs.first() {
            return Err(cli_error(format!(
                "generated batch output {} already exists; pass --overwrite to replace the batch",
                existing.display()
            ))
            .into());
        }
    }

    for destination in &destinations {
        if destination.exists() && !overwrite {
            return Err(cli_error(format!(
                "output file {} already exists; pass --overwrite to replace the batch",
                destination.display()
            ))
            .into());
        }
        if destination.is_dir() {
            return Err(cli_error(format!(
                "output path {} points to a directory",
                destination.display()
            ))
            .into());
        }
    }

    let stale_outputs = existing_outputs
        .iter()
        .filter(|existing| !destinations.contains(existing))
        .collect::<Vec<_>>();
    for stale in &stale_outputs {
        if stale.is_dir() {
            return Err(cli_error(format!(
                "generated output path {} points to a directory",
                stale.display()
            ))
            .into());
        }
    }

    fs::create_dir_all(directory)?;
    for stale in stale_outputs {
        fs::remove_file(stale)?;
    }
    for (destination, content) in destinations.iter().zip(contents) {
        fs::write(destination, content)?;
    }
    Ok(())
}

fn run_encode_items(
    source: &str,
    format: ImageFormat,
    size: u32,
    quality: u8,
    destination: Option<&Path>,
    overwrite: bool,
    invoice_source: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let items = deserialize_invoice_items_list(source)?;
    validate_against_invoice(&items, invoice_source)?;
    let payloads = items.encode_chunks()?;

    if let Some(directory) = destination {
        let contents = render_items_for_files(&payloads, format, size, quality)?;
        save_items_outputs(directory, format, &contents, overwrite)?;
    } else {
        let output = render_items_for_stdout(&payloads, format, size, quality)?;
        println!("{}", serde_json::to_string(&output)?);
    }

    Ok(())
}

fn run_decode_items(
    source: &str,
    format: &DataFormat,
    invoice_source: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let input = read_source(source)?;
    let payloads: Vec<String> = serde_json::from_str(&input).map_err(|error| {
        cli_error(format!(
            "unable to deserialize INVOICE ITEMS payload array: {error}"
        ))
    })?;
    if payloads.is_empty() {
        return Err(cli_error("INVOICE ITEMS payload array must not be empty").into());
    }

    let items = invoice_items::decode_chunks(payloads.iter().map(|payload| payload.trim()))?;
    validate_against_invoice(&items, invoice_source)?;
    let output = match format {
        DataFormat::Json => serde_json::to_string_pretty(&items)?,
        DataFormat::Xml => items.to_xml_string()?,
    };
    println!("{output}");
    Ok(())
}

fn decode_source(source: &str) -> Result<Document, Box<dyn Error>> {
    if source == "-" {
        return Ok(document::decode(read_source(source)?.trim())?);
    }

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

fn run() -> Result<(), Box<dyn Error>> {
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
            logo_layout,
            logo_position,
            logo_color,
        }) => run_encode(
            src,
            EncodeOptions {
                preview_requested: *preview,
                requested_format: format,
                destination: save,
                size: *size,
                quality: *quality,
                overwrite: *overwrite,
                logo_theme: logo_theme(*logo_layout, *logo_position, *logo_color),
            },
        ),
        Some(Commands::Decode { src, format }) => run_decode(src, format),
        Some(Commands::EncodeItems {
            src,
            format,
            size,
            quality,
            save,
            overwrite,
            invoice_src,
        }) => run_encode_items(
            src,
            *format,
            *size,
            *quality,
            save.as_deref(),
            *overwrite,
            invoice_src.as_deref(),
        ),
        Some(Commands::DecodeItems {
            src,
            format,
            invoice_src,
        }) => run_decode_items(src, format, invoice_src.as_deref()),
        None => Ok(()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::is_generated_items_output;

    #[test]
    fn recognizes_only_deterministic_batch_output_names() {
        for name in [
            "invoice-items-001.svg",
            "invoice-items-999.svg",
            "invoice-items-1000.svg",
        ] {
            assert!(is_generated_items_output(name, "svg"), "{name}");
        }

        for name in [
            "invoice-items-000.svg",
            "invoice-items-01.svg",
            "invoice-items-0001.svg",
            "invoice-items-one.svg",
            "invoice-items-001.png",
            "other-001.svg",
        ] {
            assert!(!is_generated_items_output(name, "svg"), "{name}");
        }
    }
}
