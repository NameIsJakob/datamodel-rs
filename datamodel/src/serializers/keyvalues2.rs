use std::{
    fmt::Display,
    io::{BufRead, Write},
    num::{ParseFloatError, ParseIntError},
    str::SplitWhitespace,
};

use const_format::concatcp;
use indexmap::IndexMap;
use thiserror::Error as ThisError;

use crate::{
    Element, ElementClass, Header, Serializer,
    attribute::{
        Angle, Attribute, AttributeInfo, AttributeType, AttributeValue, BinaryBlock, Color, Matrix, Quaternion, Time, UUID, Vector2, Vector3, Vector4,
    },
};

/// An error returned by [KeyValues2Serializer] and [KeyValues2FlatSerializer] from serializing or deserializing.
#[derive(Debug, ThisError)]
pub enum KeyValues2SerializationError {
    #[error("Read IO Error: {0}")]
    ReadIOError(#[from] std::io::Error),
    #[error("Invalid Encoding: Expected \"{correct_encoding}\", Got \"{wrong_encoding}\"")]
    WrongEncoding { wrong_encoding: String, correct_encoding: String },
    #[error("Invalid Encoding Version: Expected {minimum_version} - {maximum_version}, Got {wrong_version}")]
    InvalidEncodingVersion {
        wrong_version: i32,
        minimum_version: i32,
        maximum_version: i32,
    },
    #[error("Attribute \"name\" In Element \"{}\" Is Not Type String", element.get_id())]
    InvalidNameAttribute { element: Element },
    #[error("Attribute \"id\" In Element \"{}\" Can't Be Type ObjectId", element.get_id())]
    InvalidIdAttribute { element: Element },
    #[error("Buffer Ended Unexpectedly")]
    UnexpectedEOB,
    #[error("Buffer Ended Unexpectedly While In Delimited String")]
    UnexpectedEOBInDelimitedString,
    #[error("Escape Character Unfinished: Line {line} Column {column}")]
    UnfinishedEscapeCharacter { line: usize, column: usize },
    #[error("Invalid Escape Character: Gotten {invalid_character} At Line {line} Column {column}")]
    InvalidEscapeCharacter { invalid_character: char, line: usize, column: usize },
    #[error("Invalid Token: Gotten {invalid_token} At Line {line} Column {column}")]
    InvalidToken { invalid_token: char, line: usize, column: usize },
    #[error("Unexpected String Token: Gotten \"{token}\" At Line {line} Column {column}")]
    UnexpectedStringToken { token: String, line: usize, column: usize },
    #[error("Unexpected Open Brace Token: Line {line} Column {column}")]
    UnexpectedOpenBraceToken { line: usize, column: usize },
    #[error("Unexpected Close Brace Token: Line {line} Column {column}")]
    UnexpectedCloseBraceToken { line: usize, column: usize },
    #[error("Unexpected Open Bracket Token: Line {line} Column {column}")]
    UnexpectedOpenBracketToken { line: usize, column: usize },
    #[error("Unexpected Close Bracket Token: Line {line} Column {column}")]
    UnexpectedCloseBracketToken { line: usize, column: usize },
    #[error("Unexpected Comma Token: Line {line} Column {column}")]
    UnexpectedCommaToken { line: usize, column: usize },
    #[error("Element Attribute \"id\" Is Not UUID: Line {line} Column {column}")]
    ElementIdNotUUID { line: usize, column: usize },
    #[error("Deserialized Element Has Duplicate Id: Duplicate ID {duplicate_id} At Line {line} Column {column}")]
    DuplicateElementId { duplicate_id: UUID, line: usize, column: usize },
    #[error("Failed To Parse UUID: {error} At Line {line} Column {column}")]
    ParseUUIDError { error: uuid::Error, line: usize, column: usize },
    #[error("Parsed Time Out Of Range: Line {line} Column {column}")]
    TimeOutOfRange { line: usize, column: usize },
    #[error("Failed To Parse Signed Integer: {error} At Line {line} Column {column}")]
    ParseI32Error { error: ParseIntError, line: usize, column: usize },
    #[error("Failed To Parse Float: {error} At Line {line} Column {column}")]
    ParseF32Error { error: ParseFloatError, line: usize, column: usize },
    #[error("Failed To Parse Unsigned Byte: {error} At Line {line} Column {column}")]
    ParseU8Error { error: ParseIntError, line: usize, column: usize },
    #[error("Failed To Parse Unsigned Long: {error} At Line {line} Column {column}")]
    ParseU64Error { error: ParseIntError, line: usize, column: usize },
    #[error("Missing \"{argument}\": Line {line} Column {column}")]
    MissingArgument { argument: &'static str, line: usize, column: usize },
}

/// Valve's KeyValues2 encoding Serializer.
///
/// Encodes the data in a ASCII text format.
///
/// Versions are between 1 and 4.
pub struct KeyValues2Serializer;
impl Serializer for KeyValues2Serializer {
    type Error = KeyValues2SerializationError;

    fn name() -> &'static str {
        "keyvalues2"
    }

    fn version() -> i32 {
        4
    }

    fn serialize_version(buffer: &mut impl Write, header: &Header, root: &Element, version: i32) -> Result<(), Self::Error> {
        if version < 1 || version > Self::version() {
            return Err(KeyValues2SerializationError::InvalidEncodingVersion {
                wrong_version: version,
                minimum_version: 1,
                maximum_version: Self::version(),
            });
        }

        let mut writer = Writer::new(buffer);
        let mut collected_elements = IndexMap::new();
        collect_elements(root.clone(), &mut collected_elements);
        writer.write(&header.create_header(Self::name(), version))?;
        write_collected_elements(&mut writer, collected_elements)?;
        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(KeyValues2SerializationError::WrongEncoding {
                wrong_encoding: encoding,
                correct_encoding: Self::name().to_owned(),
            });
        }

        if version < 1 || version > Self::version() {
            return Err(KeyValues2SerializationError::InvalidEncodingVersion {
                wrong_version: version,
                minimum_version: 1,
                maximum_version: Self::version(),
            });
        }

        let mut string_buffer = String::new();
        buffer.read_to_string(&mut string_buffer)?;
        let mut tokens = Reader::new(&string_buffer);
        let mut element_dictionary = IndexMap::new();
        let mut root = None;

        while let Some(element) = parse_element(&mut tokens, &mut element_dictionary)? {
            if element.get_class().eq("$prefix_element$") {
                continue;
            }
            if root.is_none() {
                root = Some(element);
            }
        }

        if let Some(root_element) = root {
            return Ok(root_element);
        }

        Err(KeyValues2SerializationError::UnexpectedEOB)
    }
}

/// Valve's KeyValues2 Flat encoding Serializer.
///
/// This is the same as [KeyValues2Serializer] but no elements are inlined.
///
/// Versions are between 1 and 4.
pub struct KeyValues2FlatSerializer;
impl Serializer for KeyValues2FlatSerializer {
    type Error = KeyValues2SerializationError;

