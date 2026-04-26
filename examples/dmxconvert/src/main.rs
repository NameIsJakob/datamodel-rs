use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use clap::Parser;
use datamodel::{
    SerializationError, Serializer,
    serializers::{BinarySerializationError, BinarySerializer, KeyValues2FlatSerializer, KeyValues2SerializationError, KeyValues2Serializer},
};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
enum ConvertDMXError {
    #[error("File Had IO Error: {0}")]
    FileIoError(#[from] std::io::Error),
    #[error("In File Does Not Exist")]
    InFileDoesNotExist,
    #[error("In File Failed To Be Deserialized: {0}")]
    InFileSerializationError(#[from] SerializationError),
    #[error("Out File Failed To Serialize: {0}")]
    BinaryError(#[from] BinarySerializationError),
    #[error("Out File Failed To Serialize: {0}")]
    KeyValues2Error(#[from] KeyValues2SerializationError),
    #[error("Unknown Out File Encoding: {0}")]
    UnknownEncoding(String),
}

/// An application to convert a dmx file to a different encoding, encoding version, format, or format version
#[derive(Parser)]
#[command(about, version)]
struct ConvertDMXArguments {
    /// The dmx file for conversion
    in_file: PathBuf,

    /// Specify the name and where the converted dmx file will go
    #[arg(short, long)]
    out_file: Option<PathBuf>,

    /// Specify the encoding for the conversion.
    /// Valid encodings are: binary - keyvalues2 - keyvalues2_flat
    #[arg(short, long)]
    encoding: Option<String>,

    /// Specify the encoding version for the conversion.
    /// Valid version are: binary 1..5 - keyvalues2 1 - keyvalues2_flat 1
    #[arg(long)]
    encoding_version: Option<i32>,
}

fn main() {
    let arguments = ConvertDMXArguments::parse();

    match arguments.in_file.try_exists() {
        Ok(file_exists) => {
            if !file_exists {
                return eprint!("{}", ConvertDMXError::InFileDoesNotExist);
            }
        }
        Err(check_error) => {
            return eprint!("In {}", ConvertDMXError::FileIoError(check_error));
        }
    }

    let in_file = match File::open(&arguments.in_file) {
        Ok(in_file) => in_file,
        Err(file_error) => return eprint!("In {}", ConvertDMXError::FileIoError(file_error)),
    };
    let mut in_file_buffer = BufReader::new(in_file);
    let (header, root) = match datamodel::deserialize(&mut in_file_buffer) {
        Ok((header, root)) => (header, root),
        Err(serialization_error) => return eprint!("{}", ConvertDMXError::InFileSerializationError(serialization_error)),
    };

    let out_file = match File::create(arguments.out_file.unwrap_or(arguments.in_file)) {
        Ok(out_file) => out_file,
        Err(file_error) => return eprint!("Out {}", ConvertDMXError::FileIoError(file_error)),
    };
    let mut out_file_buffer = BufWriter::new(out_file);
    let out_encoding = arguments.encoding.unwrap_or(BinarySerializer::name().to_string());

    macro_rules! supported_serializer {
        ($serializer:ident, $serializer_error:ident, $out_file_buffer:expr, $out_header:expr, $root:expr, $encoding_version:expr) => {
            if let Some(encoding_version) = $encoding_version {
                if let Err(serialization_error) = $serializer::serialize_version(&mut $out_file_buffer, &$out_header, &$root, encoding_version) {
                    eprint!("{}", ConvertDMXError::$serializer_error(serialization_error));
                }
                return;
            }

            if let Err(serialization_error) = $serializer::serialize(&mut $out_file_buffer, &$out_header, &$root) {
                eprint!("{}", ConvertDMXError::$serializer_error(serialization_error));
            }
            return;
        };
    }

    if out_encoding == BinarySerializer::name() {
        supported_serializer!(BinarySerializer, BinaryError, out_file_buffer, header, root, arguments.encoding_version);
    }

    if out_encoding == KeyValues2Serializer::name() {
        supported_serializer!(KeyValues2Serializer, KeyValues2Error, out_file_buffer, header, root, arguments.encoding_version);
    }

    if out_encoding == KeyValues2FlatSerializer::name() {
        supported_serializer!(
            KeyValues2FlatSerializer,
            KeyValues2Error,
            out_file_buffer,
            header,
            root,
            arguments.encoding_version
        );
    }

    eprint!("{}", ConvertDMXError::UnknownEncoding(out_encoding));
}
