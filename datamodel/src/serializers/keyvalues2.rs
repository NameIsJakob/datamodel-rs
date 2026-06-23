use std::io::{BufRead, Error as IOError, Write};

use indexmap::IndexMap;
use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

use crate::{
    attribute::{Angle, Attribute, AttributeType, AttributeValue, BinaryBlock, Color, Matrix, Quaternion, Time, Vector2, Vector3, Vector4},
    element::Element,
    serializing::{Header, Serializer},
};

/// An error returned by [KeyValues2Serializer] and [KeyValues2FlatSerializer] from serializing or deserializing.
#[derive(Debug, ThisError)]
pub enum KeyValues2SerializationError {
    #[error("IO Error: {0}")]
    Io(#[from] IOError),
    #[error("Header Serializer Is Different")]
    WrongEncoding,
    #[error("Header Serializer Version Is Different")]
    InvalidEncodingVersion,
    #[error("Unknown Token \"{0}\" At {1},{2}")]
    UnknownToken(char, usize, usize),
    #[error("Unknown Escape Character \"{0}\" At {1},{2}")]
    UnknownEscapeCharacter(char, usize, usize),
    #[error("Unfinished Escape Character At {0},{1}")]
    UnfinishedEscapeCharacter(usize, usize),
    #[error("Unfinished Quote String At {0},{1}")]
    UnfinishedQuoteString(usize, usize),
    #[error("Expected Open Brace At {0},{1}")]
    ExpectedOpenBrace(usize, usize),
    #[error("Unexpected Open Brace At {0},{1}")]
    UnexpectedOpenBrace(usize, usize),
    #[error("Unexpected Close Brace At {0},{1}")]
    UnexpectedCloseBrace(usize, usize),
    #[error("Expected Open Bracket At {0},{1}")]
    ExpectedOpenBracket(usize, usize),
    #[error("Unexpected Open Bracket At {0},{1}")]
    UnexpectedOpenBracket(usize, usize),
    #[error("Unexpected Close Bracket At {0},{1}")]
    UnexpectedCloseBracket(usize, usize),
    #[error("Unexpected End Of File")]
    UnexpectedEndOfFile,
    #[error("Failed To Parse Integer At {0},{1}")]
    ParseIntegerError(usize, usize),
    #[error("Failed To Parse Float At {0},{1}")]
    ParseFloatError(usize, usize),
    #[error("Failed To Parse Boolean At {0},{1}")]
    ParseBooleanError(usize, usize),
    #[error("Failed To Parse UUID At {0},{1}")]
    ParseUUIDError(usize, usize),
    #[error("Time Value Out Of Range At {0},{1} - Min {min} Max {max}", min = Time(i32::MIN).as_seconds(), max = Time(i32::MAX).as_seconds())]
    TimeAttributeOutOFRange(usize, usize),
    #[error("Invalid Id Attribute Type At {0},{1}")]
    InvalidNameAttributeType(usize, usize),
    #[error("Attribute \"name\" In Element \"{}\" Is Not Type String", element.get_id())]
    InvalidNameAttribute { element: Element },
    #[error("Attribute \"id\" In Element \"{}\" Can't Be Type ObjectId", element.get_id())]
    InvalidIdAttribute { element: Element },
    #[error("Element Generated With Existing Id")]
    DuplicateGeneratedElementId,
    #[error("Element Id \"{0}\" Already Exists")]
    DuplicateElementId(UUID),
    #[error("Invalid Attribute Value At {0},{1}")]
    InvalidAttributeValue(usize, usize),
    #[error("No Elements In File")]
    NoElements,
}

struct StringWriter<T: Write> {
    buffer: T,
    tab_index: usize,
}

impl<T: Write> StringWriter<T> {
    fn new(buffer: T) -> Self {
        Self { buffer, tab_index: 0 }
    }

    fn write_header(&mut self, line: &str) -> Result<(), KeyValues2SerializationError> {
        self.buffer.write_all(line.as_bytes())?;
        Ok(())
    }

    fn write_tabs(&mut self) -> Result<(), KeyValues2SerializationError> {
        if self.tab_index == 0 {
            return Ok(());
        }
        self.buffer.write_all(&vec![b'\t'; self.tab_index])?;
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<(), KeyValues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(line.as_bytes())?;
        self.buffer.write_all(b"\r\n")?;
        Ok(())
    }

    fn write_open_brace(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(b"{\r\n")?;
        self.tab_index += 1;
        Ok(())
    }

    fn write_close_brace(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.tab_index -= 1;
        self.write_tabs()?;
        self.buffer.write_all(b"}\r\n")?;
        Ok(())
    }

    fn write_open_bracket(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(b"[\r\n")?;
        self.tab_index += 1;
        Ok(())
    }

    fn write_close_bracket(&mut self) -> Result<(), KeyValues2SerializationError> {
        self.tab_index -= 1;
        self.write_tabs()?;
        self.buffer.write_all(b"]\r\n")?;
        Ok(())
    }