    fn name() -> &'static str {
        "keyvalues2_flat"
    }

    fn version() -> i32 {
        4
    }

    fn serialize_version(buffer: &mut impl Write, header: &Header, root: &Element, version: i32) -> Result<(), Self::Error> {
        if version < 1 || version > Self::version() {
            return Err(KeyValues2SerializationError::InvalidEncodingVersion {
                wrong_version: version,
                minimum_version: 1,
                maximum_version: Self::version(),
            });
        }

        let mut writer = Writer::new(buffer);
        let mut collected_elements = IndexMap::new();
        collect_elements(root.clone(), &mut collected_elements);
        collected_elements.values_mut().for_each(|count| *count = 1);
        writer.write(&header.create_header(Self::name(), version))?;
        write_collected_elements(&mut writer, collected_elements)?;
        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(KeyValues2SerializationError::WrongEncoding {
                wrong_encoding: encoding,
                correct_encoding: Self::name().to_owned(),
            });
        }

        if version < 1 || version > Self::version() {
            return Err(KeyValues2SerializationError::InvalidEncodingVersion {
                wrong_version: version,
                minimum_version: 1,
                maximum_version: Self::version(),
            });
        }

        KeyValues2Serializer::deserialize(buffer, String::from(KeyValues2Serializer::name()), KeyValues2Serializer::version())
    }
}

const ATTRIBUTE_ELEMENT_NAME: &str = "element";
const ATTRIBUTE_INTEGER_NAME: &str = "int";
const ATTRIBUTE_FLOAT_NAME: &str = "float";
const ATTRIBUTE_BOOLEAN_NAME: &str = "bool";
const ATTRIBUTE_STRING_NAME: &str = "string";
const ATTRIBUTE_BINARY_NAME: &str = "binary";
const ATTRIBUTE_OBJECT_ID_NAME: &str = "elementid";
const ATTRIBUTE_TIME_NAME: &str = "time";
const ATTRIBUTE_COLOR_NAME: &str = "color";
const ATTRIBUTE_VECTOR2_NAME: &str = "vector2";
const ATTRIBUTE_VECTOR3_NAME: &str = "vector3";
const ATTRIBUTE_VECTOR4_NAME: &str = "vector4";
const ATTRIBUTE_ANGLE_NAME: &str = "qangle";
const ATTRIBUTE_QUATERNION_NAME: &str = "quaternion";
const ATTRIBUTE_MATRIX_NAME: &str = "matrix";
const ATTRIBUTE_ULONG_NAME: &str = "uint64";
const ATTRIBUTE_UBYTE_NAME: &str = "uint8";
const ATTRIBUTE_ARRAY_POSTFIX: &str = "_array";
const HEX_RADIX: u32 = 16;

struct Writer<B: Write> {
    buffer: B,
    tab_index: usize,
}

impl<B: Write> Writer<B> {
    fn new(buffer: B) -> Self {
        Self { buffer, tab_index: 0 }
    }

    fn write(&mut self, string: &str) -> Result<(), KeyValues2SerializationError> {
        self.buffer.write_all(string.as_bytes())?;
        Ok(())
    }

    fn write_tabs(&mut self) -> Result<(), KeyValues2SerializationError> {
        for _ in 0..self.tab_index {
            self.buffer.write_all(b"\t")?;
        }
        Ok(())
    }

    fn write_line(&mut self, line_writer: impl FnOnce(&mut B) -> Result<(), KeyValues2SerializationError>) -> Result<(), KeyValues2SerializationError> {
        self.write_tabs()?;
        line_writer(&mut self.buffer)?;
        self.buffer.write_all(b"\n")?;
        Ok(())
    }

    fn write_open_brace(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.write_tabs()?;
        self.tab_index += 1;
        self.buffer.write_all("{\n".as_bytes())?;
        Ok(())
    }

    fn write_close_brace(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.tab_index -= 1;
        self.write_tabs()?;
        self.buffer.write_all(b"}")?;
        Ok(())
    }

    fn write_open_bracket(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.write_tabs()?;
        self.tab_index += 1;
        self.buffer.write_all("[\n".as_bytes())?;
        Ok(())
    }

    fn write_close_bracket(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.tab_index -= 1;
        self.write_tabs()?;
        self.buffer.write_all(b"]")?;
        Ok(())
    }
}

fn collect_elements(root: Element, collected_elements: &mut IndexMap<Element, usize>) {
    collected_elements.insert(root.clone(), if collected_elements.is_empty() { 1 } else { 0 });

    for attribute in root.get_attributes().values() {
        match &*attribute.get_inner_value() {
            AttributeValue::Element(element) => match element {
                Some(existing_element) => {
                    if let Some(count) = collected_elements.get_mut(existing_element) {
                        *count += 1;
                        continue;
                    }
                    collect_elements(Element::clone(existing_element), collected_elements);
                }
                None => continue,
            },
            AttributeValue::ElementArray(elements) => {
                for element in elements {
                    match element.as_ref() {
                        Some(existing_element) => {
                            if let Some(count) = collected_elements.get_mut(existing_element) {
                                *count += 1;
                                continue;
                            }
                            collect_elements(Element::clone(existing_element), collected_elements);
                        }
                        None => continue,
                    }
                }
            }
            _ => {}
        }
    }
}

fn format_escape_characters(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();

    while let Some(character) = chars.next() {
        match character {
            '\\' => match chars.next() {
                Some('n') => {
                    result.push('\\');
                    result.push('n');
                }
                Some('t') => {
                    result.push('\\');
                    result.push('t');
                }
                Some('v') => {
                    result.push('\\');
                    result.push('v');
                }
                Some('b') => {
                    result.push('\\');
                    result.push('b');
                }
                Some('r') => {
                    result.push('\\');
                    result.push('r');
                }
                Some('f') => {
                    result.push('\\');
                    result.push('f');
                }
                Some('a') => {
                    result.push('\\');
                    result.push('a');
                }
                Some('\\') => {
                    result.push('\\');
                    result.push('\\');
                }
                Some('?') => {
                    result.push('\\');
                    result.push('?');
                }
                Some('\'') => {
                    result.push('\\');
                    result.push('\'');
                }
                Some('"') => {
                    result.push('\\');
                    result.push('"');
                }
                Some(escape_character) => {
                    result.push('\\');
                    result.push('\\');
                    result.push(escape_character);
                }
                None => {
                    result.push('\\');
                    result.push('\\');
                }
            },
            '\n' => {
                result.push('\\');
                result.push('n');
            }
            '\t' => {
                result.push('\\');
                result.push('t');
            }
            '\r' => {
                result.push('\\');
                result.push('r');
            }
            '?' => {
                result.push('\\');
                result.push('?');
            }
            '\'' => {
                result.push('\\');
                result.push('\'');
            }
            '"' => {
                result.push('\\');
                result.push('"');
            }
            character => result.push(character),
        }
    }

    result
}

