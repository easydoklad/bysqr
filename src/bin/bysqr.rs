use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(feature = "qr-reader")]
use bysqr::qr_reader;
use bysqr::{
    pay::{self, Pay},
    qr,
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

fn deserialize_pay(source: &str) -> Result<Pay, Box<dyn Error>> {
    Ok(pay::try_deserialize_pay(&read_source(source)?)?)
}

fn run_encode(
    source: &str,
    preview_requested: bool,
    requested_format: &Option<String>,
    destination: &Option<PathBuf>,
    size: u32,
    quality: u8,
    overwrite: bool,
) -> Result<(), Box<dyn Error>> {
    let pay = deserialize_pay(source)?;
    let encoded = pay::encode(&pay)?;
    let svg_code = qr::create_pay_svg(&encoded, qr::Theme::default());

    if preview_requested {
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

    match guess_output_mode(destination, requested_format)? {
        OutputMode::Save(destination, output_format) => {
            if destination.exists() && !overwrite {
                return Err(cli_error(format!(
                    "output file {} already exists; pass --overwrite to replace it",
                    destination.display()
                ))
                .into());
            }

            let content = match output_format {
                ImageFormat::Svg => svg_code,
                ImageFormat::Png => qr::render_png(&svg_code, size),
                ImageFormat::Jpeg => qr::render_jpeg(&svg_code, size, quality),
            };

            ensure_directory_for_file(&destination)?;
            fs::write(destination, content)?;
        }
        OutputMode::Print(output_format) => match output_format {
            ImageFormat::Svg => println!("{}", String::from_utf8(svg_code)?),
            ImageFormat::Png => println!("{}", qr::to_base64_png(&svg_code, size)),
            ImageFormat::Jpeg => println!("{}", qr::to_base64_jpeg(&svg_code, size, quality)),
        },
    }

    Ok(())
}

fn run_decode(source: &str, format: &DataFormat) -> Result<(), Box<dyn Error>> {
    let pay = decode_source(source)?;
    let output = match format {
        DataFormat::Json => serde_json::to_string_pretty(&pay)?,
        DataFormat::Xml => quick_xml::se::to_string(&pay)?,
    };
    println!("{output}");
    Ok(())
}

fn decode_source(source: &str) -> Result<Pay, Box<dyn Error>> {
    let path = Path::new(source);
    if !path.is_file() {
        return Ok(pay::decode(source.trim())?);
    }

    let bytes = fs::read(path)?;
    if image::guess_format(&bytes).is_ok() {
        #[cfg(feature = "qr-reader")]
        return Ok(qr_reader::decode_pay_from_bytes(&bytes)?);

        #[cfg(not(feature = "qr-reader"))]
        return Err(cli_error(
            "QR image decoding is unavailable; rebuild with the qr-reader feature",
        )
        .into());
    }

    let payload = String::from_utf8(bytes)?;
    Ok(pay::decode(payload.trim())?)
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
        }) => run_encode(src, *preview, format, save, *size, *quality, *overwrite),
        Some(Commands::Decode { src, format }) => run_decode(src, format),
        None => Ok(()),
    }
}