    fn write_attributes(&mut self, root: &Element, collected_elements: &IndexMap<Element, usize>) -> Result<(), KeyValues2SerializationError> {
        macro_rules! write_attribute_string {
            ($self:ident, $attribute_name:expr, $attribute_type:expr, $attribute_value:expr) => {
                self.write_line(&format!(
                    "\"{}\" \"{}\" \"{}\"",
                    self.format_escape_characters($attribute_name),
                    $attribute_type,
                    $attribute_value
                ))
            };

            ($self:ident, $attribute_name:expr, $attribute_type:expr) => {
                self.write_line(&format!("\"{}\" \"{}\"", self.format_escape_characters($attribute_name), $attribute_type))
            };
        }

        for (name, attribute) in root.get_attributes().iter() {
            let attribute_type_name = Self::get_attribute_type_name(attribute);

            if name == "name" && attribute.get_type() != AttributeType::String {
                return Err(KeyValues2SerializationError::InvalidNameAttribute { element: Element::clone(root) });
            }

            if name == "id" && attribute.get_type() == AttributeType::ObjectId {
                return Err(KeyValues2SerializationError::InvalidIdAttribute { element: Element::clone(root) });
            }

            match &*attribute.get_inner() {
                AttributeValue::Element(element) => {
                    if let Some(element) = element {
                        let &count = collected_elements.get(element).unwrap();

                        if count > 0 {
                            write_attribute_string!(self, name, attribute_type_name, element.get_id())?;
                            continue;
                        }

                        write_attribute_string!(self, name, self.format_escape_characters(&element.get_class()))?;
                        self.write_open_brace()?;
                        write_attribute_string!(self, "id", "elementid", element.get_id())?;
                        self.write_attributes(element, collected_elements)?;
                        self.write_close_brace()?;
                        self.write_line("")?;

                        continue;
                    }

                    write_attribute_string!(self, name, attribute_type_name, "")?;
                }
                AttributeValue::Integer(integer) => write_attribute_string!(self, name, attribute_type_name, integer)?,
                AttributeValue::Float(float) => write_attribute_string!(self, name, attribute_type_name, float)?,
                AttributeValue::Boolean(boolean) => write_attribute_string!(self, name, attribute_type_name, *boolean as u8)?,
                AttributeValue::String(string) => write_attribute_string!(self, name, attribute_type_name, self.format_escape_characters(string))?,
                AttributeValue::Binary(binary) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_line("\"")?;
                    self.tab_index += 1;
                    for chunk in binary.0.chunks(40) {
                        self.write_line(
                            &chunk
                                .iter()
                                .fold(String::with_capacity(chunk.len() * 2), |mut output, byte| {
                                    output.push_str(&format!("{byte:02X}"));
                                    output
                                })
                                .to_string(),
                        )?;
                    }
                    self.tab_index -= 1;
                    self.write_line("\"")?;
                }
                AttributeValue::ObjectId(uuid) => write_attribute_string!(self, name, attribute_type_name, uuid)?,
                AttributeValue::Time(time) => write_attribute_string!(self, name, attribute_type_name, format!("{:.4}", time.as_seconds()))?,
                AttributeValue::Color(color) => write_attribute_string!(
                    self,
                    name,
                    attribute_type_name,
                    format!("{} {} {} {}", color.red, color.green, color.blue, color.alpha)
                )?,
                AttributeValue::Vector2(vector2) => write_attribute_string!(self, name, attribute_type_name, format!("{} {}", vector2.x, vector2.y))?,
                AttributeValue::Vector3(vector3) => {
                    write_attribute_string!(self, name, attribute_type_name, format!("{} {} {}", vector3.x, vector3.y, vector3.z))?
                }
                AttributeValue::Vector4(vector4) => write_attribute_string!(
                    self,
                    name,
                    attribute_type_name,
                    format!("{} {} {} {}", vector4.x, vector4.y, vector4.z, vector4.w)
                )?,
                AttributeValue::Angle(angle) => {
                    write_attribute_string!(self, name, attribute_type_name, format!("{} {} {}", angle.pitch, angle.yaw, angle.roll))?
                }
                AttributeValue::Quaternion(quaternion) => write_attribute_string!(
                    self,
                    name,
                    attribute_type_name,
                    format!("{} {} {} {}", quaternion.x, quaternion.y, quaternion.z, quaternion.w)
                )?,
                AttributeValue::Matrix(matrix) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_line("\"")?;
                    self.tab_index += 1;
                    self.write_line(&format!("{} {} {} {}", matrix.0[0][0], matrix.0[0][1], matrix.0[0][2], matrix.0[0][3]))?;
                    self.write_line(&format!("{} {} {} {}", matrix.0[1][0], matrix.0[1][1], matrix.0[1][2], matrix.0[1][3]))?;
                    self.write_line(&format!("{} {} {} {}", matrix.0[2][0], matrix.0[2][1], matrix.0[2][2], matrix.0[2][3]))?;
                    self.write_line(&format!("{} {} {} {}", matrix.0[3][0], matrix.0[3][1], matrix.0[3][2], matrix.0[3][3]))?;
                    self.tab_index -= 1;
                    self.write_line("\"")?;
                }
                AttributeValue::ULong(unsigned_long) => write_attribute_string!(self, name, attribute_type_name, format!("0x{unsigned_long:01X}"))?,
                AttributeValue::UByte(unsigned_byte) => write_attribute_string!(self, name, attribute_type_name, unsigned_byte)?,
                AttributeValue::ElementArray(elements) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_element, elements)) = elements.split_last() {
                        for element in elements {
                            if let Some(element) = element {
                                let &count = collected_elements.get(element).unwrap();

                                if count > 0 {
                                    self.write_line(&format!("\"element\" \"{}\",", element.get_id()))?;
                                    continue;
                                }

                                self.write_line(&format!("\"{}\"", self.format_escape_characters(&element.get_class())))?;
                                self.write_open_brace()?;
                                write_attribute_string!(self, "id", "elementid", element.get_id())?;
                                self.write_attributes(element, collected_elements)?;
                                self.tab_index -= 1;
                                self.write_line("},")?;

                                continue;
                            }

                            self.write_line("\"element\" \"\",")?;
                        }

                        if let Some(element) = last_element {
                            let &count = collected_elements.get(element).unwrap();

                            if count > 0 {
                                self.write_line(&format!("\"element\" \"{}\"", element.get_id()))?;
                            } else {
                                self.write_line(&format!("\"{}\"", self.format_escape_characters(&element.get_class())))?;
                                self.write_open_brace()?;
                                write_attribute_string!(self, "id", "elementid", element.get_id())?;
                                self.write_attributes(element, collected_elements)?;
                                self.write_close_brace()?;
                            }
                        } else {
                            self.write_line("\"element\" \"\"")?;
                        }
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::IntegerArray(integers) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_integer, integers)) = integers.split_last() {
                        for integer in integers {
                            self.write_line(&format!("\"{integer}\","))?;
                        }
                        self.write_line(&format!("\"{last_integer}\""))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::FloatArray(floats) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_float, floats)) = floats.split_last() {
                        for float in floats {
                            self.write_line(&format!("\"{float}\","))?;
                        }
                        self.write_line(&format!("\"{last_float}\""))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::BooleanArray(booleans) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_boolean, booleans)) = booleans.split_last() {
                        for boolean in booleans {
                            self.write_line(&format!("\"{}\",", *boolean as u8))?;
                        }
                        self.write_line(&format!("\"{}\"", *last_boolean as u8))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::StringArray(strings) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_string, strings)) = strings.split_last() {
                        for string in strings {
                            self.write_line(&format!("\"{}\",", self.format_escape_characters(string)))?;
                        }
                        self.write_line(&format!("\"{}\"", self.format_escape_characters(last_string)))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::BinaryArray(binaries) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_binary, binaries)) = binaries.split_last() {
                        for binary in binaries {
                            self.write_line("\"")?;
                            self.tab_index += 1;
                            for chunk in binary.0.chunks(40) {
                                self.write_line(
                                    &chunk
                                        .iter()
                                        .fold(String::with_capacity(chunk.len() * 2), |mut output, byte| {
                                            output.push_str(&format!("{byte:02X}"));
                                            output
                                        })
                                        .to_string(),
                                )?;
                            }
                            self.tab_index -= 1;
                            self.write_line("\",")?;
                        }
                        self.write_line("\"")?;
                        self.tab_index += 1;
                        for chunk in last_binary.0.chunks(40) {
                            self.write_line(
                                &chunk
                                    .iter()
                                    .fold(String::with_capacity(chunk.len() * 2), |mut output, byte| {
                                        output.push_str(&format!("{byte:02X}"));
                                        output
                                    })
                                    .to_string(),
                            )?;
                        }
                        self.tab_index -= 1;
                        self.write_line("\"")?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::ObjectIdArray(uuids) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_uuid, uuids)) = uuids.split_last() {
                        for uuid in uuids {
                            self.write_line(&format!("\"{uuid}\","))?;
                        }
                        self.write_line(&format!("\"{last_uuid}\""))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::TimeArray(times) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_time, times)) = times.split_last() {
                        for time in times {
                            self.write_line(&format!("\"{:.4}\",", time.as_seconds()))?;
                        }
                        self.write_line(&format!("\"{:.4}\"", last_time.as_seconds()))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::ColorArray(colors) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_color, colors)) = colors.split_last() {
                        for color in colors {
                            self.write_line(&format!("\"{} {} {} {}\",", color.red, color.green, color.blue, color.alpha))?;
                        }
                        self.write_line(&format!("\"{} {} {} {}\"", last_color.red, last_color.green, last_color.blue, last_color.alpha))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::Vector2Array(vector2s) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_vector2, vector2s)) = vector2s.split_last() {
                        for vector2 in vector2s {
                            self.write_line(&format!("\"{} {}\",", vector2.x, vector2.y))?;
                        }
                        self.write_line(&format!("\"{} {}\"", last_vector2.x, last_vector2.y))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::Vector3Array(vector3s) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_vector3, vector3s)) = vector3s.split_last() {
                        for vector3 in vector3s {
                            self.write_line(&format!("\"{} {} {}\",", vector3.x, vector3.y, vector3.z))?;
                        }
                        self.write_line(&format!("\"{} {} {}\"", last_vector3.x, last_vector3.y, last_vector3.z))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::Vector4Array(vector4s) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_vector4, vector4s)) = vector4s.split_last() {
                        for vector4 in vector4s {
                            self.write_line(&format!("\"{} {} {} {}\",", vector4.x, vector4.y, vector4.z, vector4.w))?;
                        }
                        self.write_line(&format!("\"{} {} {} {}\"", last_vector4.x, last_vector4.y, last_vector4.z, last_vector4.w))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::AngleArray(angles) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_angle, angles)) = angles.split_last() {
                        for angle in angles {
                            self.write_line(&format!("\"{} {} {}\",", angle.pitch, angle.yaw, angle.roll))?;
                        }
                        self.write_line(&format!("\"{} {} {}\"", last_angle.pitch, last_angle.yaw, last_angle.roll))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::QuaternionArray(quaternions) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_quaternion, quaternions)) = quaternions.split_last() {
                        for quaternion in quaternions {
                            self.write_line(&format!("\"{} {} {} {}\",", quaternion.x, quaternion.y, quaternion.z, quaternion.w))?;
                        }
                        self.write_line(&format!(
                            "\"{} {} {} {}\"",
                            last_quaternion.x, last_quaternion.y, last_quaternion.z, last_quaternion.w
                        ))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::MatrixArray(matrixes) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_matrix, matrixes)) = matrixes.split_last() {
                        for matrix in matrixes {
                            self.write_line("\"")?;
                            self.tab_index += 1;
                            self.write_line(&format!("{} {} {} {}", matrix.0[0][0], matrix.0[0][1], matrix.0[0][2], matrix.0[0][3]))?;
                            self.write_line(&format!("{} {} {} {}", matrix.0[1][0], matrix.0[1][1], matrix.0[1][2], matrix.0[1][3]))?;
                            self.write_line(&format!("{} {} {} {}", matrix.0[2][0], matrix.0[2][1], matrix.0[2][2], matrix.0[2][3]))?;
                            self.write_line(&format!("{} {} {} {}", matrix.0[3][0], matrix.0[3][1], matrix.0[3][2], matrix.0[3][3]))?;
                            self.tab_index -= 1;
                            self.write_line("\",")?;
                        }
                        self.write_line("\"")?;
                        self.tab_index += 1;
                        self.write_line(&format!(
                            "{} {} {} {}",
                            last_matrix.0[0][0], last_matrix.0[0][1], last_matrix.0[0][2], last_matrix.0[0][3]
                        ))?;
                        self.write_line(&format!(
                            "{} {} {} {}",
                            last_matrix.0[1][0], last_matrix.0[1][1], last_matrix.0[1][2], last_matrix.0[1][3]
                        ))?;
                        self.write_line(&format!(
                            "{} {} {} {}",
                            last_matrix.0[2][0], last_matrix.0[2][1], last_matrix.0[2][2], last_matrix.0[2][3]
                        ))?;
                        self.write_line(&format!(
                            "{} {} {} {}",
                            last_matrix.0[3][0], last_matrix.0[3][1], last_matrix.0[3][2], last_matrix.0[3][3]
                        ))?;
                        self.tab_index -= 1;
                        self.write_line("\"")?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::ULongArray(unsigned_longs) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_unsigned_long, unsigned_longs)) = unsigned_longs.split_last() {
                        for unsigned_long in unsigned_longs {
                            self.write_line(&format!("\"0x{unsigned_long:01X}\","))?;
                        }
                        self.write_line(&format!("\"0x{last_unsigned_long:01X}\""))?;
                    }
                    self.write_close_bracket()?;
                }
                AttributeValue::UByteArray(unsigned_bytes) => {
                    write_attribute_string!(self, name, attribute_type_name)?;
                    self.write_open_bracket()?;
                    if let Some((last_unsigned_byte, unsigned_bytes)) = unsigned_bytes.split_last() {
                        for unsigned_byte in unsigned_bytes {
                            self.write_line(&format!("\"{unsigned_byte}\","))?;
                        }
                        self.write_line(&format!("\"{last_unsigned_byte}\""))?;
                    }
                    self.write_close_bracket()?;
                }
            }
        }
        Ok(())
    }

    fn get_attribute_type_name(attribute: &Attribute) -> &'static str {
        match attribute.get_type() {
            AttributeType::Element => "element",
            AttributeType::Integer => "int",
            AttributeType::Float => "float",
            AttributeType::Boolean => "bool",
            AttributeType::String => "string",
            AttributeType::Binary => "binary",
            AttributeType::ObjectId => "elementid",
            AttributeType::Time => "time",
            AttributeType::Color => "color",
            AttributeType::Vector2 => "vector2",
            AttributeType::Vector3 => "vector3",
            AttributeType::Vector4 => "vector4",
            AttributeType::Angle => "qangle",
            AttributeType::Quaternion => "quaternion",
            AttributeType::Matrix => "matrix",
            AttributeType::ULong => "uint64",
            AttributeType::UByte => "uint8",
            AttributeType::ElementArray => "element_array",
            AttributeType::IntegerArray => "int_array",
            AttributeType::FloatArray => "float_array",
            AttributeType::BooleanArray => "bool_array",
            AttributeType::StringArray => "string_array",
            AttributeType::BinaryArray => "binary_array",
            AttributeType::ObjectIdArray => "elementid_array",
            AttributeType::TimeArray => "time_array",
            AttributeType::ColorArray => "color_array",
            AttributeType::Vector2Array => "vector2_array",
            AttributeType::Vector3Array => "vector3_array",
            AttributeType::Vector4Array => "vector4_array",
            AttributeType::AngleArray => "qangle_array",
            AttributeType::QuaternionArray => "quaternion_array",
            AttributeType::MatrixArray => "matrix_array",
            AttributeType::ULongArray => "uint64_array",
            AttributeType::UByteArray => "uint8_array",
        }
    }

    fn format_escape_characters(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars();

        while let Some(character) = chars.next() {
            match character {
                '\\' => match chars.next() {
                    Some('\\') => {
                        result.push('\\');
                        result.push('\\');
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
                '\'' => {
                    result.push('\\');
                    result.push('\'');
                }
                '"' => {
                    result.push('\\');
                    result.push('"');
                }
                _ => result.push(character),
            }
        }

        result
    }
}

struct StringReader<T: BufRead> {
    buffer: T,
    current_line: String,
    line: usize,
    column: usize,
}

impl<T: BufRead> StringReader<T> {
    fn new(buffer: T) -> Self {
        Self {
            buffer,
            current_line: String::new(),
            line: 1,
            column: 0,
        }
    }

    fn next_token(&mut self) -> Result<Option<ReadToken>, KeyValues2SerializationError> {
        if self.current_line.len() == self.column {
            self.current_line = match self.next_line()? {
                Some(line) => line,
                None => return Ok(None),
            };
            self.line += 1;
            self.column = 0;
        }

        let mut line_characters = self.current_line[self.column..].chars().peekable();
        let mut token = None;

        loop {
            let current_character = line_characters.next();
            self.column += 1;

            match current_character {
                Some('/') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push('/');
                        continue;
                    }

                    if let Some('/') = line_characters.peek() {
                        self.current_line = match self.next_line()? {
                            Some(line) => line,
                            None => return Ok(None),
                        };
                        self.line += 1;
                        self.column = 0;
                        line_characters = self.current_line.chars().peekable();
                        continue;
                    }

                    return Err(KeyValues2SerializationError::UnknownToken('/', self.line, self.column));
                }
                Some('"') => {
                    if matches!(token, Some(ReadToken::String(_))) {
                        break;
                    }

                    token = Some(ReadToken::String(String::with_capacity(32)));
                }
                Some('{') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push('{');
                        continue;
                    }

                    token = Some(ReadToken::OpenBrace);
                    break;
                }
                Some('}') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push('}');
                        continue;
                    }

                    token = Some(ReadToken::CloseBrace);
                    break;
                }
                Some('[') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push('[');
                        continue;
                    }

                    token = Some(ReadToken::OpenBracket);
                    break;
                }
                Some(']') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push(']');
                        continue;
                    }

                    token = Some(ReadToken::CloseBracket);
                    break;
                }
                Some(',') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push(',');
                    }
                }
                Some('<') => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        string_token.push('<');
                        continue;
                    }

                    self.current_line = match self.next_line()? {
                        Some(line) => line,
                        None => return Ok(None),
                    };
                    self.line += 1;
                    self.column = 0;
                    line_characters = self.current_line.chars().peekable();
                    continue;
                }
                Some(character) => {
                    if let Some(ReadToken::String(ref mut string_token)) = token {
                        if character == '\\' {
                            match line_characters.next() {
                                Some('n') => {
                                    string_token.push('\n');
                                }
                                Some('t') => {
                                    string_token.push('\t');
                                }
                                Some('v') => {
                                    string_token.push('v');
                                }
                                Some('b') => {
                                    string_token.push('b');
                                }
                                Some('r') => {
                                    string_token.push('\r');
                                }
                                Some('f') => {
                                    string_token.push('f');
                                }
                                Some('a') => {
                                    string_token.push('a');
                                }
                                Some('\\') => {
                                    string_token.push('\\');
                                }
                                Some('?') => {
                                    string_token.push('?');
                                }
                                Some('\'') => {
                                    string_token.push('\'');
                                }
                                Some('"') => {
                                    string_token.push('"');
                                }
                                Some(escape_character) => {
                                    if escape_character.is_whitespace() {
                                        return Err(KeyValues2SerializationError::UnfinishedEscapeCharacter(self.line, self.column));
                                    }
                                    return Err(KeyValues2SerializationError::UnknownEscapeCharacter(escape_character, self.line, self.column));
                                }
                                None => {
                                    return Err(KeyValues2SerializationError::UnfinishedEscapeCharacter(self.line, self.column));
                                }
                            }
                            self.column += 1;
                            continue;
                        }

                        string_token.push(character);
                        continue;
                    }

                    if character.is_whitespace() {
                        continue;
                    }

                    return Err(KeyValues2SerializationError::UnknownToken(character, self.line, self.column));
                }
                None => {
                    self.current_line = match self.next_line()? {
                        Some(line) => line,
                        None => {
                            if let Some(ReadToken::String(_)) = token {
                                return Err(KeyValues2SerializationError::UnfinishedQuoteString(self.line, self.column));
                            }
                            return Ok(None);
                        }
                    };
                    self.line += 1;
                    self.column = 0;
                    line_characters = self.current_line.chars().peekable();
                }
            }
        }

        Ok(token)
    }

    fn next_line(&mut self) -> Result<Option<String>, KeyValues2SerializationError> {
        let mut line = String::new();
        let byte_count = self.buffer.read_line(&mut line)?;
        if byte_count == 0 {
            return Ok(None);
        }
        Ok(Some(line))
    }

    fn read_element(
        &mut self,
        collected_elements: &mut IndexMap<UUID, Element>,
        element_remap: &mut IndexMap<Element, Vec<(String, ElementAttributeRemap)>>,
    ) -> Result<Option<Element>, KeyValues2SerializationError> {
        let element_class = match self.next_token()? {
            Some(ReadToken::String(string_token)) => string_token,
            Some(ReadToken::OpenBrace) => {
                return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column));
            }
            Some(ReadToken::CloseBrace) => {
                return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column));
            }
            Some(ReadToken::OpenBracket) => {
                return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
            }
            Some(ReadToken::CloseBracket) => {
                return Err(KeyValues2SerializationError::UnexpectedCloseBracket(self.line, self.column));
            }
            None => return Ok(None),
        };

        let mut element = Element::new(element_class);
        if collected_elements.insert(*element.get_id(), Element::clone(&element)).is_some() {
            return Err(KeyValues2SerializationError::DuplicateGeneratedElementId);
        }

        if !matches!(self.next_token()?, Some(ReadToken::OpenBrace)) {
            return Err(KeyValues2SerializationError::ExpectedOpenBrace(self.line, self.column));
        }

        self.read_attributes(&mut element, collected_elements, element_remap)?;

        Ok(Some(element))
    }

    fn read_attributes(
        &mut self,
        element: &mut Element,
        collected_elements: &mut IndexMap<UUID, Element>,
        element_remap: &mut IndexMap<Element, Vec<(String, ElementAttributeRemap)>>,
    ) -> Result<(), KeyValues2SerializationError> {
        loop {
            let attribute_name = match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                ReadToken::String(string_token) => string_token,
                ReadToken::OpenBrace => {
                    return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column));
                }
                ReadToken::CloseBrace => return Ok(()),
                ReadToken::OpenBracket => {
                    return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
                }
                ReadToken::CloseBracket => {
                    return Err(KeyValues2SerializationError::UnexpectedCloseBracket(self.line, self.column));
                }
            };

            let attribute_type = match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                ReadToken::String(string_token) => string_token,
                ReadToken::OpenBrace => {
                    return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column));
                }
                ReadToken::CloseBrace => {
                    return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column));
                }
                ReadToken::OpenBracket => {
                    return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
                }
                ReadToken::CloseBracket => {
                    return Err(KeyValues2SerializationError::UnexpectedCloseBracket(self.line, self.column));
                }
            };

            if attribute_name == "name" && attribute_type != "string" {
                return Err(KeyValues2SerializationError::InvalidNameAttributeType(
                    self.line,
                    self.column.saturating_sub(attribute_type.len().saturating_sub(1)),
                ));
            }

            if attribute_name == "id" && attribute_type == "elementid" {
                let attribute_value = match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                    ReadToken::String(string_token) => string_token,
                    ReadToken::OpenBrace => {
                        return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column));
                    }
                    ReadToken::CloseBrace => {
                        return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column));
                    }
                    ReadToken::OpenBracket => {
                        return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
                    }
                    ReadToken::CloseBracket => {
                        return Err(KeyValues2SerializationError::UnexpectedCloseBracket(self.line, self.column));
                    }
                };

                let element_id = attribute_value.parse::<UUID>().map_err(|_| {
                    KeyValues2SerializationError::ParseUUIDError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                })?;

                if element_id == *element.get_id() {
                    continue;
                }

                if collected_elements.contains_key(&element_id) {
                    return Err(KeyValues2SerializationError::DuplicateElementId(element_id));
                }

                collected_elements.shift_remove(&*element.get_id()).unwrap();
                element.set_id(element_id);
                collected_elements.insert(element_id, Element::clone(element));
                continue;
            }

            if let Some(attribute) = self.read_attribute_value(&attribute_type)? {
                element.set_attribute(attribute_name, Attribute::new(attribute));
                continue;
            }

            if let Some(array_attribute) = self.read_attribute_array(&attribute_type)? {
                element.set_attribute(attribute_name, Attribute::new(array_attribute));
                continue;
            }

            if attribute_type == "element" {
                let attribute_value = match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                    ReadToken::String(string_token) => string_token,
                    ReadToken::OpenBrace => {
                        return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column));
                    }
                    ReadToken::CloseBrace => {
                        return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column));
                    }
                    ReadToken::OpenBracket => {
                        return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
                    }
                    ReadToken::CloseBracket => {
                        return Err(KeyValues2SerializationError::UnexpectedCloseBracket(self.line, self.column));
                    }
                };

                if attribute_value.is_empty() {
                    element.set_attribute(attribute_name, Attribute::new(AttributeValue::Element(None)));
                    continue;
                }

                let element_id = attribute_value.parse::<UUID>().map_err(|_| {
                    KeyValues2SerializationError::ParseUUIDError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                })?;

                element_remap
                    .entry(Element::clone(element))
                    .or_default()
                    .push((attribute_name.clone(), ElementAttributeRemap::Single(element_id)));

                element.set_attribute(attribute_name, Attribute::new(AttributeValue::Element(None)));
                continue;
            }

            if attribute_type == "element_array" {
                if !matches!(self.next_token()?, Some(ReadToken::OpenBracket)) {
                    return Err(KeyValues2SerializationError::ExpectedOpenBracket(self.line, self.column));
                }

                let mut elements = Vec::new();
                let mut remaps = Vec::new();

                loop {
                    let attribute_value = match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                        ReadToken::String(string_token) => string_token,
                        ReadToken::OpenBrace => {
                            return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column));
                        }
                        ReadToken::CloseBrace => {
                            return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column));
                        }
                        ReadToken::OpenBracket => {
                            return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
                        }
                        ReadToken::CloseBracket => break,
                    };

                    match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                        ReadToken::String(string_token) => {
                            if attribute_value != "element" {
                                return Err(KeyValues2SerializationError::ExpectedOpenBrace(
                                    self.line,
                                    self.column.saturating_sub(attribute_value.len().saturating_sub(1)),
                                ));
                            }

                            if string_token.is_empty() {
                                elements.push(None);
                                continue;
                            }

                            let element_id = string_token.parse::<UUID>().map_err(|_| {
                                KeyValues2SerializationError::ParseUUIDError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                            })?;

                            remaps.push((elements.len(), element_id));
                            elements.push(None);
                        }
                        ReadToken::OpenBrace => {
                            elements.push(Some(self.read_element_attribute(attribute_value, collected_elements, element_remap)?));
                        }
                        ReadToken::CloseBrace => {
                            return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column));
                        }
                        ReadToken::OpenBracket => {
                            return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column));
                        }
                        ReadToken::CloseBracket => {
                            return Err(KeyValues2SerializationError::UnexpectedCloseBracket(self.line, self.column));
                        }
                    };
                }

                if !remaps.is_empty() {
                    element_remap
                        .entry(Element::clone(element))
                        .or_default()
                        .push((attribute_name.clone(), ElementAttributeRemap::Array(remaps)));
                }

                element.set_attribute(attribute_name, Attribute::new(AttributeValue::ElementArray(elements)));
                continue;
            }

            if !matches!(self.next_token()?, Some(ReadToken::OpenBrace)) {
                return Err(KeyValues2SerializationError::ExpectedOpenBrace(self.line, self.column));
            }

            element.set_attribute(
                attribute_name,
                Attribute::new(AttributeValue::Element(Some(self.read_element_attribute(
                    attribute_type,
                    collected_elements,
                    element_remap,
                )?))),
            );
        }
    }

    fn read_attribute_array(&mut self, attribute_type: &str) -> Result<Option<AttributeValue>, KeyValues2SerializationError> {
        macro_rules! parse_array_attribute {
            ($self:ident, $match_variant:path, $single_type:expr, $result_variant:path) => {
                if !matches!($self.next_token()?, Some(ReadToken::OpenBracket)) {
                    return Err(KeyValues2SerializationError::ExpectedOpenBracket($self.line, $self.column));
                } else {
                    let mut array = Vec::new();
                    while let Some($match_variant(value)) = self.read_attribute_value($single_type)? {
                        array.push(value);
                    }
                    Some($result_variant(array))
                }
            };
        }

        Ok(match attribute_type {
            "int_array" => {
                parse_array_attribute!(self, AttributeValue::Integer, "int", AttributeValue::IntegerArray)
            }
            "float_array" => {
                parse_array_attribute!(self, AttributeValue::Float, "float", AttributeValue::FloatArray)
            }
            "bool_array" => {
                parse_array_attribute!(self, AttributeValue::Boolean, "bool", AttributeValue::BooleanArray)
            }
            "string_array" => {
                parse_array_attribute!(self, AttributeValue::String, "string", AttributeValue::StringArray)
            }
            "binary_array" => {
                parse_array_attribute!(self, AttributeValue::Binary, "binary", AttributeValue::BinaryArray)
            }
            "elementid_array" => {
                parse_array_attribute!(self, AttributeValue::ObjectId, "elementid", AttributeValue::ObjectIdArray)
            }
            "time_array" => {
                parse_array_attribute!(self, AttributeValue::Time, "time", AttributeValue::TimeArray)
            }
            "color_array" => {
                parse_array_attribute!(self, AttributeValue::Color, "color", AttributeValue::ColorArray)
            }
            "vector2_array" => {
                parse_array_attribute!(self, AttributeValue::Vector2, "vector2", AttributeValue::Vector2Array)
            }
            "vector3_array" => {
                parse_array_attribute!(self, AttributeValue::Vector3, "vector3", AttributeValue::Vector3Array)
            }
            "vector4_array" => {
                parse_array_attribute!(self, AttributeValue::Vector4, "vector4", AttributeValue::Vector4Array)
            }
            "qangle_array" => {
                parse_array_attribute!(self, AttributeValue::Angle, "qangle", AttributeValue::AngleArray)
            }
            "quaternion_array" => {
                parse_array_attribute!(self, AttributeValue::Quaternion, "quaternion", AttributeValue::QuaternionArray)
            }
            "matrix_array" => {
                parse_array_attribute!(self, AttributeValue::Matrix, "matrix", AttributeValue::MatrixArray)
            }
            "uint64_array" => {
                parse_array_attribute!(self, AttributeValue::ULong, "uint64", AttributeValue::ULongArray)
            }
            "uint8_array" => {
                parse_array_attribute!(self, AttributeValue::UByte, "uint8", AttributeValue::UByteArray)
            }
            _ => None,
        })
    }

    fn read_attribute_value(&mut self, attribute_type: &str) -> Result<Option<AttributeValue>, KeyValues2SerializationError> {
        macro_rules! get_attribute_value {
            ($self:ident) => {
                match self.next_token()?.ok_or(KeyValues2SerializationError::UnexpectedEndOfFile)? {
                    ReadToken::String(string_token) => string_token,
                    ReadToken::OpenBrace => return Err(KeyValues2SerializationError::UnexpectedOpenBrace(self.line, self.column)),
                    ReadToken::CloseBrace => return Err(KeyValues2SerializationError::UnexpectedCloseBrace(self.line, self.column)),
                    ReadToken::OpenBracket => return Err(KeyValues2SerializationError::UnexpectedOpenBracket(self.line, self.column)),
                    ReadToken::CloseBracket => return Ok(None),
                }
            };
        }

        macro_rules! parse_primitive {
            ($self:ident, $attribute_value:expr, $parse_error_variant:path) => {
                $attribute_value
                    .parse()
                    .map_err(|_| $parse_error_variant(self.line, self.column.saturating_sub($attribute_value.len().saturating_sub(1))))?
            };
            ($self:ident, $tokens:expr, $attribute_value:expr, $parse_error_variant:path) => {
                $tokens
                    .next()
                    .ok_or(KeyValues2SerializationError::InvalidAttributeValue(
                        self.line,
                        self.column.saturating_sub($attribute_value.len().saturating_sub(1)),
                    ))?
                    .parse()
                    .map_err(|_| $parse_error_variant(self.line, self.column.saturating_sub($attribute_value.len().saturating_sub(1))))?
            };
        }

        Ok(match attribute_type {
            "int" => {
                let attribute_value = get_attribute_value!(self);

                Some(AttributeValue::Integer(parse_primitive!(
                    self,
                    attribute_value,
                    KeyValues2SerializationError::ParseIntegerError
                )))
            }
            "float" => {
                let attribute_value = get_attribute_value!(self);
                Some(AttributeValue::Float(parse_primitive!(
                    self,
                    attribute_value,
                    KeyValues2SerializationError::ParseFloatError
                )))
            }
            "bool" => {
                let attribute_value = get_attribute_value!(self);
                Some(AttributeValue::Boolean(
                    attribute_value.parse::<u8>().map_err(|_| {
                        KeyValues2SerializationError::ParseBooleanError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                    })? != 0,
                ))
            }
            "string" => {
                let attribute_value = get_attribute_value!(self);
                Some(AttributeValue::String(attribute_value))
            }
            "binary" => {
                let attribute_value = get_attribute_value!(self);
                let mut block = BinaryBlock::default();

                for byte in attribute_value.chars().filter(|c| !c.is_whitespace()).collect::<Vec<char>>().chunks(2) {
                    let byte = byte.iter().collect::<String>();
                    block.0.push(u8::from_str_radix(&byte, 16).map_err(|_| {
                        KeyValues2SerializationError::ParseIntegerError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                    })?);
                }

                Some(AttributeValue::Binary(block))
            }
            "elementid" => {
                let attribute_value = get_attribute_value!(self);
                let object_id = attribute_value.parse::<UUID>().map_err(|_| {
                    KeyValues2SerializationError::ParseUUIDError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                })?;

                Some(AttributeValue::ObjectId(object_id))
            }
            "time" => {
                let attribute_value = get_attribute_value!(self);
                let seconds: f32 = parse_primitive!(self, attribute_value, KeyValues2SerializationError::ParseFloatError);
                let tenths_of_milliseconds = seconds * 10000.0;

                if tenths_of_milliseconds > i32::MAX as f32 || tenths_of_milliseconds < i32::MIN as f32 {
                    return Err(KeyValues2SerializationError::TimeAttributeOutOFRange(self.line, self.column));
                }

                Some(AttributeValue::Time(Time((seconds + 0.5).floor() as i32)))
            }
            "color" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();
                Some(AttributeValue::Color(Color {
                    red: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseIntegerError),
                    green: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseIntegerError),
                    blue: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseIntegerError),
                    alpha: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseIntegerError),
                }))
            }
            "vector2" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();
                Some(AttributeValue::Vector2(Vector2 {
                    x: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    y: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                }))
            }
            "vector3" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();
                Some(AttributeValue::Vector3(Vector3 {
                    x: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    y: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    z: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                }))
            }
            "vector4" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();
                Some(AttributeValue::Vector4(Vector4 {
                    x: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    y: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    z: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    w: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                }))
            }
            "qangle" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();

                Some(AttributeValue::Angle(Angle {
                    pitch: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    yaw: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    roll: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                }))
            }
            "quaternion" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();
                Some(AttributeValue::Quaternion(Quaternion {
                    x: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    y: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    z: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    w: parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                }))
            }
            "matrix" => {
                let attribute_value = get_attribute_value!(self);
                let mut tokens = attribute_value.split_whitespace();
                Some(AttributeValue::Matrix(Matrix([
                    [
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    ],
                    [
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    ],
                    [
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    ],
                    [
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                        parse_primitive!(self, tokens, attribute_value, KeyValues2SerializationError::ParseFloatError),
                    ],
                ])))
            }
            "uint64" => {
                let attribute_value = get_attribute_value!(self);

                if let Some(stripped) = attribute_value.strip_prefix("0x") {
                    Some(AttributeValue::ULong(u64::from_str_radix(stripped, 16).map_err(|_| {
                        KeyValues2SerializationError::ParseIntegerError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                    })?))
                } else {
                    Some(AttributeValue::ULong(u64::from_str_radix(&attribute_value, 16).map_err(|_| {
                        KeyValues2SerializationError::ParseIntegerError(self.line, self.column.saturating_sub(attribute_value.len().saturating_sub(1)))
                    })?))
                }
            }
            "uint8" => {
                let attribute_value = get_attribute_value!(self);

                Some(AttributeValue::UByte(parse_primitive!(
                    self,
                    attribute_value,
                    KeyValues2SerializationError::ParseIntegerError
                )))
            }
            _ => None,
        })
    }

    fn read_element_attribute(
        &mut self,
        element_class: String,
        collected_elements: &mut IndexMap<UUID, Element>,
        element_remap: &mut IndexMap<Element, Vec<(String, ElementAttributeRemap)>>,
    ) -> Result<Element, KeyValues2SerializationError> {
        let mut element = Element::new(element_class);
        if collected_elements.insert(*element.get_id(), Element::clone(&element)).is_some() {
            return Err(KeyValues2SerializationError::DuplicateGeneratedElementId);
        }

        self.read_attributes(&mut element, collected_elements, element_remap)?;

        Ok(element)
    }
}

