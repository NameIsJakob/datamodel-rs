use std::{
    io::{BufRead, Error, Write},
    num::ParseIntError,
};

use thiserror::Error as ThisError;

use crate::{
    element::Element,
    serializers::{BinarySerializationError, BinarySerializer, KeyValues2FlatSerializer, KeyValues2SerializationError, KeyValues2Serializer},
};

/// An error returned by [Header] when parsing a header.
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
const CURRENT_FORMAT_VERSION: i32 = 22;

/// The data stored in the header data of the DMX file.
///
/// The header stores what the data represents in the file.
///
/// The header must be at the beginning of the file.
#[derive(Debug, Clone)]
pub struct Header {
    /// The identifier of what the file data represents for example "model" or "sfm".
    pub format: String,
    /// The numerical valve of the version that the file is representing.
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
    /// A way to create a new [Header] with specified format identifier and version.
    pub fn new(format: impl Into<String>, format_version: i32) -> Self {
        let format = format.into();
        Self { format, format_version }
    }

    /// Parses a [Header] from a string.
    ///
    /// # Returns
    /// The [Header], encoding string, and encoding version that was parsed.
    pub fn from_string(value: String) -> Result<(Self, String, i32), FileHeaderError> {
        let trimmed_header = value.trim();
        const HEADER_START: &str = "<!-- dmx encoding ";
        const HEADER_END: &str = " -->";
        if !trimmed_header.starts_with(HEADER_START) {
            return Self::read_legacy(value);
        }
        if !trimmed_header.ends_with(HEADER_END) {
            return Err(FileHeaderError::InvalidFileHeader);
        }

        let inner_tokens = &trimmed_header[HEADER_START.len()..trimmed_header.len() - HEADER_END.len()];
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
        let trimmed_header = value.trim();
        const HEADER_START: &str = "<!-- DMXVersion ";
        const HEADER_END: &str = " -->";
        if !trimmed_header.starts_with(HEADER_START) || !trimmed_header.ends_with(HEADER_END) {
            return Err(FileHeaderError::InvalidFileHeader);
        }

        let inner_tokens = &trimmed_header[HEADER_START.len()..trimmed_header.len() - HEADER_END.len()];
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

        Err(FileHeaderError::UnknownLegacyEncoding(legacy_encoding.to_string()))
    }

    /// Parses a [Header] from a buffer.
    ///
    /// # Returns
    /// The [Header], encoding string, and encoding version that was parsed.
    pub fn from_buffer(buffer: &mut impl BufRead) -> Result<(Self, String, i32), FileHeaderError> {
        let mut string_buffer = Vec::new();
        buffer.read_until(b'\n', &mut string_buffer)?;
        Self::from_string(String::from_utf8_lossy(&string_buffer).into_owned())
    }

    /// Creates a proper DMX file header.
    ///
    /// # Example
    /// ```text
    /// <!-- dmx encoding {encoding} {encoding_version} format {format} {format_version} -->
    /// ```
    pub fn create_header(&self, encoding: &str, encoding_version: i32) -> String {
        format!(
            "<!-- dmx encoding {} {} format {} {} -->\n",
            encoding, encoding_version, self.format, self.format_version
        )
    }
}

/// An error returned by [deserialize].
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

/// Deserialize a buffer with Valve Serializers.
///
/// The serializer and version is selected from the file header at the start of the buffer.
///
/// Supports legacy headers.
///
/// # Returns
/// The parsed [Header] and the root [Element] from the buffer.
///
/// # Supported Encodings
/// - `binary` with [BinarySerializer]
/// - `keyvalues2` with [KeyValues2Serializer]
/// - `keyvalues2_flat` with [KeyValues2FlatSerializer]
pub fn deserialize(buffer: &mut impl BufRead) -> Result<(Header, Element), SerializationError> {
    let (header, encoding, version) = Header::from_buffer(buffer)?;

    match encoding.as_str() {
        "binary" => Ok((header, BinarySerializer::deserialize(buffer, encoding, version)?)),
        "keyvalues2" => Ok((header, KeyValues2Serializer::deserialize(buffer, encoding, version)?)),
        "keyvalues2_flat" => Ok((header, KeyValues2FlatSerializer::deserialize(buffer, encoding, version)?)),
        _ => Err(SerializationError::UnknownEncoding),
    }
}

/// The trait allows for serialize and deserialize of a buffer for a root element from an encoding.
pub trait Serializer {
    /// The error type that serialize_version and deserialize might return.
    type Error;

    /// The name of the encoding that will be put in the header of the file.
    fn name() -> &'static str;
    /// The current version of the encoding.
    fn version() -> i32;
    /// Encodes a root element to a buffer with a selected version.
    ///
    /// The implementation must check the passed in version if its valid.
    fn serialize_version(buffer: &mut impl Write, header: &Header, root: &Element, version: i32) -> Result<(), Self::Error>;
    /// Encodes a root element to a buffer with the current version of the encoding.
    fn serialize(buffer: &mut impl Write, header: &Header, root: &Element) -> Result<(), Self::Error> {
        Self::serialize_version(buffer, header, root, Self::version())
    }
    /// Decodes the buffer for the root element.
    ///
    /// The implementation must check the passed in encoding and version are valid and must handle the file header that might exist.
    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error>;
}
