use std::{
    io::{BufRead, Error, Write},
    num::ParseIntError,
};

use regex::Regex;
use thiserror::Error as ThisError;

use crate::{
    Element,
    serializers::{BinarySerializationError, BinarySerializer, KeyValues2FlatSerializer, KeyValues2Serializer, Keyvalues2SerializationError},
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
    UnknownLegacyEncoding,
    #[error("Legacy Header Had No Version")]
    NoLegacyVersion,
}

const CURRENT_ENCODING: &str = "dmx";
const CURRENT_FORMAT_VERSION: i32 = 18;

/// The header struct represents the header of a DMX file.
#[derive(Debug, Clone)]
pub struct Header {
    format: String,
    /// The version of the format.
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
    /// Creates a new header with the given format and format version.
    /// The format is truncated to 64 characters.
    pub fn new(format: impl Into<String>, format_version: i32) -> Self {
        let mut format = format.into();
        format.truncate(64);
        Self { format, format_version }
    }

    /// Sets the format of the header.
    /// The format is truncated to 64 characters.
    pub fn set_format(&mut self, format: impl Into<String>) {
        let mut format = format.into();
        format.truncate(64);
        self.format = format;
    }

    /// Returns the format of the header.
    pub fn get_format(&self) -> &str {
        &self.format
    }

    /// Returns a header from a string.
    pub fn from_string(value: String) -> Result<(Self, String, i32), FileHeaderError> {
        let header_match =
            Regex::new(r"<!-- dmx encoding (?P<encoding>(\S+)) (?P<encoding_version>(\d+)) format (?P<format>(\S+)) (?P<format_version>(\d+)) -->").unwrap();

        match header_match.captures(&value) {
            Some(captures) => {
                let encoding = captures["encoding"].to_string();
                let encoding_version = captures["encoding_version"].parse::<i32>()?;
                let format = captures["format"].to_string();
                let format_version = captures["format_version"].parse::<i32>()?;

                Ok((Self::new(format, format_version), encoding, encoding_version))
            }
            None => {
                let legacy_match = Regex::new(r"<!-- DMXVersion (?P<encoding>\S+?)(?:_v(?P<version>\S+))? -->").unwrap();

                match legacy_match.captures(&value) {
                    Some(captures) => match &captures["encoding"] {
                        "binary" => Ok((
                            Self {
                                format: String::from(CURRENT_ENCODING),
                                format_version: CURRENT_FORMAT_VERSION,
                            },
                            String::from("binary"),
                            captures.name("version").ok_or(FileHeaderError::NoLegacyVersion)?.as_str().parse()?,
                        )),
                        "sfm" => Ok((
                            Self {
                                format: String::from("sfm"),
                                format_version: 1,
                            },
                            String::from("binary"),
                            1,
                        )),
                        "keyvalues2" => Ok((
                            Self {
                                format: String::from(CURRENT_ENCODING),
                                format_version: CURRENT_FORMAT_VERSION,
                            },
                            String::from("keyvalues2"),
                            1,
                        )),
                        "keyvalues2_flat" => Ok((
                            Self {
                                format: String::from(CURRENT_ENCODING),
                                format_version: CURRENT_FORMAT_VERSION,
                            },
                            String::from("keyvalues2_flat"),
                            1,
                        )),
                        _ => Err(FileHeaderError::UnknownLegacyEncoding),
                    },
                    None => Err(FileHeaderError::InvalidFileHeader),
                }
            }
        }
    }

    /// Reads a header from a buffer.
    pub fn from_buffer(buffer: &mut impl BufRead) -> Result<(Self, String, i32), FileHeaderError> {
        let mut string_buffer = Vec::new();
        buffer.read_until(b'\n', &mut string_buffer)?;
        Self::from_string(String::from_utf8_lossy(&string_buffer).into_owned())
    }

    /// Creates a dmx header string.
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
    KeyValues2(#[from] Keyvalues2SerializationError),
}

/// Deserialize a buffer with built-in serializers.
pub fn deserialize(buffer: &mut impl BufRead) -> Result<(Header, Element), SerializationError> {
    let (header, encoding, version) = Header::from_buffer(buffer)?;

    match encoding.as_str() {
        "binary" => Ok((header, BinarySerializer::deserialize(buffer, encoding, version)?)),
        "keyvalues2" => Ok((header, KeyValues2Serializer::deserialize(buffer, encoding, version)?)),
        "keyvalues2_flat" => Ok((header, KeyValues2FlatSerializer::deserialize(buffer, encoding, version)?)),
        _ => Err(SerializationError::UnknownEncoding),
    }
}

/// A trait for serializing and deserializing elements.
pub trait Serializer {
    type Error;

    /// Returns the name of the serializer.
    fn name() -> &'static str;
    /// Returns the current version of the serializer.
    fn version() -> i32;
    fn serialize(buffer: &mut impl Write, header: &Header, root: &Element) -> Result<(), Self::Error>;
    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error>;
}
