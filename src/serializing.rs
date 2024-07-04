use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{BufRead, BufReader, Seek},
    num::ParseIntError,
};

use regex::Regex;
use thiserror::Error as ThisError;

use crate::{
    serializers::{BinarySerializer, KeyValues2FlatSerializer, KeyValues2Serializer, XMLFlatSerializer, XMLSerializer},
    Element,
};

#[derive(Debug, ThisError)]
pub enum FileHeaderError {
    #[error("Header Had An Invalid Integer Value")]
    InvalidInteger(#[from] ParseIntError),
    #[error("Header Was Legacy With An Invalid Encoding")]
    UnknownLegacyEncoding,
    #[error("Legacy Header Has No Version")]
    NoLegacyVersion,
    #[error("The Header Was In An Invalid Format")]
    InvalidFileHeader,
}

/// A repetition of the dmx header that all file start with.
#[derive(Debug, Clone)]
pub struct Header {
    encoding: String,
    /// The version of encoding. You can get the current version of a serializer with [Serializer::version()]
    pub encoding_version: i32,
    format: String,
    /// The format of the file version.
    pub format_version: i32,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            encoding: String::from(BinarySerializer::name()),
            encoding_version: BinarySerializer::version(),
            format: String::from("dmx"),
            format_version: 18,
        }
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "<!-- dmx encoding {} {} format {} {} -->",
            self.encoding, self.encoding_version, self.format, self.format_version,
        )
    }
}

impl Header {
    pub fn new<F: Into<String>, S: Serializer>(format: F, format_version: i32) -> Self {
        Self {
            encoding: String::from(S::name()),
            encoding_version: S::version(),
            format: format.into(),
            format_version,
        }
    }

    pub fn set_encoding<S: Serializer>(&mut self) {
        self.encoding = S::name().to_string();
        self.encoding_version = S::version();
    }

    pub fn get_encoding(&self) -> &str {
        &self.encoding
    }

    pub fn set_format<S: Into<String>>(&mut self, format: S) {
        let mut format = format.into();
        format.truncate(64);
        self.format = format;
    }

    pub fn get_format(&self) -> &str {
        &self.format
    }

    pub fn from_string(value: String) -> Result<Self, FileHeaderError> {
        let header_match =
            Regex::new(r"<!-- dmx encoding (?P<encoding>(\S+)) (?P<encoding_version>(\d+)) format (?P<format>(\S+)) (?P<format_version>(\d+)) -->").unwrap();

        match header_match.captures(&value) {
            Some(captures) => {
                let encoding = captures["encoding"].to_string();
                let encoding_version = captures["encoding_version"].parse::<i32>()?;
                let format = captures["format"].to_string();
                let format_version = captures["format_version"].parse::<i32>()?;

                Ok(Self {
                    encoding,
                    encoding_version,
                    format,
                    format_version,
                })
            }
            None => {
                let legacy_match = Regex::new(r"<!-- DMXVersion (?P<encoding>\S+?)(?:_v(?P<version>\S+))? -->").unwrap();

                match legacy_match.captures(&value) {
                    Some(captures) => {
                        match &captures["encoding"] {
                            "binary" => Ok(Self {
                                encoding: String::from(BinarySerializer::name()),
                                encoding_version: captures.name("version").ok_or(FileHeaderError::NoLegacyVersion)?.as_str().parse()?,
                                format: String::from("dmx"),
                                format_version: 1,
                            }),
                            "sfm" => Ok(Self {
                                encoding: String::from(BinarySerializer::name()),
                                encoding_version: 1,
                                format: String::from("dmx"), // Should this be sfm?
                                format_version: 1,
                            }),
                            "keyvalues2" => Ok(Self {
                                encoding: String::from(KeyValues2Serializer::name()),
                                encoding_version: 1,
                                format: String::from("dmx"),
                                format_version: 1,
                            }),
                            "keyvalues2_flat" => Ok(Self {
                                encoding: String::from(KeyValues2FlatSerializer::name()),
                                encoding_version: 1,
                                format: String::from("dmx"),
                                format_version: 1,
                            }),
                            "xml" => Ok(Self {
                                encoding: String::from(XMLSerializer::name()),
                                encoding_version: 1,
                                format: String::from("dmx"),
                                format_version: 1,
                            }),
                            "xml_flat" => Ok(Self {
                                encoding: String::from(XMLFlatSerializer::name()),
                                encoding_version: 1,
                                format: String::from("dmx"),
                                format_version: 1,
                            }),
                            _ => Err(FileHeaderError::UnknownLegacyEncoding),
                        }
                    }
                    None => Err(FileHeaderError::InvalidFileHeader),
                }
            }
        }
    }

    pub fn from_buffer(data: &mut BufReader<File>) -> Result<Self, FileHeaderError> {
        let mut string_buffer = Vec::new();
        let _ = data.read_until(b'\n', &mut string_buffer);
        let header = Self::from_string(String::from_utf8_lossy(&string_buffer).into_owned())?;
        let _ = data.rewind();
        Ok(header)
    }
}

pub trait Serializer {
    type Error;

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error>;
    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error>;
    fn name() -> &'static str;
    fn version() -> i32;
}