fn write_collected_elements(writer: &mut Writer<impl Write>, collected_elements: IndexMap<Element, usize>) -> Result<(), KeyValues2SerializationError> {
    for (element, &use_count) in &collected_elements {
        if use_count == 0 {
            continue;
        }
        writer.write_line(|buffer| {
            write!(buffer, "\"{}\"", format_escape_characters(&element.get_class()))?;
            Ok(())
        })?;
        writer.write_open_brace()?;
        writer.write_tabs()?;
        write!(&mut writer.buffer, "\"id\" \"elementid\" \"{}\"", element.get_id())?;
        write_attributes(writer, element, &collected_elements)?;
        writer.write("\n")?;
        writer.write_close_brace()?;
        writer.write("\n")?;
    }
    Ok(())
}

fn attribute_type_name(attribute_type: AttributeType) -> &'static str {
    match attribute_type {
        AttributeType::Element => ATTRIBUTE_ELEMENT_NAME,
        AttributeType::Integer => ATTRIBUTE_INTEGER_NAME,
        AttributeType::Float => ATTRIBUTE_FLOAT_NAME,
        AttributeType::Boolean => ATTRIBUTE_BOOLEAN_NAME,
        AttributeType::String => ATTRIBUTE_STRING_NAME,
        AttributeType::Binary => ATTRIBUTE_BINARY_NAME,
        AttributeType::ObjectId => ATTRIBUTE_OBJECT_ID_NAME,
        AttributeType::Time => ATTRIBUTE_TIME_NAME,
        AttributeType::Color => ATTRIBUTE_COLOR_NAME,
        AttributeType::Vector2 => ATTRIBUTE_VECTOR2_NAME,
        AttributeType::Vector3 => ATTRIBUTE_VECTOR3_NAME,
        AttributeType::Vector4 => ATTRIBUTE_VECTOR4_NAME,
        AttributeType::Angle => ATTRIBUTE_ANGLE_NAME,
        AttributeType::Quaternion => ATTRIBUTE_QUATERNION_NAME,
        AttributeType::Matrix => ATTRIBUTE_MATRIX_NAME,
        AttributeType::ULong => ATTRIBUTE_ULONG_NAME,
        AttributeType::UByte => ATTRIBUTE_UBYTE_NAME,
        AttributeType::ElementArray => concatcp!(ATTRIBUTE_ELEMENT_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::IntegerArray => concatcp!(ATTRIBUTE_INTEGER_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::FloatArray => concatcp!(ATTRIBUTE_FLOAT_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::BooleanArray => concatcp!(ATTRIBUTE_BOOLEAN_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::StringArray => concatcp!(ATTRIBUTE_STRING_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::BinaryArray => concatcp!(ATTRIBUTE_BINARY_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::ObjectIdArray => concatcp!(ATTRIBUTE_OBJECT_ID_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::TimeArray => concatcp!(ATTRIBUTE_TIME_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::ColorArray => concatcp!(ATTRIBUTE_COLOR_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::Vector2Array => concatcp!(ATTRIBUTE_VECTOR2_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::Vector3Array => concatcp!(ATTRIBUTE_VECTOR3_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::Vector4Array => concatcp!(ATTRIBUTE_VECTOR4_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::AngleArray => concatcp!(ATTRIBUTE_ANGLE_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::QuaternionArray => concatcp!(ATTRIBUTE_QUATERNION_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::MatrixArray => concatcp!(ATTRIBUTE_MATRIX_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::ULongArray => concatcp!(ATTRIBUTE_ULONG_NAME, ATTRIBUTE_ARRAY_POSTFIX),
        AttributeType::UByteArray => concatcp!(ATTRIBUTE_UBYTE_NAME, ATTRIBUTE_ARRAY_POSTFIX),
    }
}

fn write_attributes(
    writer: &mut Writer<impl Write>,
    root: &Element,
    collected_elements: &IndexMap<Element, usize>,
) -> Result<(), KeyValues2SerializationError> {
    for (attribute_name, attribute) in root.get_attributes().iter() {
        let attribute_type_name = attribute_type_name(attribute.get_type());

        if attribute_name == "name" && attribute.get_type() != AttributeType::String {
            return Err(KeyValues2SerializationError::InvalidNameAttribute { element: Element::clone(root) });
        }

        if attribute_name == "id" && attribute.get_type() == AttributeType::ObjectId {
            return Err(KeyValues2SerializationError::InvalidIdAttribute { element: Element::clone(root) });
        }
        writer.write("\n")?;
        let formatted_attribute_name = format_escape_characters(attribute_name);
        match &*attribute.get_inner_value() {
            AttributeValue::Element(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\"")?;
                write_element_attribute(writer, value, collected_elements)?;
            }
            AttributeValue::Integer(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_generic_attribute(writer, value)?;
            }
            AttributeValue::Float(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_generic_attribute(writer, value)?;
            }
            AttributeValue::Boolean(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_generic_attribute(writer, &(*value as u8))?;
            }
            AttributeValue::String(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_string_attribute(writer, value)?;
            }
            AttributeValue::Binary(value) => {
                writer.write_tabs()?;
                writeln!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\"")?;
                writer.write_tabs()?;
                write_binary_attribute(writer, value)?;
            }
            AttributeValue::ObjectId(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_generic_attribute(writer, value)?;
            }
            AttributeValue::Time(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_time_attribute(writer, value)?;
            }
            AttributeValue::Color(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_color_attribute(writer, value)?;
            }
            AttributeValue::Vector2(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_vector2_attribute(writer, value)?;
            }
            AttributeValue::Vector3(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_vector3_attribute(writer, value)?;
            }
            AttributeValue::Vector4(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_vector4_attribute(writer, value)?;
            }
            AttributeValue::Angle(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_angle_attribute(writer, value)?;
            }
            AttributeValue::Quaternion(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_quaternion_attribute(writer, value)?;
            }
            AttributeValue::Matrix(value) => {
                writer.write_tabs()?;
                writeln!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\"")?;
                writer.write_tabs()?;
                write_matrix_attribute(writer, value)?;
            }
            AttributeValue::ULong(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_ulong_attribute(writer, value)?;
            }
            AttributeValue::UByte(value) => {
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"{formatted_attribute_name}\" \"{attribute_type_name}\" ")?;
                write_generic_attribute(writer, value)?;
            }
            AttributeValue::ElementArray(values) => write_element_array_attribute(writer, &formatted_attribute_name, values, collected_elements)?,
            AttributeValue::IntegerArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_generic_attribute)?;
            }
            AttributeValue::FloatArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_generic_attribute)?;
            }
            AttributeValue::BooleanArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, |writer, value| {
                    write_generic_attribute(writer, &(*value as u8))
                })?;
            }
            AttributeValue::StringArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, |writer, value| {
                    write_string_attribute(writer, value.as_str())
                })?;
            }
            AttributeValue::BinaryArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_binary_attribute)?;
            }
            AttributeValue::ObjectIdArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_generic_attribute)?;
            }
            AttributeValue::TimeArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_time_attribute)?;
            }
            AttributeValue::ColorArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_color_attribute)?;
            }
            AttributeValue::Vector2Array(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_vector2_attribute)?;
            }
            AttributeValue::Vector3Array(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_vector3_attribute)?;
            }
            AttributeValue::Vector4Array(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_vector4_attribute)?;
            }
            AttributeValue::AngleArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_angle_attribute)?;
            }
            AttributeValue::QuaternionArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_quaternion_attribute)?;
            }
            AttributeValue::MatrixArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_matrix_attribute)?;
            }
            AttributeValue::ULongArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_generic_attribute)?;
            }
            AttributeValue::UByteArray(values) => {
                write_attribute_array(writer, &formatted_attribute_name, attribute_type_name, values, write_generic_attribute)?;
            }
        };
    }
    Ok(())
}

