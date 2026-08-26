use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidInput {
        field: &'static str,
        message: String,
    },
    Unsupported(String),
    SequenceTooLong {
        actual: usize,
        maximum: usize,
    },
    PayloadTooLong(usize),
    InvalidPayload(String),
    InvalidSequence {
        position: usize,
        field: &'static str,
        message: String,
    },
    Compression(String),
    ChecksumMismatch {
        expected: u32,
        actual: u32,
    },
    Deserialize {
        format: &'static str,
        message: String,
    },
    QrEncode(String),
    SvgRender(String),
    ImageEncode {
        format: &'static str,
        message: String,
    },
    ImageDecode(String),
    QrNotFound,
    QrDecode(String),
    PayQrNotFound {
        decoded: usize,
    },
    InvoiceItemsQrNotFound {
        decoded: usize,
    },
    MultiplePayQrCodes(usize),
    BySquareQrNotFound {
        decoded: usize,
    },
    MultipleBySquareQrCodes(usize),
    Utf8(std::string::FromUtf8Error),
}

impl Error {
    pub(crate) fn invalid(field: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { field, message } => {
                write!(formatter, "invalid {field}: {message}")
            }
            Self::Unsupported(message) => formatter.write_str(message),
            Self::SequenceTooLong { actual, maximum } => write!(
                formatter,
                "by-square sequence has {actual} characters; the maximum is {maximum}"
            ),
            Self::PayloadTooLong(actual) => write!(
                formatter,
                "uncompressed payload has {actual} bytes; the 16-bit limit is {}",
                u16::MAX
            ),
            Self::InvalidPayload(message) => {
                write!(formatter, "invalid by square payload: {message}")
            }
            Self::InvalidSequence {
                position,
                field,
                message,
            } => write!(
                formatter,
                "invalid by-square sequence field {field} at position {position}: {message}"
            ),
            Self::Compression(message) => write!(formatter, "LZMA error: {message}"),
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "CRC32 mismatch: payload contains {expected:#010x}, calculated {actual:#010x}"
            ),
            Self::Deserialize { format, message } => {
                write!(formatter, "unable to deserialize {format}: {message}")
            }
            Self::QrEncode(message) => write!(formatter, "unable to encode QR code: {message}"),
            Self::SvgRender(message) => write!(formatter, "unable to render SVG: {message}"),
            Self::ImageEncode { format, message } => {
                write!(formatter, "unable to encode {format} image: {message}")
            }
            Self::ImageDecode(message) => write!(formatter, "unable to decode image: {message}"),
            Self::QrNotFound => formatter.write_str("image does not contain a detectable QR code"),
            Self::QrDecode(message) => write!(formatter, "unable to decode QR code: {message}"),
            Self::PayQrNotFound { decoded } => write!(
                formatter,
                "decoded {decoded} QR codes, but none contains a valid PAY by square payload"
            ),
            Self::InvoiceItemsQrNotFound { decoded } => write!(
                formatter,
                "decoded {decoded} QR codes, but none contains a valid INVOICE ITEMS by square payload"
            ),
            Self::MultiplePayQrCodes(count) => write!(
                formatter,
                "image contains {count} valid PAY by square QR codes; expected exactly one"
            ),
            Self::BySquareQrNotFound { decoded } => write!(
                formatter,
                "decoded {decoded} QR codes, but none contains a valid by-square payload"
            ),
            Self::MultipleBySquareQrCodes(count) => write!(
                formatter,
                "image contains {count} valid by-square QR codes; expected exactly one"
            ),
            Self::Utf8(error) => write!(formatter, "payload sequence is not valid UTF-8: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Utf8(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::Utf8(error)
    }
}
