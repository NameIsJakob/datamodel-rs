use crate::DmElement;
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
    const LEGACY_VERSION_STARTING_TOKEN: &str = "<!-- DMXVersion";
    const VERSION_STARTING_TOKEN: &str = "<!-- dmx encoding";
    const VERSION_ENDING_TOKEN: &str = "-->";
    const MAX_FORMAT_LENGTH: u8 = 64;
    const MAX_HEADER_LENGTH: u8 = 40 + 2 * DmHeader::MAX_FORMAT_LENGTH;

    fn from_bytes(data: &[u8]) -> Result<Self, SerializingError> {
        for (index, &value) in data.iter().enumerate() {
            if index > Self::MAX_HEADER_LENGTH as usize {
                return Err(SerializingError::new("Exceeded iteration limit without finding header!"));
            }

            if value == b'\n' {
                return Self::from_string(from_utf8(&data[0..index]).unwrap());
            }
        }

        Err(SerializingError::new("Unexpected end of file!"))
    }

    fn from_string(data: &str) -> Result<Self, SerializingError> {
        if data.starts_with(Self::LEGACY_VERSION_STARTING_TOKEN) {
            let trimmed = data.trim();

            if !trimmed.ends_with(Self::VERSION_ENDING_TOKEN) {
                return Err(SerializingError::new("String does not match the required format!"));
            }

            let header = trimmed[Self::LEGACY_VERSION_STARTING_TOKEN.len()..trimmed.len() - Self::VERSION_ENDING_TOKEN.len()].trim();

            let version_start = match header.rfind('v') {
                Some(index) => index + 1,
                None => return Err(SerializingError::new("String does not match the required format!")),
            };

            let version = header[version_start..].trim().parse::<i32>().map_err(SerializingError::from)?;

            let format = &header[..version_start];

            match format {
                "binary_v" | "sfm_v" => {
                    return Ok(Self {
                        encoding_name: "binary".to_string(),
                        encoding_version: version,
                        format_name: "dmx".to_string(),
                        format_version: -1,
                    })
                }
                _ => return Err(SerializingError::new("Unsupported format type!")),
            }
        }

        if data.starts_with(Self::VERSION_STARTING_TOKEN) {
            let trimmed = data.trim();

            if !trimmed.ends_with(Self::VERSION_ENDING_TOKEN) {
                return Err(SerializingError::new("String does not match the required format!"));
            }

            let format = trimmed[Self::VERSION_STARTING_TOKEN.len()..trimmed.len() - Self::VERSION_ENDING_TOKEN.len()].trim();

            let mut parts = format.split_whitespace();

            let encoding_name = parts.next().ok_or(SerializingError::new("Header Missing Encodeing Name!"))?.to_string();
            let encoding_version = parts
                .next()
                .ok_or(SerializingError::new("Header Missing Encodeing Version!"))?
                .parse::<i32>()
                .map_err(SerializingError::from)?;

            if parts.next().ok_or(SerializingError::new("Header missing format!"))? != "format" {
                return Err(SerializingError::new("Uknown Argument in Header!"));
            }

            let format_name = parts.next().ok_or(SerializingError::new("Header Missing Format Name!"))?.to_string();
            let format_version = parts
                .next()
                .ok_or(SerializingError::new("Header Missing Format Version!"))?
                .parse::<i32>()
                .map_err(SerializingError::from)?;

            return Ok(Self {
                encoding_name,
                encoding_version,
                format_name,
                format_version,
            });
        }

        Err(SerializingError::new("String does not match the required format!"))
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