fn write_element_attribute(
    writer: &mut Writer<impl Write>,
    attribute_value: &Option<Element>,
    collected_elements: &IndexMap<Element, usize>,
) -> Result<(), KeyValues2SerializationError> {
    if let Some(element) = attribute_value
        && let Some(&count) = collected_elements.get(element)
    {
        if count > 0 {
            write!(&mut writer.buffer, " \"{ATTRIBUTE_ELEMENT_NAME}\" \"{}\"", element.get_id())?;
            return Ok(());
        }

        writeln!(&mut writer.buffer, " \"{}\"", format_escape_characters(&element.get_class()))?;
        writer.write_open_brace()?;
        writer.write_tabs()?;
        write!(&mut writer.buffer, "\"id\" \"elementid\" \"{}\"", element.get_id())?;
        write_attributes(writer, element, collected_elements)?;
        writer.write("\n")?;
        writer.write_close_brace()?;
        return Ok(());
    }
    write!(&mut writer.buffer, " \"{ATTRIBUTE_ELEMENT_NAME}\" \"\"")?;
    Ok(())
}

fn write_generic_attribute<T: Display>(writer: &mut Writer<impl Write>, attribute_value: &T) -> Result<(), KeyValues2SerializationError> {
    write!(&mut writer.buffer, "\"{attribute_value}\"")?;
    Ok(())
}

fn write_string_attribute(writer: &mut Writer<impl Write>, attribute_value: &str) -> Result<(), KeyValues2SerializationError> {
    write!(&mut writer.buffer, "\"{}\"", format_escape_characters(attribute_value))?;
    Ok(())
}

fn write_binary_attribute(writer: &mut Writer<impl Write>, attribute_value: &BinaryBlock) -> Result<(), KeyValues2SerializationError> {
    writer.write("\"\n")?;
    writer.tab_index += 1;
    for chunk in attribute_value.0.chunks(40) {
        writer.write_tabs()?;
        for byte in chunk {
            write!(&mut writer.buffer, "{byte:02X}")?;
        }
        writer.write("\n")?;
    }
    writer.tab_index -= 1;
    writer.write_tabs()?;
    writer.write("\"")?;
    Ok(())
}

fn write_time_attribute(writer: &mut Writer<impl Write>, attribute_value: &Time) -> Result<(), KeyValues2SerializationError> {
    write!(&mut writer.buffer, "\"{:.4}\"", attribute_value.as_seconds())?;
    Ok(())
}

fn write_color_attribute(writer: &mut Writer<impl Write>, attribute_value: &Color) -> Result<(), KeyValues2SerializationError> {
    write!(
        &mut writer.buffer,
        "\"{} {} {} {}\"",
        attribute_value.red, attribute_value.green, attribute_value.blue, attribute_value.alpha
    )?;
    Ok(())
}

fn write_vector2_attribute(writer: &mut Writer<impl Write>, attribute_value: &Vector2) -> Result<(), KeyValues2SerializationError> {
    write!(&mut writer.buffer, "\"{} {}\"", attribute_value.x, attribute_value.y)?;
    Ok(())
}

fn write_vector3_attribute(writer: &mut Writer<impl Write>, attribute_value: &Vector3) -> Result<(), KeyValues2SerializationError> {
    write!(&mut writer.buffer, "\"{} {} {}\"", attribute_value.x, attribute_value.y, attribute_value.z)?;
    Ok(())
}

fn write_vector4_attribute(writer: &mut Writer<impl Write>, attribute_value: &Vector4) -> Result<(), KeyValues2SerializationError> {
    write!(
        &mut writer.buffer,
        "\"{} {} {} {}\"",
        attribute_value.x, attribute_value.y, attribute_value.z, attribute_value.w
    )?;
    Ok(())
}

fn write_angle_attribute(writer: &mut Writer<impl Write>, attribute_value: &Angle) -> Result<(), KeyValues2SerializationError> {
    write!(
        &mut writer.buffer,
        "\"{} {} {}\"",
        attribute_value.pitch, attribute_value.yaw, attribute_value.roll
    )?;
    Ok(())
}

fn write_quaternion_attribute(writer: &mut Writer<impl Write>, attribute_value: &Quaternion) -> Result<(), KeyValues2SerializationError> {
    write!(
        &mut writer.buffer,
        "\"{} {} {} {}\"",
        attribute_value.x, attribute_value.y, attribute_value.z, attribute_value.w
    )?;
    Ok(())
}

fn write_matrix_attribute(writer: &mut Writer<impl Write>, attribute_value: &Matrix) -> Result<(), KeyValues2SerializationError> {
    writer.write("\"\n")?;
    writer.tab_index += 1;
    writer.write_line(|buffer| {
        write!(
            buffer,
            "{} {} {} {}",
            attribute_value.0[0][0], attribute_value.0[0][1], attribute_value.0[0][2], attribute_value.0[0][3]
        )?;
        Ok(())
    })?;
    writer.write_line(|buffer| {
        write!(
            buffer,
            "{} {} {} {}",
            attribute_value.0[1][0], attribute_value.0[1][1], attribute_value.0[1][2], attribute_value.0[1][3]
        )?;
        Ok(())
    })?;
    writer.write_line(|buffer| {
        write!(
            buffer,
            "{} {} {} {}",
            attribute_value.0[2][0], attribute_value.0[2][1], attribute_value.0[2][2], attribute_value.0[2][3]
        )?;
        Ok(())
    })?;
    writer.write_line(|buffer| {
        write!(
            buffer,
            "{} {} {} {}",
            attribute_value.0[3][0], attribute_value.0[3][1], attribute_value.0[3][2], attribute_value.0[3][3]
        )?;
        Ok(())
    })?;
    writer.tab_index -= 1;
    writer.write_tabs()?;
    writer.write("\"")?;
    Ok(())
}

