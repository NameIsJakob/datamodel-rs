use std::{
    fmt::{self, Display, Formatter},
    fs::{read, write},
    io::ErrorKind,
    path::Path,
};

mod binary;

pub use self::binary::BinarySerializer;
use crate::elements::Element;

pub trait Serializer {
    fn serialize(root: &Element, header: &Header) -> Result<Vec<u8>, SerializationError>;
    fn deserialize(data: Vec<u8>) -> Result<Element, SerializationError>;
}

#[derive(Debug, Clone)]
pub enum SerializationFormat {
    BinaryV1,
    BinaryV2,
    BinaryV3,
    BinaryV4,
    BinaryV5,
}

#[derive(Debug)]
pub enum SerializationError {
    FileReadError,
    FileNotFound,
    FilePermissionDenied,
    InvalidHeaderLength,
    InvalidFormatLength,
    InvalidHeader,
    InvalidHeaderEncodingVersion,
    InvalidEncoding,
    UnexpectedEndOfFile,
    ByteExhaustion,
    InvalidStringIndex,
    InvalidElementIndex,
    InvalidAttributeType,
    InvalidUUID,
    WrongDeserializer,
    InvalidAttributeForVersion,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub encoding: SerializationFormat,
    format: String,
    pub version: i32,
    legacy: bool,
}

impl Header {
    pub fn new<F: Into<String>>(encoding: SerializationFormat, format: F, version: i32) -> Self {
        Self {
            encoding,
            format: format.into(),
            version,
            legacy: false,
        }
    }

    pub fn is_legacy(&self) -> bool {
        self.legacy
    }

    pub fn encoding_version(&self) -> u8 {
        match self.encoding {
            SerializationFormat::BinaryV1 => 1,
            SerializationFormat::BinaryV2 => 2,
            SerializationFormat::BinaryV3 => 3,
            SerializationFormat::BinaryV4 => 4,
            SerializationFormat::BinaryV5 => 5,
        }
    }

    pub fn encoding_string(&self) -> String {
        match self.encoding {
            SerializationFormat::BinaryV1 => "binary",
            SerializationFormat::BinaryV2 => "binary",
            SerializationFormat::BinaryV3 => "binary",
            SerializationFormat::BinaryV4 => "binary",
            SerializationFormat::BinaryV5 => "binary",
        }
        .to_string()
    }

    pub fn set_format(&mut self, format: String) {
        self.format = format;
    }

    const LEGACY_VERSION_STARTING_TOKEN: &'static str = "<!-- DMXVersion";
    const VERSION_STARTING_TOKEN: &'static str = "<!-- dmx encoding";
    const VERSION_ENDING_TOKEN: &'static str = "-->";
    const MAX_FORMAT_LENGTH: u8 = 64;
    const MAX_HEADER_LENGTH: u8 = 40 + 2 * Header::MAX_FORMAT_LENGTH;

    pub fn from_bytes(data: &[u8]) -> Result<Self, SerializationError> {
        for (index, &value) in data.iter().enumerate() {
            if index > Self::MAX_HEADER_LENGTH as usize {
                return Err(SerializationError::InvalidHeaderLength);
            }

            if value == b'\n' {
                return Self::from_string(String::from_utf8_lossy(&data[0..index]).into_owned().as_str());
            }
        }

        Err(SerializationError::UnexpectedEndOfFile)
    }