enum ElementAttributeRemap {
    Single(UUID),
    Array(Vec<(usize, UUID)>),
}

enum ReadToken {
    String(String),
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
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
            return Err(KeyValues2SerializationError::InvalidEncodingVersion);
        }

        let mut writer = StringWriter::new(buffer);
        writer.write_header(&header.create_header(Self::name(), version))?;

        fn collect_elements(root: Element, elements: &mut IndexMap<Element, usize>) {
            elements.insert(root.clone(), if elements.is_empty() { 1 } else { 0 });

            for attribute in root.get_attributes().values() {
                match &*attribute.get_inner() {
                    AttributeValue::Element(value) => match value {
                        Some(element) => {
                            if let Some(count) = elements.get_mut(element) {
                                *count += 1;
                                continue;
                            }
                            collect_elements(element.clone(), elements);
                        }
                        None => continue,
                    },
                    AttributeValue::ElementArray(values) => {
                        for value in values {
                            match value {
                                Some(element) => {
                                    if let Some(count) = elements.get_mut(element) {
                                        *count += 1;
                                        continue;
                                    }
                                    collect_elements(element.clone(), elements);
                                }
                                None => continue,
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut collected_elements = IndexMap::new();
        collect_elements(root.clone(), &mut collected_elements);

        for (element, &use_count) in &collected_elements {
            if use_count == 0 {
                continue;
            }

            writer.write_line(&format!("\"{}\"", writer.format_escape_characters(&element.get_class())))?;
            writer.write_open_brace()?;
            writer.write_line(&format!("\"id\" \"elementid\" \"{}\"", element.get_id()))?;
            writer.write_attributes(element, &collected_elements)?;
            writer.write_close_brace()?;
            writer.write_line("")?;
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(KeyValues2SerializationError::WrongEncoding);
        }

        if version < 1 || version > Self::version() {
            return Err(KeyValues2SerializationError::InvalidEncodingVersion);
        }

        let mut reader = StringReader::new(buffer);
        let mut collected_elements = IndexMap::new();
        let mut element_remap = IndexMap::new();
        let mut root = None;

        while let Some(root_element) = reader.read_element(&mut collected_elements, &mut element_remap)? {
            if root.is_none() && !root_element.get_class().eq("$prefix_element$") {
                root = Some(root_element);
            }
        }

        for (mut element, remapping) in element_remap {
            for (attribute_name, attribute_remap) in remapping {
                match attribute_remap {
                    ElementAttributeRemap::Single(uuid) => {
                        if let Some(reference_element) = collected_elements.get(&uuid) {
                            element.set_attribute(attribute_name, Attribute::new(AttributeValue::Element(Some(Element::clone(reference_element)))));
                        }
                    }
                    ElementAttributeRemap::Array(remaps) => {
                        if let Some(mut remapped_array) = element.get_attribute(&attribute_name).and_then(|attr| match &*attr.get_inner() {
                            AttributeValue::ElementArray(arr) => Some(arr.clone()),
                            _ => None,
                        }) {
                            for (index, uuid) in remaps {
                                if let Some(reference_element) = collected_elements.get(&uuid) {
                                    remapped_array[index] = Some(Element::clone(reference_element));
                                }
                            }

                            element.set_attribute(attribute_name, Attribute::new(AttributeValue::ElementArray(remapped_array)));
                        }
                    }
                }
            }
        }

        if let Some(root_element) = root {
            return Ok(root_element);
        }

        Err(KeyValues2SerializationError::NoElements)
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
            return Err(KeyValues2SerializationError::InvalidEncodingVersion);
        }

        let mut writer = StringWriter::new(buffer);
        writer.write_header(&header.create_header(Self::name(), version))?;

        fn collect_elements(root: Element, elements: &mut IndexMap<Element, usize>) {
            elements.insert(root.clone(), 1);

            for attribute in root.get_attributes().values() {
                match &*attribute.get_inner() {
                    AttributeValue::Element(value) => match value {
                        Some(element) => {
                            if let Some(count) = elements.get_mut(element) {
                                *count += 1;
                                continue;
                            }
                            collect_elements(element.clone(), elements);
                        }
                        None => continue,
                    },
                    AttributeValue::ElementArray(values) => {
                        for value in values {
                            match value {
                                Some(element) => {
                                    if let Some(count) = elements.get_mut(element) {
                                        *count += 1;
                                        continue;
                                    }
                                    collect_elements(element.clone(), elements);
                                }
                                None => continue,
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut collected_elements = IndexMap::new();
        collect_elements(root.clone(), &mut collected_elements);

        for (element, &use_count) in &collected_elements {
            if use_count == 0 {
                continue;
            }

            writer.write_line(&format!("\"{}\"", writer.format_escape_characters(&element.get_class())))?;
            writer.write_open_brace()?;
            writer.write_line(&format!("\"id\" \"elementid\" \"{}\"", element.get_id()))?;
            writer.write_attributes(element, &collected_elements)?;
            writer.write_close_brace()?;
            writer.write_line("")?;
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(KeyValues2SerializationError::WrongEncoding);
        }

        if version < 1 || version > Self::version() {
            return Err(KeyValues2SerializationError::InvalidEncodingVersion);
        }

        KeyValues2Serializer::deserialize(buffer, String::from(KeyValues2Serializer::name()), KeyValues2Serializer::version())
    }
}