fn write_ulong_attribute(writer: &mut Writer<impl Write>, attribute_value: &u64) -> Result<(), KeyValues2SerializationError> {
    write!(&mut writer.buffer, "\"0x{attribute_value:01X}\"")?;
    Ok(())
}

fn write_element_array_attribute(
    writer: &mut Writer<impl Write>,
    attribute_name: &str,
    attribute_values: &[Option<Element>],
    collected_elements: &IndexMap<Element, usize>,
) -> Result<(), KeyValues2SerializationError> {
    writer.write_line(|buffer| {
        write!(buffer, "\"{attribute_name}\" \"{ATTRIBUTE_ELEMENT_NAME}{ATTRIBUTE_ARRAY_POSTFIX}\"")?;
        Ok(())
    })?;
    writer.write_open_bracket()?;
    if let Some((last_value, values)) = attribute_values.split_last() {
        for value in values {
            if let Some(element) = value
                && let Some(&count) = collected_elements.get(element)
            {
                writer.write_tabs()?;
                if count > 0 {
                    writeln!(&mut writer.buffer, "\"{ATTRIBUTE_ELEMENT_NAME}\" \"{}\",", element.get_id())?;
                    continue;
                }
                writeln!(&mut writer.buffer, "\"{}\"", format_escape_characters(&element.get_class()))?;
                writer.write_open_brace()?;
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"id\" \"elementid\" \"{}\"", element.get_id())?;
                write_attributes(writer, element, collected_elements)?;
                writer.write("\n")?;
                writer.write_close_brace()?;
                writeln!(&mut writer.buffer, ",")?;
                continue;
            }
            writeln!(&mut writer.buffer, "\"{ATTRIBUTE_ELEMENT_NAME}\" \"\",")?;
        }
        if let Some(element) = last_value
            && let Some(&count) = collected_elements.get(element)
        {
            writer.write_tabs()?;
            if count > 0 {
                writeln!(&mut writer.buffer, "\"{ATTRIBUTE_ELEMENT_NAME}\" \"{}\"", element.get_id())?;
            } else {
                writeln!(&mut writer.buffer, "\"{}\"", format_escape_characters(&element.get_class()))?;
                writer.write_open_brace()?;
                writer.write_tabs()?;
                write!(&mut writer.buffer, "\"id\" \"elementid\" \"{}\"", element.get_id())?;
                write_attributes(writer, element, collected_elements)?;
                writer.write("\n")?;
                writer.write_close_brace()?;
                writer.write("\n")?;
            }
        } else {
            writeln!(&mut writer.buffer, "\"{ATTRIBUTE_ELEMENT_NAME}\" \"\"")?;
        }
    }
    writer.write_close_bracket()?;
    Ok(())
}

type AttributeWriteFunction<T, B> = fn(&mut Writer<B>, &T) -> Result<(), KeyValues2SerializationError>;

fn write_attribute_array<T: AttributeInfo, B: Write>(
    writer: &mut Writer<B>,
    attribute_name: &str,
    attribute_type: &str,
    attribute_values: &[T],
    attribute_write: AttributeWriteFunction<T, B>,
) -> Result<(), KeyValues2SerializationError> {
    writer.write_line(|buffer| {
        write!(buffer, "\"{attribute_name}\" \"{attribute_type}\"")?;
        Ok(())
    })?;
    writer.write_open_bracket()?;
    if let Some((last_value, values)) = attribute_values.split_last() {
        for value in values {
            writer.write_tabs()?;
            attribute_write(writer, value)?;
            writeln!(&mut writer.buffer, ",")?;
        }
        writer.write_tabs()?;
        attribute_write(writer, last_value)?;
        writer.write("\n")?;
    }
    writer.write_close_bracket()?;
    Ok(())
}

#[derive(Debug)]
enum TokenType<'b> {
    String(&'b str),
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Comma,
}

#[derive(Debug)]
struct Token<'b> {
    line: usize,
    column: usize,
    token_type: TokenType<'b>,
}

struct Reader<'b> {
    buffer: &'b str,
    cursor: usize,
    current_line: usize,
    current_column: usize,
}

impl<'b> Reader<'b> {
    fn new(buffer: &'b str) -> Self {
        Self {
            buffer,
            cursor: 0,
            current_line: 1,
            current_column: 1,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token<'b>>, KeyValues2SerializationError> {
        let mut characters = self.buffer[self.cursor..].chars();

        loop {
            let Some(current_character) = characters.next() else {
                return Ok(None);
            };

            match current_character {
                '<' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    loop {
                        let Some(current_character) = characters.next() else {
                            return Ok(None);
                        };

                        if current_character == '>' {
                            break;
                        }

                        if current_character == '\n' {
                            self.current_line += 1;
                            self.current_column = 0;
                        }

                        self.cursor += 1;
                        self.current_column += 1;
                    }
                }
                '"' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    let start_line = self.current_line;
                    let start_column = self.current_column;
                    let start_cursor = self.cursor;
                    loop {
                        let Some(current_character) = characters.next() else {
                            return Err(KeyValues2SerializationError::UnexpectedEOBInDelimitedString);
                        };

                        if current_character == '"' {
                            self.cursor += 1;
                            self.current_column += 1;
                            return Ok(Some(Token {
                                line: start_line,
                                column: start_column,
                                token_type: TokenType::String(&self.buffer[start_cursor..self.cursor - 1]),
                            }));
                        }

                        if current_character == '\\' {
                            self.cursor += 1;
                            self.current_column += 1;
                            let Some(current_character) = characters.next() else {
                                return Err(KeyValues2SerializationError::UnexpectedEOBInDelimitedString);
                            };
                            match current_character {
                                'n' | 't' | 'v' | 'b' | 'r' | 'f' | 'a' | '\\' | '?' | '\'' | '"' => {}
                                character if character.is_whitespace() => {
                                    return Err(KeyValues2SerializationError::UnfinishedEscapeCharacter {
                                        line: self.current_line,
                                        column: self.current_column,
                                    });
                                }
                                character => {
                                    return Err(KeyValues2SerializationError::InvalidEscapeCharacter {
                                        invalid_character: character,
                                        line: self.current_line,
                                        column: self.current_column,
                                    });
                                }
                            }
                        }

                        if current_character == '\n' {
                            self.current_line += 1;
                            self.current_column = 0;
                        }

                        self.cursor += 1;
                        self.current_column += 1;
                    }
                }
                '{' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    return Ok(Some(Token {
                        line: self.current_line,
                        column: self.current_column - 1,
                        token_type: TokenType::OpenBrace,
                    }));
                }
                '}' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    return Ok(Some(Token {
                        line: self.current_line,
                        column: self.current_column - 1,
                        token_type: TokenType::CloseBrace,
                    }));
                }
                '[' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    return Ok(Some(Token {
                        line: self.current_line,
                        column: self.current_column - 1,
                        token_type: TokenType::OpenBracket,
                    }));
                }
                ']' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    return Ok(Some(Token {
                        line: self.current_line,
                        column: self.current_column - 1,
                        token_type: TokenType::CloseBracket,
                    }));
                }
                ',' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    return Ok(Some(Token {
                        line: self.current_line,
                        column: self.current_column - 1,
                        token_type: TokenType::Comma,
                    }));
                }
                '/' => {
                    self.cursor += 1;
                    self.current_column += 1;
                    loop {
                        let Some(current_character) = characters.next() else {
                            return Ok(None);
                        };

                        if current_character == '\n' {
                            self.current_line += 1;
                            self.current_column = 0;
                            break;
                        }

                        self.cursor += 1;
                        self.current_column += 1;
                    }
                }
                '\n' => {
                    self.current_line += 1;
                    self.current_column = 0;
                }
                character if character.is_whitespace() => {}
                character => {
                    return Err(KeyValues2SerializationError::InvalidToken {
                        invalid_token: character,
                        line: self.current_line,
                        column: self.current_column,
                    });
                }
            }

            self.cursor += 1;
            self.current_column += 1;
        }
    }
}

