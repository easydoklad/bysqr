use crc32fast::Hasher;
use liblzma::stream::{Action, Filters, LzmaOptions, Status, Stream};
use liblzma::write::XzEncoder;
use std::io::Write;

use crate::error::{Error, Result};

const BASE32HEX: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";
const LZMA_DICTIONARY_SIZE: u32 = 128 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub by_square_type: u8,
    pub version: u8,
    pub document_type: u8,
    pub reserved: u8,
}

impl Header {
    pub const PAY: Self = Self {
        by_square_type: 0,
        version: 0,
        document_type: 0,
        reserved: 0,
    };

    fn to_bytes(self) -> Result<[u8; 2]> {
        for (name, value) in [
            ("by square type", self.by_square_type),
            ("version", self.version),
            ("document type", self.document_type),
            ("reserved", self.reserved),
        ] {
            if value > 0x0f {
                return Err(Error::InvalidPayload(format!(
                    "{name} value {value} does not fit in four bits"
                )));
            }
        }

        let value = (u16::from(self.by_square_type) << 12)
            | (u16::from(self.version) << 8)
            | (u16::from(self.document_type) << 4)
            | u16::from(self.reserved);

        Ok(value.to_be_bytes())
    }

    fn from_bytes(bytes: [u8; 2]) -> Self {
        let value = u16::from_be_bytes(bytes);
        Self {
            by_square_type: ((value >> 12) & 0x0f) as u8,
            version: ((value >> 8) & 0x0f) as u8,
            document_type: ((value >> 4) & 0x0f) as u8,
            reserved: (value & 0x0f) as u8,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedPayload {
    pub header: Header,
    pub sequence: String,
}

pub fn encode_payload(header: Header, sequence: &str) -> Result<String> {
    let mut hasher = Hasher::new();
    hasher.update(sequence.as_bytes());

    let mut uncompressed = Vec::with_capacity(sequence.len() + 4);
    uncompressed.extend_from_slice(&hasher.finalize().to_le_bytes());
    uncompressed.extend_from_slice(sequence.as_bytes());

    let uncompressed_length =
        u16::try_from(uncompressed.len()).map_err(|_| Error::PayloadTooLong(uncompressed.len()))?;
    let compressed = compress(&uncompressed)?;

    let mut payload = Vec::with_capacity(compressed.len() + 4);
    payload.extend_from_slice(&header.to_bytes()?);
    payload.extend_from_slice(&uncompressed_length.to_le_bytes());
    payload.extend_from_slice(&compressed);

    Ok(base32hex_encode(&payload))
}

pub fn decode_payload(encoded: &str) -> Result<DecodedPayload> {
    let payload = base32hex_decode(encoded)?;
    if payload.len() < 5 {
        return Err(Error::InvalidPayload(
            "payload is shorter than its four-byte header".to_owned(),
        ));
    }

    let header = Header::from_bytes([payload[0], payload[1]]);
    let uncompressed_length = usize::from(u16::from_le_bytes([payload[2], payload[3]]));
    if uncompressed_length < 4 {
        return Err(Error::InvalidPayload(
            "declared uncompressed length cannot contain CRC32".to_owned(),
        ));
    }

    let uncompressed = decompress(&payload[4..], uncompressed_length)?;
    let expected_crc = u32::from_le_bytes(
        uncompressed[..4]
            .try_into()
            .expect("four CRC32 bytes were checked above"),
    );
    let sequence_bytes = &uncompressed[4..];

    let mut hasher = Hasher::new();
    hasher.update(sequence_bytes);
    let actual_crc = hasher.finalize();
    if expected_crc != actual_crc {
        return Err(Error::ChecksumMismatch {
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    Ok(DecodedPayload {
        header,
        sequence: String::from_utf8(sequence_bytes.to_vec())?,
    })
}

fn lzma_options() -> Result<LzmaOptions> {
    let mut options =
        LzmaOptions::new_preset(6).map_err(|error| Error::Compression(format!("{error:?}")))?;
    options
        .literal_context_bits(3)
        .literal_position_bits(0)
        .position_bits(2)
        .dict_size(LZMA_DICTIONARY_SIZE);
    Ok(options)
}

fn lzma_filters() -> Result<Filters> {
    let options = lzma_options()?;
    let mut filters = Filters::new();
    filters.lzma1(&options);
    Ok(filters)
}

fn compress(input: &[u8]) -> Result<Vec<u8>> {
    let filters = lzma_filters()?;
    let stream = Stream::new_raw_encoder(&filters)
        .map_err(|error| Error::Compression(format!("{error:?}")))?;
    let mut encoder = XzEncoder::new_stream(Vec::new(), stream);
    encoder
        .write_all(input)
        .map_err(|error| Error::Compression(error.to_string()))?;
    encoder
        .finish()
        .map_err(|error| Error::Compression(error.to_string()))
}

fn decompress(input: &[u8], expected_length: usize) -> Result<Vec<u8>> {
    let filters = lzma_filters()?;
    let mut stream = Stream::new_raw_decoder(&filters)
        .map_err(|error| Error::Compression(format!("{error:?}")))?;
    // Keep one spare byte so a payload that expands beyond its declared size
    // cannot be accepted merely because the output buffer filled up.
    let mut output = vec![0; expected_length + 1];

    let status = stream
        .process(input, &mut output, Action::Run)
        .map_err(|error| Error::Compression(format!("{error:?}")))?;

    let consumed = usize::try_from(stream.total_in()).map_err(|_| {
        Error::InvalidPayload("compressed input length does not fit usize".to_owned())
    })?;
    if status == Status::StreamEnd && consumed != input.len() {
        return Err(Error::InvalidPayload(format!(
            "compressed stream contains {} trailing bytes",
            input.len() - consumed
        )));
    }

    let actual_length = usize::try_from(stream.total_out()).map_err(|_| {
        Error::InvalidPayload("decompressed output length does not fit usize".to_owned())
    })?;
    if actual_length != expected_length {
        return Err(Error::InvalidPayload(format!(
            "declared uncompressed length is {expected_length}, decoded {actual_length} bytes"
        )));
    }

    output.truncate(expected_length);
    Ok(output)
}

fn base32hex_encode(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len().div_ceil(5) * 8);
    let mut buffer = 0_u32;
    let mut bits = 0_u8;

    for byte in input {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;

        while bits >= 5 {
            bits -= 5;
            let index = ((buffer >> bits) & 0x1f) as usize;
            output.push(char::from(BASE32HEX[index]));
        }
    }

    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        output.push(char::from(BASE32HEX[index]));
    }

    output
}

fn base32hex_decode(input: &str) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len() * 5 / 8);
    let mut buffer = 0_u32;
    let mut bits = 0_u8;

    for (index, character) in input.bytes().enumerate() {
        let value = match character {
            b'0'..=b'9' => character - b'0',
            b'A'..=b'V' => character - b'A' + 10,
            _ => {
                return Err(Error::InvalidPayload(format!(
                    "invalid Base32hex character at position {index}"
                )))
            }
        };

        buffer = (buffer << 5) | u32::from(value);
        bits += 5;

        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    if bits > 0 && buffer & ((1_u32 << bits) - 1) != 0 {
        return Err(Error::InvalidPayload(
            "non-zero Base32hex padding bits".to_owned(),
        ));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{
        base32hex_decode, base32hex_encode, compress, decode_payload, encode_payload, Header,
    };
    use crate::error::Error;

    #[test]
    fn base32hex_round_trip() {
        let data = b"\x00\x00\x61\x00PAY by square";
        let encoded = base32hex_encode(data);
        assert_eq!(base32hex_decode(&encoded).unwrap(), data);
    }

    #[test]
    fn payload_round_trip() {
        let sequence = "\t1\t1\t12.34\tEUR";
        let encoded = encode_payload(Header::PAY, sequence).unwrap();
        let decoded = decode_payload(&encoded).unwrap();

        assert_eq!(decoded.header, Header::PAY);
        assert_eq!(decoded.sequence, sequence);
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let sequence = b"\t1\t1\t12.34\tEUR";
        let mut uncompressed = vec![0, 0, 0, 0];
        uncompressed.extend_from_slice(sequence);

        let mut payload = Vec::new();
        payload.extend_from_slice(&Header::PAY.to_bytes().unwrap());
        payload.extend_from_slice(&(uncompressed.len() as u16).to_le_bytes());
        payload.extend_from_slice(&compress(&uncompressed).unwrap());

        assert!(matches!(
            decode_payload(&base32hex_encode(&payload)),
            Err(Error::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_incorrect_declared_length() {
        let encoded = encode_payload(Header::PAY, "\t1\t1\t12.34\tEUR").unwrap();
        let mut payload = base32hex_decode(&encoded).unwrap();
        let length = u16::from_le_bytes([payload[2], payload[3]]);
        payload[2..4].copy_from_slice(&(length - 1).to_le_bytes());

        assert!(matches!(
            decode_payload(&base32hex_encode(&payload)),
            Err(Error::InvalidPayload(_))
        ));
    }

    #[test]
    fn rejects_trailing_compressed_data() {
        let encoded = encode_payload(Header::PAY, "\t1\t1\t12.34\tEUR").unwrap();
        let mut payload = base32hex_decode(&encoded).unwrap();
        payload.push(0);

        assert!(matches!(
            decode_payload(&base32hex_encode(&payload)),
            Err(Error::InvalidPayload(_))
        ));
    }
}
