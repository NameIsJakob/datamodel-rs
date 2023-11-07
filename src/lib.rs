use regex::Regex;
use std::{fs, path::Path, str::from_utf8};

mod attribute;
mod element;
mod serializing;

use element::DmElement;
use serializing::get_serializer;

#[derive(Clone, Debug)]
pub struct DmHeader {
    pub encoding_name: String,
    pub encoding_version: i32,
    pub format_name: String,
    pub format_version: i32,
}

impl DmHeader {
    const MAX_HEADER_LENGTH: usize = 168;

    fn from_bytes(data: &[u8]) -> Result<Self, String> {
        for (index, &value) in data.iter().enumerate() {
            if index >= Self::MAX_HEADER_LENGTH {
                return Err("Exceeded iteration limit without finding header!".to_string());
            }

            if value == b'\n' {
                return Self::from_string(from_utf8(&data[0..index]).unwrap());
            }
        }

        Err("Unexpected end of file!".to_string())
    }

    fn from_string(data: &str) -> Result<Self, String> {
        let header_match = Regex::new(r"<!-- dmx encoding (\w+) (\d+) format (\w+) (\d+) -->").unwrap();

        match header_match.captures(data) {
            Some(caps) => {
                let encoding_name = caps[1].to_string();
                let encoding_version = caps[2].parse::<i32>().unwrap(); // TODO: Handle this if not i32
                let format_name = caps[3].to_string();
                let format_version = caps[4].parse::<i32>().unwrap(); // TODO: Handle this if not i32

                Ok(Self {
                    encoding_name,
                    encoding_version,
                    format_name,
                    format_version,
                })
            }
            None => Err("String does not match the required format!".to_string()),
        }
    }
}

// TODO: Give this a proper error.
pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<DmElement, String> {
    let file_data = fs::read(path).unwrap(); // TODO: Validate the file exist and handle any read errors.

    let header = DmHeader::from_bytes(&file_data)?;

    let serializer = get_serializer(&header)?;

    serializer.unserialize(file_data)
}