enum ElementEntry {
    Defined(Element),
    Reference(Element),
}

impl ElementEntry {
    fn element(&self) -> Element {
        match self {
            Self::Defined(element) => Element::clone(element),
            Self::Reference(element) => Element::clone(element),
        }
    }
}

type ElementDictionary = IndexMap<UUID, ElementEntry>;

fn unexpected_token_error(unexpected_token: Token<'_>) -> KeyValues2SerializationError {
    let line = unexpected_token.line;
    let column = unexpected_token.column;
    match unexpected_token.token_type {
        TokenType::String(token) => KeyValues2SerializationError::UnexpectedStringToken {
            token: token.to_owned(),
            line,
            column,
        },
        TokenType::OpenBrace => KeyValues2SerializationError::UnexpectedOpenBraceToken { line, column },
        TokenType::CloseBrace => KeyValues2SerializationError::UnexpectedCloseBraceToken { line, column },
        TokenType::OpenBracket => KeyValues2SerializationError::UnexpectedOpenBracketToken { line, column },
        TokenType::CloseBracket => KeyValues2SerializationError::UnexpectedCloseBracketToken { line, column },
        TokenType::Comma => KeyValues2SerializationError::UnexpectedCommaToken { line, column },
    }
}

fn parse_element(tokens: &mut Reader<'_>, element_dictionary: &mut ElementDictionary) -> Result<Option<Element>, KeyValues2SerializationError> {
    let element_class = match tokens.next_token()? {
        Some(element_class_token) => match element_class_token.token_type {
            TokenType::String(element_class) => element_class,
            _ => return Err(unexpected_token_error(element_class_token)),
        },
        None => return Ok(None),
    };

    let element_start_token = tokens.next_token()?;
    if !matches!(element_start_token.map(|token| token.token_type), Some(TokenType::OpenBrace)) {
        return Err(KeyValues2SerializationError::UnexpectedEOB);
    }

    Ok(Some(parse_element_inline(tokens, element_class, element_dictionary)?))
}

fn parse_element_inline(tokens: &mut Reader<'_>, class: &str, element_dictionary: &mut ElementDictionary) -> Result<Element, KeyValues2SerializationError> {
    let mut element_attributes = IndexMap::new();
    let attribute_line_start = tokens.current_line;
    let attribute_column_start = tokens.current_column;

    loop {
        let attribute_name = match tokens.next_token()? {
            Some(attribute_name_token) => match attribute_name_token.token_type {
                TokenType::String(attribute_name) => attribute_name,
                TokenType::CloseBrace => break,
                _ => return Err(unexpected_token_error(attribute_name_token)),
            },
            None => return Err(KeyValues2SerializationError::UnexpectedEOB),
        };

        let attribute_type = match tokens.next_token()? {
            Some(attribute_type_token) => match attribute_type_token.token_type {
                TokenType::String(attribute_type) => attribute_type,
                _ => return Err(unexpected_token_error(attribute_type_token)),
            },
            None => return Err(KeyValues2SerializationError::UnexpectedEOB),
        };

        let attribute = parse_attribute(tokens, attribute_type, element_dictionary)?;
        if element_attributes.insert(attribute_name.to_owned(), attribute).is_some() {
            return Err(KeyValues2SerializationError::UnexpectedEOB);
        }
    }

    let element_id = match element_attributes.shift_remove("id") {
        Some(id_attribute) => match *id_attribute.get_inner_value() {
            AttributeValue::ObjectId(uuid) => uuid,
            _ => {
                return Err(KeyValues2SerializationError::ElementIdNotUUID {
                    line: attribute_line_start,
                    column: attribute_column_start,
                });
            }
        },
        None => UUID::new_v4(),
    };

    let element = match element_dictionary.entry(element_id) {
        indexmap::map::Entry::Occupied(mut occupied_element) => {
            let value = occupied_element.get_mut();
            match value {
                ElementEntry::Defined(_) => {
                    return Err(KeyValues2SerializationError::DuplicateElementId {
                        duplicate_id: element_id,
                        line: attribute_line_start,
                        column: attribute_column_start,
                    });
                }
                ElementEntry::Reference(element) => {
                    let mut element = Element::clone(element);
                    element.set_class_name(class);
                    element.set_attributes(element_attributes);
                    *value = ElementEntry::Defined(Element::clone(&element));
                    element
                }
            }
        }
        indexmap::map::Entry::Vacant(vacant_element) => {
            let mut new_element = Element::full(class, element_id);
            new_element.set_attributes(element_attributes);
            vacant_element.insert(ElementEntry::Defined(Element::clone(&new_element)));
            new_element
        }
    };
    Ok(element)
}

