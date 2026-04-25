use std::{
    io::{BufRead, Error, Write},
    num::ParseIntError,
};

use thiserror::Error as ThisError;

use crate::{
    element::Element,
    serializers::{BinarySerializationError, BinarySerializer, KeyValues2FlatSerializer, KeyValues2SerializationError, KeyValues2Serializer},
};

#[derive(Debug, ThisError)]
pub enum FileHeaderError {
    #[error("IO error: {0}")]
    Io(#[from] Error),
    #[error("Integer Parse Error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("The Header Was In An Invalid Format")]
    InvalidFileHeader,
    #[error("Header Was Legacy With An Invalid Encoding")]
    UnknownLegacyEncoding(String),
}

const CURRENT_ENCODING: &str = "dmx";
const CURRENT_FORMAT_VERSION: i32 = 18;

#[derive(Debug, Clone)]
pub struct Header {
    pub format: String,
    pub format_version: i32,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            format: String::from(CURRENT_ENCODING),
            format_version: CURRENT_FORMAT_VERSION,
        }
    }
}

impl Header {
    pub fn new(format: impl Into<String>, format_version: i32) -> Self {
        let format = format.into();
        Self { format, format_version }
    }

    pub fn from_string(value: String) -> Result<(Self, String, i32), FileHeaderError> {
        const HEADER_START: &str = "<!-- dmx encoding ";
        const HEADER_END: &str = " -->";
        if !value.starts_with(HEADER_START) {
            return Self::read_legacy(value);
        }
        if !value.ends_with(HEADER_END) {
            return Err(FileHeaderError::InvalidFileHeader);
        }

        let inner_tokens = &value[HEADER_START.len()..value.len() - HEADER_END.len()];
        let tokens = inner_tokens.split_whitespace().collect::<Vec<_>>();
        if tokens.len() != 5 || tokens[2] != "format" {
            return Err(FileHeaderError::InvalidFileHeader);
        }

        let encoding = tokens[0].to_string();
        let encoding_version = tokens[1].parse::<i32>()?;
        let format = tokens[3].to_string();
        let format_version = tokens[4].parse::<i32>()?;

        Ok((Self::new(format, format_version), encoding, encoding_version))
    }

    fn read_legacy(value: String) -> Result<(Self, String, i32), FileHeaderError> {
        const HEADER_START: &str = "<!-- DMXVersion ";
        const HEADER_END: &str = " -->";
        if !value.starts_with(HEADER_START) || !value.ends_with(HEADER_END) {
            return Err(FileHeaderError::InvalidFileHeader);
        }

        let inner_tokens = &value[HEADER_START.len()..value.len() - HEADER_END.len()];
        let tokens = inner_tokens.split_whitespace().collect::<Vec<_>>();
        if tokens.len() != 1 {
            return Err(FileHeaderError::InvalidFileHeader);
        }
        let legacy_encoding = tokens[0];

        if legacy_encoding.starts_with("binary_v") {
            return Ok((
                Self {
                    format: String::from(CURRENT_ENCODING),
                    format_version: CURRENT_FORMAT_VERSION,
                },
                String::from("binary"),
                1,
            ));
        }

        if legacy_encoding.starts_with("sfm_v") {
            return Ok((
                Self {
                    format: String::from(legacy_encoding),
                    format_version: 1,
                },
                String::from("binary"),
                1,
            ));
        }

        if legacy_encoding.starts_with("keyvalues2_v") {
            return Ok((
                Self {
                    format: String::from(CURRENT_ENCODING),
                    format_version: CURRENT_FORMAT_VERSION,
                },
                String::from("keyvalues2"),
                1,
            ));
        }

        if legacy_encoding.starts_with("keyvalues2_flat_v") {
            return Ok((
                Self {
                    format: String::from(CURRENT_ENCODING),
                    format_version: CURRENT_FORMAT_VERSION,
                },
                String::from("keyvalues2_flat"),
                1,
            ));
        }

        Err(FileHeaderError::InvalidFileHeader)
    }

    pub fn from_buffer(buffer: &mut impl BufRead) -> Result<(Self, String, i32), FileHeaderError> {
        let mut string_buffer = Vec::new();
        buffer.read_until(b'\n', &mut string_buffer)?;
        Self::from_string(String::from_utf8_lossy(&string_buffer).into_owned())
    }

    pub fn create_header(&self, encoding: &str, encoding_version: i32) -> String {
        format!(
            "<!-- dmx encoding {} {} format {} {} -->\n",
            encoding, encoding_version, self.format, self.format_version
        )
    }
}

#[derive(Debug, ThisError)]
pub enum SerializationError {
    #[error("Unknown Encoding")]
    UnknownEncoding,
    #[error("Header Error: {0}")]
    Header(#[from] FileHeaderError),
    #[error("Binary Serialization Error: {0}")]
    Binary(#[from] BinarySerializationError),
    #[error("KeyValues2 Serialization Error: {0}")]
    KeyValues2(#[from] KeyValues2SerializationError),
}

pub fn deserialize(buffer: &mut impl BufRead) -> Result<(Header, Element), SerializationError> {
    let (header, encoding, version) = Header::from_buffer(buffer)?;

    match encoding.as_str() {
        "binary" => Ok((header, BinarySerializer::deserialize(buffer, encoding, version)?)),
        "keyvalues2" => Ok((header, KeyValues2Serializer::deserialize(buffer, encoding, version)?)),
        "keyvalues2_flat" => Ok((header, KeyValues2FlatSerializer::deserialize(buffer, encoding, version)?)),
        _ => Err(SerializationError::UnknownEncoding),
    }
}

pub trait Serializer {
    type Error;

    fn name() -> &'static str;
    fn version() -> i32;
    fn serialize_version(buffer: &mut impl Write, header: &Header, root: &Element, version: i32) -> Result<(), Self::Error>;
    fn serialize(buffer: &mut impl Write, header: &Header, root: &Element) -> Result<(), Self::Error> {
        Self::serialize_version(buffer, header, root, Self::version())
    }
    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error>;
}
