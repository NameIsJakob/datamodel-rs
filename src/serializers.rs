use crate::DmElement;
use regex::Regex;
use std::{
    error::Error,
    fmt,
    fs::{read, write},
    path::Path,
    str::from_utf8,
};

mod binary;
pub use binary::BinarySerializer;

/// The header of a DMX file.
/// Tells what encoding and version the file is using.
pub struct DmHeader {
    pub encoding_name: String,
    pub encoding_version: i32,
    pub format_name: String,
    pub format_version: i32,
}

impl DmHeader {
    const MAX_HEADER_LENGTH: usize = 168;

    fn from_bytes(data: &[u8]) -> Result<Self, SerializingError> {
        for (index, &value) in data.iter().enumerate() {
            if index >= Self::MAX_HEADER_LENGTH {
                return Err(SerializingError::new("Exceeded iteration limit without finding header!"));
            }

            if value == b'\n' {
                return Self::from_string(from_utf8(&data[0..index]).unwrap());
            }
        }

        Err(SerializingError::new("Unexpected end of file!"))
    }

    fn from_string(data: &str) -> Result<Self, SerializingError> {
        let header_match = Regex::new(r"<!-- dmx encoding (\w+) (\d+) format (\w+) (\d+) -->").unwrap();

        match header_match.captures(data) {
            Some(caps) => {
                let encoding_name = caps[1].to_string();
                let encoding_version = caps[2].parse::<i32>().map_err(SerializingError::from)?;
                let format_name = caps[3].to_string();
                let format_version = caps[4].parse::<i32>().map_err(SerializingError::from)?;

                Ok(Self {
                    encoding_name,
                    encoding_version,
                    format_name,
                    format_version,
                })
            }
            None => Err(SerializingError::new("String does not match the required format!")),
        }
    }
}

pub trait Serializer {
    fn serialize(root: &DmElement, header: &DmHeader) -> Result<Vec<u8>, SerializingError>;
    fn deserialize(data: Vec<u8>) -> Result<DmElement, SerializingError>;
}

#[derive(Debug)]
pub struct SerializingError {
    details: String,
}

/// Deserializes a DMX file into a DmElement.
pub fn deserialize_file<P: AsRef<Path>>(path: P) -> Result<(DmHeader, DmElement), SerializingError> {
    let data = read(path).map_err(SerializingError::from)?;

    let header = DmHeader::from_bytes(&data)?;

    match header.encoding_name.as_str() {
        "binary" => Ok((header, BinarySerializer::deserialize(data)?)),
        _ => Err(SerializingError::new("Unsupported encoding!")),
    }
}

/// Serializes a DmElement into a DMX file.
pub fn serialize_file<P: AsRef<Path>>(path: P, root: &DmElement, header: &DmHeader) -> Result<(), SerializingError> {
    let data = match header.encoding_name.as_str() {
        "binary" => BinarySerializer::serialize(root, header)?,
        _ => return Err(SerializingError::new("Unsupported encoding!")),
    };

    write(path, data).map_err(SerializingError::from)
}

impl SerializingError {
    pub fn new(msg: &str) -> Self {
        Self { details: msg.to_string() }
    }

    pub fn from<T: Error>(error: T) -> Self {
        Self::new(&error.to_string())
    }
}

impl fmt::Display for SerializingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for SerializingError {
    fn description(&self) -> &str {
        &self.details
    }
}