fn parse_attribute(
    tokens: &mut Reader<'_>,
    attribute_type_name: &str,
    element_dictionary: &mut ElementDictionary,
) -> Result<Attribute, KeyValues2SerializationError> {
    let (attribute_type, is_array) = match attribute_type_name.strip_suffix(ATTRIBUTE_ARRAY_POSTFIX) {
        Some(attribute_base) => (attribute_base, true),
        None => (attribute_type_name, false),
    };

    if attribute_type == ATTRIBUTE_ELEMENT_NAME {
        if !is_array {
            let element = match tokens.next_token()? {
                Some(element_token) => match element_token.token_type {
                    TokenType::String(reference_id_value) => {
                        parse_element_reference(reference_id_value, element_dictionary, element_token.line, element_token.column)?
                    }
                    TokenType::OpenBrace => Some(parse_element_inline(tokens, attribute_type_name, element_dictionary)?),
                    _ => return Err(unexpected_token_error(element_token)),
                },
                None => return Err(KeyValues2SerializationError::UnexpectedEOB),
            };
            return Ok(Attribute::new(AttributeValue::Element(element)));
        }

        match tokens.next_token()? {
            Some(attribute_list_start_token) => match attribute_list_start_token.token_type {
                TokenType::OpenBracket => {}
                _ => return Err(unexpected_token_error(attribute_list_start_token)),
            },
            None => return Err(KeyValues2SerializationError::UnexpectedEOB),
        }

        let mut elements = Vec::new();
        loop {
            let element_class = match tokens.next_token()? {
                Some(element_class_token) => match element_class_token.token_type {
                    TokenType::String(element_class) => element_class,
                    TokenType::CloseBracket => break,
                    TokenType::Comma => continue,
                    _ => return Err(unexpected_token_error(element_class_token)),
                },
                None => return Err(KeyValues2SerializationError::UnexpectedEOB),
            };

            let element = match tokens.next_token()? {
                Some(element_token) => match element_token.token_type {
                    TokenType::String(reference_id_value) => {
                        parse_element_reference(reference_id_value, element_dictionary, element_token.line, element_token.column)?
                    }
                    TokenType::OpenBrace => Some(parse_element_inline(tokens, element_class, element_dictionary)?),
                    _ => return Err(unexpected_token_error(element_token)),
                },
                None => return Err(KeyValues2SerializationError::UnexpectedEOB),
            };
            elements.push(element);
        }
        return Ok(Attribute::new(AttributeValue::ElementArray(elements)));
    }

    Ok(Attribute::new(match attribute_type {
        ATTRIBUTE_INTEGER_NAME => parse_attribute_value(tokens, is_array, parse_integer_attribute)?,
        ATTRIBUTE_FLOAT_NAME => parse_attribute_value(tokens, is_array, parse_float_attribute)?,
        ATTRIBUTE_BOOLEAN_NAME => parse_attribute_value(tokens, is_array, parse_boolean_attribute)?,
        ATTRIBUTE_STRING_NAME => parse_attribute_value(tokens, is_array, parse_string_attribute)?,
        ATTRIBUTE_BINARY_NAME => parse_attribute_value(tokens, is_array, parse_binary_attribute)?,
        ATTRIBUTE_OBJECT_ID_NAME => parse_attribute_value(tokens, is_array, parse_object_id_attribute)?,
        ATTRIBUTE_TIME_NAME => parse_attribute_value(tokens, is_array, parse_time_attribute)?,
        ATTRIBUTE_COLOR_NAME => parse_attribute_value(tokens, is_array, parse_color_attribute)?,
        ATTRIBUTE_VECTOR2_NAME => parse_attribute_value(tokens, is_array, parse_vector2_attribute)?,
        ATTRIBUTE_VECTOR3_NAME => parse_attribute_value(tokens, is_array, parse_vector3_attribute)?,
        ATTRIBUTE_VECTOR4_NAME => parse_attribute_value(tokens, is_array, parse_vector4_attribute)?,
        ATTRIBUTE_ANGLE_NAME => parse_attribute_value(tokens, is_array, parse_angle_attribute)?,
        ATTRIBUTE_QUATERNION_NAME => parse_attribute_value(tokens, is_array, parse_quaternion_attribute)?,
        ATTRIBUTE_MATRIX_NAME => parse_attribute_value(tokens, is_array, parse_matrix_attribute)?,
        ATTRIBUTE_ULONG_NAME => parse_attribute_value(tokens, is_array, parse_ulong_attribute)?,
        ATTRIBUTE_UBYTE_NAME => parse_attribute_value(tokens, is_array, parse_ubyte_attribute)?,
        _ => {
            let element = match tokens.next_token()? {
                Some(element_token) => match element_token.token_type {
                    TokenType::OpenBrace => parse_element_inline(tokens, attribute_type_name, element_dictionary)?,
                    _ => return Err(unexpected_token_error(element_token)),
                },
                None => return Err(KeyValues2SerializationError::UnexpectedEOB),
            };
            AttributeValue::Element(Some(element))
        }
    }))
}

fn parse_element_reference(
    reference_id_value: &str,
    element_dictionary: &mut ElementDictionary,
    line: usize,
    column: usize,
) -> Result<Option<Element>, KeyValues2SerializationError> {
    if reference_id_value.is_empty() {
        return Ok(None);
    }
    let element_id = reference_id_value
        .parse::<UUID>()
        .map_err(|error| KeyValues2SerializationError::ParseUUIDError { error, line, column })?;
    let element_entry = element_dictionary
        .entry(element_id)
        .or_insert_with(|| ElementEntry::Reference(Element::full(Element::class_name(), element_id)));
    Ok(Some(element_entry.element()))
}

type AttributeParseFunction<T> = fn(&str, usize, usize) -> Result<T, KeyValues2SerializationError>;

