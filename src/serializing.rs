mod binary;

use std::{
    cell::RefCell,
    fmt::{self, Display, Formatter},
    fs::{write, File},
    io::{BufRead, BufReader, Error, Seek},
    num::ParseIntError,
    path::Path,
    rc::Rc,
};

use regex::Regex;
use thiserror::Error as ThisError;

use crate::Element;

use binary::BinarySerializer;

#[derive(Debug, Clone)]
pub enum SerializationFormat {
    BinaryV1,
    BinaryV2,
    BinaryV3,
    BinaryV4,
    BinaryV5,
}

#[derive(Debug, ThisError)]
pub enum SerializationError {
    #[error("The Header Is Invalid")]
    InvalidHeader,
    #[error("Header Has An Unknown Encoding Type")]
    UnknownEncoding,
    #[error("Failed To Parse Integer")]
    ParseIntegerError(#[from] ParseIntError),
    #[error("Invalid Index To String Table")]
    InvalidStringIndex,
    #[error("Not Enough Bytes To Read From")]
    ByteExhaustion,
    #[error("Wrong Deserializer Used")]
    WrongDeserializer,
    #[error("Unknown Attribute Type")]
    InvalidAttributeType,
    #[error("Element Reference Is Missing")]
    MissingElement,
    #[error("Failed To Read File")]
    ReadFileError(#[from] Error),
    #[error("Reach The End Of The File Unexpectedly")]
    UnexpectedEndOfFile,
    #[error("Wrong Version For Data Type")]
    InvalidAttributeForVersion,
    #[error("Invalid UUID")]
    InvalidUUID,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub encoding: SerializationFormat,
    format: String,
    pub version: i32,
}

impl Header {
    pub fn new<F: Into<String>>(encoding: SerializationFormat, format: F, version: i32) -> Self {
        let mut format = format.into();
        format.truncate(64);
        Self { encoding, format, version }
    }

    pub fn from_string(value: String) -> Result<Self, SerializationError> {
        let header_match =
            Regex::new(r"<!-- dmx encoding (?P<encoding>(\S+)) (?P<encoding_version>(\d+)) format (?P<format>(\S+)) (?P<format_version>(\d+)) -->").unwrap();

        match header_match.captures(&value) {
            Some(captures) => {
                let encoding_version = captures["encoding_version"].parse::<i32>()?;
                let format = &captures["format"];
                let format_version = captures["format_version"].parse::<i32>()?;

                match &captures["encoding"] {
                    "binary" => {
                        let encoding = match encoding_version {
                            1 => SerializationFormat::BinaryV1,
                            2 => SerializationFormat::BinaryV2,
                            3 => SerializationFormat::BinaryV3,
                            4 => SerializationFormat::BinaryV4,
                            5 => SerializationFormat::BinaryV5,
                            _ => return Err(SerializationError::UnknownEncoding),
                        };

                        Ok(Self::new(encoding, format, format_version))
                    }
                    _ => return Err(SerializationError::UnknownEncoding),
                }
            }
            None => {
                let legacy_match = Regex::new(r"<!-- DMXVersion (?P<encoding>(\S+)) -->").unwrap();

                match legacy_match.captures(&value) {
                    Some(captures) => {
                        let encoding = match &captures["encoding"] {
                            "binary_v1" => SerializationFormat::BinaryV1,
                            "binary_v2" => SerializationFormat::BinaryV2,
                            // "keyvalues2_v1" =>
                            // "keyvalues2_flat_v1"
                            // "xml"
                            // "xml_flat"
                            // TODO: Add any more legacy formats.
                            _ => return Err(SerializationError::UnknownEncoding),
                        };

                        Ok(Self::new(encoding, "dmx", 1))
                    }
                    None => return Err(SerializationError::InvalidHeader),
                }
            }
        }
    }

    pub fn from_buffer(data: &mut BufReader<File>) -> Result<Self, SerializationError> {
        let mut string_buffer = Vec::new();
        let _ = data.read_until(b'\n', &mut string_buffer)?;
        let header = Self::from_string(String::from_utf8_lossy(&string_buffer).into_owned())?;
        data.rewind()?;
        Ok(header)
    }

    pub fn encoding_version(&self) -> i32 {
        match self.encoding {
            SerializationFormat::BinaryV1 => 1,
            SerializationFormat::BinaryV2 => 2,
            SerializationFormat::BinaryV3 => 3,
            SerializationFormat::BinaryV4 => 4,
            SerializationFormat::BinaryV5 => 5,
        }
    }

    pub fn encoding_string(&self) -> &'static str {
        match self.encoding {
            SerializationFormat::BinaryV1 => "binary",
            SerializationFormat::BinaryV2 => "binary",
            SerializationFormat::BinaryV3 => "binary",
            SerializationFormat::BinaryV4 => "binary",
            SerializationFormat::BinaryV5 => "binary",
        }
    }

    pub fn get_format(&self) -> &str {
        &self.format
    }

    pub fn set_format<F: Into<String>>(&mut self, format: F) {
        let mut format = format.into();
        format.truncate(64);
        self.format = format
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "<!-- dmx encoding {} {} format {} {} -->",
            self.encoding_string(),
            self.encoding_version(),
            self.format,
            self.version,
        )
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            encoding: SerializationFormat::BinaryV1,
            format: String::from("dmx"),
            version: 1,
        }
    }
}

pub trait Serializer {
    fn serialize(root: Rc<RefCell<Element>>, header: &Header) -> Result<Vec<u8>, SerializationError>;
    fn deserialize(data: BufReader<File>) -> Result<Rc<RefCell<Element>>, SerializationError>;
}

pub fn deserialize<P: AsRef<Path>>(path: P) -> Result<(Rc<RefCell<Element>>, Header), SerializationError> {
    let file = File::open(path)?;
    let mut buffer = BufReader::new(file);

    let header = Header::from_buffer(&mut buffer)?;

    match header.encoding {
        SerializationFormat::BinaryV1
        | SerializationFormat::BinaryV2
        | SerializationFormat::BinaryV3
        | SerializationFormat::BinaryV4
        | SerializationFormat::BinaryV5 => Ok((BinarySerializer::deserialize(buffer)?, header)),
    }
}

pub fn serialize<P: AsRef<Path>>(path: P, root: Rc<RefCell<Element>>, header: &Header) -> Result<(), SerializationError> {
    let data = match header.encoding {
        SerializationFormat::BinaryV1
        | SerializationFormat::BinaryV2
        | SerializationFormat::BinaryV3
        | SerializationFormat::BinaryV4
        | SerializationFormat::BinaryV5 => BinarySerializer::serialize(root, header)?,
    };

    Ok(write(path, data)?)
}