    pub fn from_string(data: &str) -> Result<Self, SerializationError> {
        if data.starts_with(Self::LEGACY_VERSION_STARTING_TOKEN) {
            let trimmed = data.trim();

            if !trimmed.ends_with(Self::VERSION_ENDING_TOKEN) {
                return Err(SerializationError::InvalidHeader);
            }

            let header = trimmed[Self::LEGACY_VERSION_STARTING_TOKEN.len()..trimmed.len() - Self::VERSION_ENDING_TOKEN.len()].trim();

            let version_start = match header.rfind('v') {
                Some(index) => index + 1,
                None => 0,
            };

            let encoding = &header[..version_start];

            match encoding {
                // sfm_vN
                "binary_v" => {
                    let version = header[version_start..].trim().parse::<i32>().map_err(|_| SerializationError::InvalidHeader)?;

                    return Ok(Self {
                        encoding: match version {
                            1 => SerializationFormat::BinaryV1,
                            2 => SerializationFormat::BinaryV2,
                            _ => return Err(SerializationError::InvalidHeaderEncodingVersion),
                        },
                        format: format!("{}{}", encoding, version),
                        version: -1,
                        legacy: true,
                    });
                }
                // keyvalues2_v1
                // keyvalues2_flat_v1
                // xml
                // xml_flat
                _ => return Err(SerializationError::InvalidEncoding),
            }
        }

        if data.starts_with(Self::VERSION_STARTING_TOKEN) {
            let trimmed = data.trim();

            if !trimmed.ends_with(Self::VERSION_ENDING_TOKEN) {
                return Err(SerializationError::InvalidHeader);
            }

            let format = trimmed[Self::VERSION_STARTING_TOKEN.len()..trimmed.len() - Self::VERSION_ENDING_TOKEN.len()].trim();

            let mut parts = format.split_whitespace();

            let encoding_name = parts.next().ok_or(SerializationError::InvalidHeader)?.to_string();
            let encoding_version = parts
                .next()
                .ok_or(SerializationError::InvalidHeader)?
                .parse::<i32>()
                .map_err(|_| SerializationError::InvalidHeader)?;

            if parts.next().ok_or(SerializationError::InvalidHeader)? != "format" {
                return Err(SerializationError::InvalidHeader);
            }

            let format_name = parts.next().ok_or(SerializationError::InvalidHeader)?.to_string();
            let format_version = parts
                .next()
                .ok_or(SerializationError::InvalidHeader)?
                .parse::<i32>()
                .map_err(|_| SerializationError::InvalidHeader)?;

            return Ok(Self {
                encoding: match encoding_name.as_str() {
                    "binary" => match encoding_version {
                        1 => SerializationFormat::BinaryV1,
                        2 => SerializationFormat::BinaryV2,
                        3 => SerializationFormat::BinaryV3,
                        4 => SerializationFormat::BinaryV4,
                        5 => SerializationFormat::BinaryV5,
                        _ => return Err(SerializationError::InvalidHeaderEncodingVersion),
                    },
                    _ => return Err(SerializationError::InvalidEncoding),
                },
                format: format_name,
                version: format_version,
                legacy: false,
            });
        }

        Err(SerializationError::InvalidHeader)
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {} {} format {} {} {}",
            Self::VERSION_STARTING_TOKEN,
            self.encoding_string(),
            self.encoding_version(),
            self.format,
            self.version,
            Self::VERSION_ENDING_TOKEN
        )
    }
}

pub fn deserialize<P: AsRef<Path>>(path: P) -> Result<(Header, Element), SerializationError> {
    let data = match read(path) {
        Ok(data) => data,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => return Err(SerializationError::FileNotFound),
            ErrorKind::PermissionDenied => return Err(SerializationError::FilePermissionDenied),
            _ => return Err(SerializationError::FileReadError),
        },
    };

    let header = Header::from_bytes(&data)?;

    match header.encoding {
        SerializationFormat::BinaryV1
        | SerializationFormat::BinaryV2
        | SerializationFormat::BinaryV3
        | SerializationFormat::BinaryV4
        | SerializationFormat::BinaryV5 => BinarySerializer::deserialize(data).map(|element| (header, element)),
    }
}

pub fn serialize<P: AsRef<Path>>(path: P, root: &Element, header: &Header) -> Result<(), SerializationError> {
    let data = match header.encoding {
        SerializationFormat::BinaryV1
        | SerializationFormat::BinaryV2
        | SerializationFormat::BinaryV3
        | SerializationFormat::BinaryV4
        | SerializationFormat::BinaryV5 => BinarySerializer::serialize(root, header)?,
    };

    write(path, data).map_err(|x| match x.kind() {
        ErrorKind::PermissionDenied => SerializationError::FilePermissionDenied,
        _ => SerializationError::FileReadError,
    })
}