fn parse_attribute_value<T>(tokens: &mut Reader<'_>, is_array: bool, parse: AttributeParseFunction<T>) -> Result<AttributeValue, KeyValues2SerializationError>
where
    T: AttributeInfo,
    Vec<T>: AttributeInfo,
{
    if is_array {
        match tokens.next_token()? {
            Some(attribute_list_start_token) => match attribute_list_start_token.token_type {
                TokenType::OpenBracket => {}
                _ => return Err(unexpected_token_error(attribute_list_start_token)),
            },
            None => return Err(KeyValues2SerializationError::UnexpectedEOB),
        }

        let mut values = Vec::new();
        loop {
            let (attribute_value, attribute_line, attribute_column) = match tokens.next_token()? {
                Some(attribute_value_token) => match attribute_value_token.token_type {
                    TokenType::String(attribute_value) => (attribute_value, attribute_value_token.line, attribute_value_token.column),
                    TokenType::CloseBracket => break,
                    TokenType::Comma => continue,
                    _ => return Err(unexpected_token_error(attribute_value_token)),
                },
                None => return Err(KeyValues2SerializationError::UnexpectedEOB),
            };
            let value = parse(attribute_value, attribute_line, attribute_column)?;
            values.push(value);
        }
        return Ok(values.into_attribute_type());
    }

    let (attribute_value, attribute_line, attribute_column) = match tokens.next_token()? {
        Some(attribute_value_token) => match attribute_value_token.token_type {
            TokenType::String(attribute_value) => (attribute_value, attribute_value_token.line, attribute_value_token.column),
            _ => return Err(unexpected_token_error(attribute_value_token)),
        },
        None => return Err(KeyValues2SerializationError::UnexpectedEOB),
    };
    Ok(parse(attribute_value, attribute_line, attribute_column)?.into_attribute_type())
}

fn parse_integer_attribute(attribute_value: &str, line: usize, column: usize) -> Result<i32, KeyValues2SerializationError> {
    attribute_value
        .parse()
        .map_err(|error| KeyValues2SerializationError::ParseI32Error { error, line, column })
}

fn parse_float_attribute(attribute_value: &str, line: usize, column: usize) -> Result<f32, KeyValues2SerializationError> {
    attribute_value
        .parse()
        .map_err(|error| KeyValues2SerializationError::ParseF32Error { error, line, column })
}

fn parse_boolean_attribute(attribute_value: &str, line: usize, column: usize) -> Result<bool, KeyValues2SerializationError> {
    Ok(attribute_value
        .parse::<u8>()
        .map_err(|error| KeyValues2SerializationError::ParseU8Error { error, line, column })?
        != 0)
}

fn parse_string_attribute(attribute_value: &str, _line: usize, _column: usize) -> Result<String, KeyValues2SerializationError> {
    Ok(attribute_value.to_owned())
}

fn parse_binary_attribute(attribute_value: &str, _line: usize, _column: usize) -> Result<BinaryBlock, KeyValues2SerializationError> {
    let binary_byte_count = attribute_value
        .chars()
        .fold(0, |count, character| count + character.is_ascii_hexdigit() as usize)
        / 2;
    let mut block = Vec::with_capacity(binary_byte_count);
    let mut hex = attribute_value.chars().filter(|character| character.is_ascii_hexdigit());
    while let (Some(high), Some(low)) = (hex.next(), hex.next()) {
        let high = high.to_digit(HEX_RADIX).unwrap_or_default() as u8;
        let low = low.to_digit(HEX_RADIX).unwrap_or_default() as u8;
        block.push(high << 4 | low);
    }
    Ok(BinaryBlock(block))
}

fn parse_object_id_attribute(attribute_value: &str, line: usize, column: usize) -> Result<UUID, KeyValues2SerializationError> {
    attribute_value
        .parse()
        .map_err(|error| KeyValues2SerializationError::ParseUUIDError { error, line, column })
}

fn parse_time_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Time, KeyValues2SerializationError> {
    Time::from_seconds(
        attribute_value
            .parse()
            .map_err(|error| KeyValues2SerializationError::ParseF32Error { error, line, column })?,
    )
    .ok_or(KeyValues2SerializationError::TimeOutOfRange { line, column })
}

fn parse_color_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Color, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Color {
        red: arguments
            .next()
            .ok_or(KeyValues2SerializationError::MissingArgument { argument: "red", line, column })?
            .parse()
            .map_err(|error| KeyValues2SerializationError::ParseU8Error { error, line, column })?,
        green: arguments
            .next()
            .ok_or(KeyValues2SerializationError::MissingArgument {
                argument: "green",
                line,
                column,
            })?
            .parse()
            .map_err(|error| KeyValues2SerializationError::ParseU8Error { error, line, column })?,
        blue: arguments
            .next()
            .ok_or(KeyValues2SerializationError::MissingArgument {
                argument: "blue",
                line,
                column,
            })?
            .parse()
            .map_err(|error| KeyValues2SerializationError::ParseU8Error { error, line, column })?,
        alpha: arguments
            .next()
            .ok_or(KeyValues2SerializationError::MissingArgument {
                argument: "alpha",
                line,
                column,
            })?
            .parse()
            .map_err(|error| KeyValues2SerializationError::ParseU8Error { error, line, column })?,
    })
}

fn parse_argument(arguments: &mut SplitWhitespace<'_>, value: &'static str, line: usize, column: usize) -> Result<f32, KeyValues2SerializationError> {
    arguments
        .next()
        .ok_or(KeyValues2SerializationError::MissingArgument { argument: value, line, column })?
        .parse()
        .map_err(|error| KeyValues2SerializationError::ParseF32Error { error, line, column })
}

fn parse_vector2_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Vector2, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Vector2 {
        x: parse_argument(&mut arguments, "x", line, column)?,
        y: parse_argument(&mut arguments, "y", line, column)?,
    })
}

fn parse_vector3_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Vector3, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Vector3 {
        x: parse_argument(&mut arguments, "x", line, column)?,
        y: parse_argument(&mut arguments, "y", line, column)?,
        z: parse_argument(&mut arguments, "z", line, column)?,
    })
}

fn parse_vector4_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Vector4, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Vector4 {
        x: parse_argument(&mut arguments, "x", line, column)?,
        y: parse_argument(&mut arguments, "y", line, column)?,
        z: parse_argument(&mut arguments, "z", line, column)?,
        w: parse_argument(&mut arguments, "w", line, column)?,
    })
}

fn parse_angle_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Angle, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Angle {
        pitch: parse_argument(&mut arguments, "pitch", line, column)?,
        yaw: parse_argument(&mut arguments, "yaw", line, column)?,
        roll: parse_argument(&mut arguments, "roll", line, column)?,
    })
}

fn parse_quaternion_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Quaternion, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Quaternion {
        x: parse_argument(&mut arguments, "x", line, column)?,
        y: parse_argument(&mut arguments, "y", line, column)?,
        z: parse_argument(&mut arguments, "z", line, column)?,
        w: parse_argument(&mut arguments, "w", line, column)?,
    })
}

fn parse_matrix_attribute(attribute_value: &str, line: usize, column: usize) -> Result<Matrix, KeyValues2SerializationError> {
    let mut arguments = attribute_value.split_whitespace();
    Ok(Matrix([
        [
            parse_argument(&mut arguments, "Element 0-0", line, column)?,
            parse_argument(&mut arguments, "Element 0-1", line, column)?,
            parse_argument(&mut arguments, "Element 0-2", line, column)?,
            parse_argument(&mut arguments, "Element 0-3", line, column)?,
        ],
        [
            parse_argument(&mut arguments, "Element 1-0", line, column)?,
            parse_argument(&mut arguments, "Element 1-1", line, column)?,
            parse_argument(&mut arguments, "Element 1-2", line, column)?,
            parse_argument(&mut arguments, "Element 1-3", line, column)?,
        ],
        [
            parse_argument(&mut arguments, "Element 2-0", line, column)?,
            parse_argument(&mut arguments, "Element 2-1", line, column)?,
            parse_argument(&mut arguments, "Element 2-2", line, column)?,
            parse_argument(&mut arguments, "Element 2-3", line, column)?,
        ],
        [
            parse_argument(&mut arguments, "Element 3-0", line, column)?,
            parse_argument(&mut arguments, "Element 3-1", line, column)?,
            parse_argument(&mut arguments, "Element 3-2", line, column)?,
            parse_argument(&mut arguments, "Element 3-3", line, column)?,
        ],
    ]))
}

fn parse_ulong_attribute(attribute_value: &str, line: usize, column: usize) -> Result<u64, KeyValues2SerializationError> {
    let value = attribute_value
        .strip_prefix("0x")
        .or_else(|| attribute_value.strip_prefix("0X"))
        .unwrap_or(attribute_value);
    u64::from_str_radix(value, HEX_RADIX).map_err(|error| KeyValues2SerializationError::ParseU64Error { error, line, column })
}

fn parse_ubyte_attribute(attribute_value: &str, line: usize, column: usize) -> Result<u8, KeyValues2SerializationError> {
    attribute_value
        .parse()
        .map_err(|error| KeyValues2SerializationError::ParseU8Error { error, line, column })
}
