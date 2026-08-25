use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

use bysqr::{
    encoder,
    models::{try_deserialize_pay, Pay},
    qr,
};
use clap::{Parser, Subcommand};

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
        #[arg(long = "src", required = false)]
        src: Option<String>,

        #[arg(long = "format", required = false)]
        format: Option<String>,

        #[arg(long = "preview", required = false)]
        preview: bool,

        #[arg(long = "size", required = false, default_value = "512")]
        size: u32,

        #[arg(long = "quality", required = false, default_value = "90")]
        quality: u8,

        #[arg(long = "save", required = false)]
        save: Option<PathBuf>,

        #[arg(long = "overwrite", required = false)]
        overwrite: bool,
    },
}

#[derive(Debug)]
enum OutputFormat {
    Svg,
    Png,
    Jpeg,
}

#[derive(Debug)]
enum OutputMode {
    Save(PathBuf, OutputFormat),
    Print(OutputFormat),
}

fn cli_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn parse_output_format(format: &str) -> Result<OutputFormat, io::Error> {
    match format.to_ascii_lowercase().as_str() {
        "svg" => Ok(OutputFormat::Svg),
        "png" => Ok(OutputFormat::Png),
        "jpg" | "jpeg" => Ok(OutputFormat::Jpeg),
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
            parse_output_format(extension)?,
        ))
    } else {
        let format = requested_format.as_deref().ok_or_else(|| {
            cli_error("missing format: --format is required when printing to standard output")
        })?;
        Ok(OutputMode::Print(parse_output_format(format)?))
    }
}

fn deserialize_pay(source: &str) -> Result<Pay, Box<dyn Error>> {
    let content = if Path::new(source).is_file() {
        fs::read_to_string(source)?
    } else {
        source.to_owned()
    };

    Ok(try_deserialize_pay(&content)?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(Commands::Encode {
        src,
        preview,
        format,
        save,
        size,
        quality,
        overwrite,
    }) = &cli.command
    {
        let source = src
            .as_deref()
            .ok_or_else(|| cli_error("missing source: --src is required"))?;
        let pay = deserialize_pay(source)?;
        let encoded = encoder::encode(&pay)?;
        let svg_code = qr::create_pay_svg(&encoded, qr::Theme::default());

        if *preview {
            #[cfg(feature = "preview")]
            preview::show_svg(svg_code.clone());

            #[cfg(not(feature = "preview"))]
            return Err(cli_error(
                "preview is unavailable because the binary was built without the preview feature",
            )
            .into());
        } else {
            match guess_output_mode(save, format)? {
                OutputMode::Save(destination, output_format) => {
                    if destination.exists() && !*overwrite {
                        return Err(cli_error(format!(
                            "output file {} already exists; pass --overwrite to replace it",
                            destination.display()
                        ))
                        .into());
                    }

                    let content = match output_format {
                        OutputFormat::Svg => svg_code,
                        OutputFormat::Png => qr::render_png(&svg_code, *size),
                        OutputFormat::Jpeg => qr::render_jpeg(&svg_code, *size, *quality),
                    };

                    ensure_directory_for_file(&destination)?;
                    fs::write(destination, content)?;
                }
                OutputMode::Print(output_format) => match output_format {
                    OutputFormat::Svg => println!("{}", String::from_utf8(svg_code)?),
                    OutputFormat::Png => println!("{}", qr::to_base64_png(&svg_code, *size)),
                    OutputFormat::Jpeg => {
                        println!("{}", qr::to_base64_jpeg(&svg_code, *size, *quality));
                    }
                },
            }
        }
    }

    Ok(())
}
