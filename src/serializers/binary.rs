use std::{
    io::{BufRead, Error, Write},
    str::FromStr,
};

use chrono::Duration;
use indexmap::IndexSet;
use thiserror::Error as ThisError;
use uuid::{Error as UUIDError, Uuid as UUID};

use crate::{
    Element, Header, Serializer,
    attribute::{Angle, Attribute, BinaryBlock, Color, Matrix, Quaternion, Vector2, Vector3, Vector4},
};

const MAX_SHORT_ARRAY_LENGTH: usize = (i16::MAX as usize) + 1;
const MAX_ARRAY_LENGTH: usize = (i32::MAX as usize) + 1;

/// Version uses a table for strings.
pub const VERSION_HAS_SYMBOL_TABLE: i32 = 2;
/// Version deprecates attribute object id and replaced with time attribute.
pub const VERSION_DEPRECATES_OBJECT_ID: i32 = 3;
/// Version that the symbol table size uses an int, element names use the symbol table, string attributes uses the symbol table.
pub const VERSION_GLOBAL_SYMBOL_TABLE: i32 = 4;
/// Version that the symbol table indexes uses int.
pub const VERSION_LARGE_SYMBOL_TABLE: i32 = 5;

/// Specifics that the element is null.
const ELEMENT_INDEX_NULL: i32 = -1;
/// Specifics that the element was not serialized but element id was.
const ELEMENT_INDEX_EXTERNAL: i32 = -2;

#[derive(Debug, ThisError)]
pub enum BinarySerializationError {
    #[error("Io Error, Error \"{0}\"")]
    IoError(#[from] Error),
    #[error("Can't Serialize Deprecated Attribute Type For Attribute \"{}\" In Element \"{}\"", .attribute, .element.get_id())]
    DeprecatedAttribute { attribute: String, element: Element },
    #[error("Can't Serialize Attribute Type For Attribute \"{}\" In Element \"{}\" As Version {} Doesn't Support It", .attribute, .element.get_id(), .version)]
    InvalidVersionForAttribute { attribute: String, element: Element, version: i32 },
    #[error("Too Many Symbols To Serialize To Table, Symbol Count {} - Max {}", .count, .max)]
    TooManySymbols { count: usize, max: usize },
    #[error("Too Many Elements To Serialize To Table, Element Count {} - Max {}", .count, MAX_ARRAY_LENGTH)]
    TooManyElements { count: usize },
    #[error("Element Has Too Many Attributes To Serialize In Element \"{}\", Attribute Count {} - Max {}", .element.get_id(), .count, MAX_ARRAY_LENGTH)]
    TooManyAttributes { element: Element, count: usize },
    #[error("Binary Attribute \"{}\" In Element \"{}\" Is Too Long, Binary Length {} - Max {}", .attribute, element.get_id(), .length, MAX_ARRAY_LENGTH)]
    BinaryDataTooLong { attribute: String, element: Element, length: usize },
    #[error("Attribute Array \"{}\" In Element \"{}\" Is Too Long, Attribute Length {} - Max {}", .attribute, element.get_id(), .length, MAX_ARRAY_LENGTH)]
    AttributeArrayTooLong { attribute: String, element: Element, length: usize },
    #[error("Encoding Past In Is Invalid, Invalid Encoding \"{}\" - Expected \"{}\"", .encoding, BinarySerializer::name())]
    InvalidEncoding { encoding: String },
    #[error("Version Past In Is Invalid, Invalid Version {} - Min {} Max {}", .version, 1, BinarySerializer::version())]
    InvalidVersion { version: i32 },
    #[error("Symbol Table Length Is Invalid, Invalid Length {} - Min {} Max {}", .length, 0, MAX_ARRAY_LENGTH)]
    InvalidSymbolTableLength { length: i32 },
    #[error("Element Table Length Is Invalid, Invalid Length {} - Min {} Max {}", .length, 1, MAX_ARRAY_LENGTH)]
    InvalidElementTableLength { length: i32 },
    #[error("Index To Symbol Table Is Invalid, Invalid Index {} - Min {} Max {}", .index, 0, .length)]
    InvalidSymbolTableIndex { index: i32, length: i32 },
    #[error("Element Attribute Count Is Invalid, Invalid Count {} - Min {} Max {}", .count, 0, MAX_ARRAY_LENGTH)]
    InvalidAttributeCount { count: i32 },
    #[error("Attribute Type Is Invalid For Attribute \"{}\", Invalid Type {} - Supported {} To {}", .attribute_name, .attribute_type, 1, 28)]
    InvalidAttributeType { attribute_name: String, attribute_type: i8 },
    #[error("Index To Element Table Is Invalid, Invalid Index {} - Min {} Max {}", .index, 0, .length)]
    InvalidElementTableIndex { index: i32, length: i32 },
    #[error("Failed To Parse UUID, Error \"{0}\"")]
    UUIDParseError(#[from] UUIDError),
    #[error("Binary Data Length Is Invalid, Invalid Length {} - Min {} Max {}", .length, 0, MAX_ARRAY_LENGTH)]
    InvalidBinaryDataLength { length: i32 },
    #[error("Attribute Array Length Is Invalid, Invalid Length {} - Min {} Max {}", .length, 0, MAX_ARRAY_LENGTH)]
    InvalidAttributeArrayLength { length: i32 },
}

/// Serialize elements to a binary format.
pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    type Error = BinarySerializationError;

    fn name() -> &'static str {
        "binary"
    }

    fn version() -> i32 {
        5
    }

    fn serialize_version(buffer: &mut impl Write, header: &Header, root: &Element, version: i32) -> Result<(), Self::Error> {
        if version < 0 || version > Self::version() {
            return Err(BinarySerializationError::InvalidVersion { version });
        }

        let mut writer = Writer::new(buffer);
        writer.write_string(&header.create_header(Self::name(), version))?;

        let mut collected_elements = IndexSet::new();
        let mut collected_symbols = IndexSet::new();
        let mut element_collection_stack = Vec::new();

        if collected_elements.insert(Element::clone(root)) {
            element_collection_stack.push(Element::clone(root));
        }
        while let Some(current_check_element) = element_collection_stack.pop() {
            if version >= VERSION_HAS_SYMBOL_TABLE {
                collected_symbols.insert(current_check_element.get_class().clone());
                if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                    collected_symbols.insert(current_check_element.get_name().clone());
                }
            }

            for (attribute_name, attribute_value) in current_check_element.get_attributes().iter() {
                if version >= VERSION_HAS_SYMBOL_TABLE {
                    collected_symbols.insert(attribute_name.clone());
                }

                match attribute_value {
                    Attribute::Element(value) => {
                        if let Some(element) = value
                            && collected_elements.insert(Element::clone(element))
                        {
                            element_collection_stack.push(Element::clone(element));
                        }
                    }
                    Attribute::String(value) => {
                        if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                            collected_symbols.insert(value.clone());
                        }
                    }
                    #[allow(deprecated)]
                    Attribute::ObjectId(_) => {
                        return Err(BinarySerializationError::DeprecatedAttribute {
                            attribute: attribute_name.clone(),
                            element: Element::clone(&current_check_element),
                        });
                    }
                    Attribute::Time(_) => {
                        if version < VERSION_DEPRECATES_OBJECT_ID {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(&current_check_element),
                                version,
                            });
                        }
                    }
                    Attribute::ElementArray(values) => {
                        for element in values.iter().flatten() {
                            if collected_elements.insert(Element::clone(element)) {
                                element_collection_stack.push(Element::clone(element));
                            }
                        }
                    }
                    #[allow(deprecated)]
                    Attribute::ObjectIdArray(_) => {
                        return Err(BinarySerializationError::DeprecatedAttribute {
                            attribute: attribute_name.clone(),
                            element: Element::clone(&current_check_element),
                        });
                    }
                    Attribute::TimeArray(_) => {
                        if version < VERSION_DEPRECATES_OBJECT_ID {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(&current_check_element),
                                version,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        let max_symbol_table_length = if version >= VERSION_GLOBAL_SYMBOL_TABLE {
            MAX_ARRAY_LENGTH
        } else {
            MAX_SHORT_ARRAY_LENGTH
        };
        if collected_symbols.len() > max_symbol_table_length {
            return Err(BinarySerializationError::TooManySymbols {
                count: collected_symbols.len(),
                max: max_symbol_table_length,
            });
        }

        if collected_elements.len() > MAX_ARRAY_LENGTH {
            return Err(BinarySerializationError::TooManyElements {
                count: collected_elements.len(),
            });
        }

        if version >= VERSION_HAS_SYMBOL_TABLE {
            if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                writer.write_integer(collected_symbols.len() as i32)?;
            } else {
                writer.write_short(collected_symbols.len() as i16)?;
            }
        }
        for symbol in &collected_symbols {
            writer.write_string(symbol)?;
        }

        writer.write_integer(collected_elements.len() as i32)?;
        for collected_element in &collected_elements {
            if version >= VERSION_HAS_SYMBOL_TABLE {
                if version >= VERSION_LARGE_SYMBOL_TABLE {
                    writer.write_integer(collected_symbols.get_index_of(collected_element.get_class().as_str()).unwrap() as i32)?;
                } else {
                    writer.write_short(collected_symbols.get_index_of(collected_element.get_class().as_str()).unwrap() as i16)?;
                }
            } else {
                writer.write_string(collected_element.get_class().as_str())?;
            }

            if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                if version >= VERSION_LARGE_SYMBOL_TABLE {
                    writer.write_integer(collected_symbols.get_index_of(collected_element.get_name().as_str()).unwrap() as i32)?;
                } else {
                    writer.write_short(collected_symbols.get_index_of(collected_element.get_name().as_str()).unwrap() as i16)?;
                }
            } else {
                writer.write_string(collected_element.get_name().as_str())?;
            }

            writer.write_uuid(*collected_element.get_id())?;
        }

        for collected_element in &collected_elements {
            let collected_element_attributes = collected_element.get_attributes();
            if collected_element_attributes.len() > MAX_ARRAY_LENGTH {
                return Err(BinarySerializationError::TooManyAttributes {
                    element: Element::clone(collected_element),
                    count: collected_element_attributes.len(),
                });
            }
            writer.write_integer(collected_element_attributes.len() as i32)?;

            for (attribute_name, attribute_value) in collected_element_attributes.iter() {
                if version >= VERSION_HAS_SYMBOL_TABLE {
                    if version >= VERSION_LARGE_SYMBOL_TABLE {
                        writer.write_integer(collected_symbols.get_index_of(attribute_name).unwrap() as i32)?;
                    } else {
                        writer.write_short(collected_symbols.get_index_of(attribute_name).unwrap() as i16)?;
                    }
                } else {
                    writer.write_string(attribute_name.as_str())?;
                }

                macro_rules! check_array_length {
                    ($values:expr) => {
                        if $values.len() > MAX_ARRAY_LENGTH {
                            return Err(BinarySerializationError::AttributeArrayTooLong {
                                attribute: attribute_name.clone(),
                                element: Element::clone(collected_element),
                                length: $values.len(),
                            });
                        }
                    };
                }

                match attribute_value {
                    Attribute::Element(value) => {
                        writer.write_byte(1)?;
                        let element_value = match value {
                            Some(element_value) => element_value,
                            None => {
                                writer.write_integer(-1)?;
                                continue;
                            }
                        };
                        writer.write_integer(collected_elements.get_index_of(element_value).unwrap() as i32)?;
                    }
                    Attribute::Integer(value) => {
                        writer.write_byte(2)?;
                        writer.write_integer(*value)?;
                    }
                    Attribute::Float(value) => {
                        writer.write_byte(3)?;
                        writer.write_float(*value)?;
                    }
                    Attribute::Boolean(value) => {
                        writer.write_byte(4)?;
                        writer.write_byte(*value as i8)?;
                    }
                    Attribute::String(value) => {
                        writer.write_byte(5)?;
                        if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                            if version >= VERSION_LARGE_SYMBOL_TABLE {
                                writer.write_integer(collected_symbols.get_index_of(value).unwrap() as i32)?;
                            } else {
                                writer.write_short(collected_symbols.get_index_of(value).unwrap() as i16)?;
                            }
                        } else {
                            writer.write_string(value)?;
                        }
                    }
                    Attribute::Binary(value) => {
                        writer.write_byte(6)?;
                        if value.0.len() > MAX_ARRAY_LENGTH {
                            return Err(BinarySerializationError::BinaryDataTooLong {
                                attribute: attribute_name.clone(),
                                element: Element::clone(collected_element),
                                length: value.0.len(),
                            });
                        }
                        writer.write_integer(value.0.len() as i32)?;
                        for byte in &value.0 {
                            writer.write_unsigned_byte(*byte)?;
                        }
                    }
                    Attribute::Time(value) => {
                        writer.write_byte(7)?;
                        writer.write_integer((value.as_seconds_f64() * 10_000f64) as i32)?;
                    }
                    Attribute::Color(value) => {
                        writer.write_byte(8)?;
                        writer.write_unsigned_byte(value.red)?;
                        writer.write_unsigned_byte(value.green)?;
                        writer.write_unsigned_byte(value.blue)?;
                        writer.write_unsigned_byte(value.alpha)?;
                    }
                    Attribute::Vector2(value) => {
                        writer.write_byte(9)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                    }
                    Attribute::Vector3(value) => {
                        writer.write_byte(10)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                        writer.write_float(value.z)?;
                    }
                    Attribute::Vector4(value) => {
                        writer.write_byte(11)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                        writer.write_float(value.z)?;
                        writer.write_float(value.w)?;
                    }
                    Attribute::Angle(value) => {
                        writer.write_byte(12)?;
                        writer.write_float(value.pitch)?;
                        writer.write_float(value.yaw)?;
                        writer.write_float(value.roll)?;
                    }
                    Attribute::Quaternion(value) => {
                        writer.write_byte(13)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                        writer.write_float(value.z)?;
                        writer.write_float(value.w)?;
                    }
                    Attribute::Matrix(value) => {
                        writer.write_byte(14)?;
                        writer.write_float(value.0[0][0])?;
                        writer.write_float(value.0[0][1])?;
                        writer.write_float(value.0[0][2])?;
                        writer.write_float(value.0[0][3])?;
                        writer.write_float(value.0[1][0])?;
                        writer.write_float(value.0[1][1])?;
                        writer.write_float(value.0[1][2])?;
                        writer.write_float(value.0[1][3])?;
                        writer.write_float(value.0[2][0])?;
                        writer.write_float(value.0[2][1])?;
                        writer.write_float(value.0[2][2])?;
                        writer.write_float(value.0[2][3])?;
                        writer.write_float(value.0[3][0])?;
                        writer.write_float(value.0[3][1])?;
                        writer.write_float(value.0[3][2])?;
                        writer.write_float(value.0[3][3])?;
                    }
                    Attribute::ElementArray(values) => {
                        writer.write_byte(15)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            let element_value = match value {
                                Some(element_value) => element_value,
                                None => {
                                    writer.write_integer(-1)?;
                                    continue;
                                }
                            };
                            writer.write_integer(collected_elements.get_index_of(element_value).unwrap() as i32)?;
                        }
                    }
                    Attribute::IntegerArray(values) => {
                        writer.write_byte(16)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_integer(*value)?;
                        }
                    }
                    Attribute::FloatArray(values) => {
                        writer.write_byte(17)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(*value)?;
                        }
                    }
                    Attribute::BooleanArray(values) => {
                        writer.write_byte(18)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_byte(*value as i8)?;
                        }
                    }
                    Attribute::StringArray(values) => {
                        writer.write_byte(19)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_string(value)?;
                        }
                    }
                    Attribute::BinaryArray(values) => {
                        writer.write_byte(20)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            if value.0.len() > MAX_ARRAY_LENGTH {
                                return Err(BinarySerializationError::BinaryDataTooLong {
                                    attribute: attribute_name.clone(),
                                    element: Element::clone(collected_element),
                                    length: value.0.len(),
                                });
                            }
                            writer.write_integer(value.0.len() as i32)?;
                            for byte in &value.0 {
                                writer.write_unsigned_byte(*byte)?;
                            }
                        }
                    }
                    Attribute::TimeArray(values) => {
                        writer.write_byte(21)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_integer((value.as_seconds_f64() * 10_000f64) as i32)?;
                        }
                    }
                    Attribute::ColorArray(values) => {
                        writer.write_byte(22)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_unsigned_byte(value.red)?;
                            writer.write_unsigned_byte(value.green)?;
                            writer.write_unsigned_byte(value.blue)?;
                            writer.write_unsigned_byte(value.alpha)?;
                        }
                    }
                    Attribute::Vector2Array(values) => {
                        writer.write_byte(23)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(value.x)?;
                            writer.write_float(value.y)?;
                        }
                    }
                    Attribute::Vector3Array(values) => {
                        writer.write_byte(24)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(value.x)?;
                            writer.write_float(value.y)?;
                            writer.write_float(value.z)?;
                        }
                    }
                    Attribute::Vector4Array(values) => {
                        writer.write_byte(25)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(value.x)?;
                            writer.write_float(value.y)?;
                            writer.write_float(value.z)?;
                            writer.write_float(value.w)?;
                        }
                    }
                    Attribute::AngleArray(values) => {
                        writer.write_byte(26)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(value.pitch)?;
                            writer.write_float(value.yaw)?;
                            writer.write_float(value.roll)?;
                        }
                    }
                    Attribute::QuaternionArray(values) => {
                        writer.write_byte(27)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(value.x)?;
                            writer.write_float(value.y)?;
                            writer.write_float(value.z)?;
                            writer.write_float(value.w)?;
                        }
                    }
                    Attribute::MatrixArray(values) => {
                        writer.write_byte(28)?;
                        check_array_length!(values);
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_float(value.0[0][0])?;
                            writer.write_float(value.0[0][1])?;
                            writer.write_float(value.0[0][2])?;
                            writer.write_float(value.0[0][3])?;
                            writer.write_float(value.0[1][0])?;
                            writer.write_float(value.0[1][1])?;
                            writer.write_float(value.0[1][2])?;
                            writer.write_float(value.0[1][3])?;
                            writer.write_float(value.0[2][0])?;
                            writer.write_float(value.0[2][1])?;
                            writer.write_float(value.0[2][2])?;
                            writer.write_float(value.0[2][3])?;
                            writer.write_float(value.0[3][0])?;
                            writer.write_float(value.0[3][1])?;
                            writer.write_float(value.0[3][2])?;
                            writer.write_float(value.0[3][3])?;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(BinarySerializationError::InvalidEncoding { encoding });
        }

        if version < 0 || version > Self::version() {
            return Err(BinarySerializationError::InvalidVersion { version });
        }

        let mut reader = Reader::new(buffer);
        reader.read_string()?;

        let symbol_table_length = if version >= VERSION_HAS_SYMBOL_TABLE {
            if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                reader.read_integer()?
            } else {
                reader.read_short()? as i32
            }
        } else {
            0
        };
        if symbol_table_length < 0 {
            return Err(BinarySerializationError::InvalidSymbolTableLength { length: symbol_table_length });
        }
        let mut symbol_table = Vec::with_capacity(symbol_table_length as usize);
        for _ in 0..symbol_table_length {
            symbol_table.push(reader.read_string()?);
        }

        macro_rules! get_string_from_table {
            () => {
                if version >= VERSION_HAS_SYMBOL_TABLE {
                    let string_index = if version >= VERSION_LARGE_SYMBOL_TABLE {
                        reader.read_integer()?
                    } else {
                        reader.read_short()? as i32
                    };
                    if string_index == -1 {
                        String::new()
                    } else if string_index < -1 || string_index > symbol_table_length {
                        return Err(BinarySerializationError::InvalidSymbolTableIndex {
                            index: string_index,
                            length: symbol_table_length,
                        });
                    } else {
                        symbol_table[string_index as usize].clone()
                    }
                } else {
                    reader.read_string()?
                }
            };
        }

        let element_table_length = reader.read_integer()?;
        if element_table_length <= 0 {
            return Err(BinarySerializationError::InvalidElementTableLength { length: symbol_table_length });
        }
        let mut element_table = Vec::with_capacity(element_table_length as usize);
        for _ in 0..element_table_length {
            let element_class = get_string_from_table!();
            let element_name = if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                get_string_from_table!()
            } else {
                reader.read_string()?
            };
            let element_id = reader.read_uuid()?;

            element_table.push(Element::full(element_name, element_class, element_id));
        }

        for current_element_index in 0..element_table.len() {
            let mut current_element = Element::clone(&element_table[current_element_index]);
            let current_element_attribute_length = reader.read_integer()?;
            if current_element_attribute_length < 0 {
                return Err(BinarySerializationError::InvalidAttributeCount {
                    count: current_element_attribute_length,
                });
            }

            macro_rules! read_attribute_array {
                ($body:block) => {{
                    let attribute_array_length = reader.read_integer()?;
                    if attribute_array_length < 0 {
                        return Err(BinarySerializationError::InvalidAttributeArrayLength {
                            length: attribute_array_length,
                        });
                    }
                    let mut attribute_array = Vec::with_capacity(attribute_array_length as usize);
                    for _ in 0..attribute_array_length {
                        attribute_array.push($body)
                    }
                    attribute_array
                }};
            }

            for _ in 0..current_element_attribute_length {
                let attribute_name = get_string_from_table!();

                let attribute_type = reader.read_byte()?;
                let attribute_value = match attribute_type {
                    1 => Attribute::Element(match reader.read_integer()? {
                        index if index < ELEMENT_INDEX_EXTERNAL || index > element_table_length => {
                            return Err(BinarySerializationError::InvalidElementTableIndex {
                                index,
                                length: element_table_length,
                            });
                        }
                        ELEMENT_INDEX_NULL => None,
                        ELEMENT_INDEX_EXTERNAL => Some(Element::full(
                            Element::DEFAULT_ELEMENT_NAME,
                            Element::DEFAULT_ELEMENT_CLASS,
                            UUID::from_str(&reader.read_string()?)?,
                        )),
                        index => Some(Element::clone(&element_table[index as usize])),
                    }),
                    2 => Attribute::Integer(reader.read_integer()?),
                    3 => Attribute::Float(reader.read_float()?),
                    4 => Attribute::Boolean(reader.read_unsigned_byte()? != 0),
                    5 => Attribute::String(if version >= VERSION_GLOBAL_SYMBOL_TABLE {
                        get_string_from_table!()
                    } else {
                        reader.read_string()?
                    }),
                    6 => {
                        let binary_data_length = reader.read_integer()?;
                        if binary_data_length > 0 {
                            return Err(BinarySerializationError::InvalidBinaryDataLength { length: binary_data_length });
                        }
                        let mut binary_data = Vec::with_capacity(binary_data_length as usize);
                        for _ in 0..binary_data_length {
                            binary_data.push(reader.read_unsigned_byte()?);
                        }
                        Attribute::Binary(BinaryBlock(binary_data))
                    }
                    attribute_type if attribute_type == 7 && version < VERSION_DEPRECATES_OBJECT_ID =>
                    {
                        #[allow(deprecated)]
                        Attribute::ObjectId(reader.read_uuid()?)
                    }
                    attribute_type if attribute_type == 7 && version >= VERSION_DEPRECATES_OBJECT_ID => {
                        Attribute::Time(Duration::nanoseconds(((reader.read_integer()? as f64 / 10_000.0) * 1_000_000_000.0) as i64))
                    }
                    8 => Attribute::Color(Color {
                        red: reader.read_unsigned_byte()?,
                        green: reader.read_unsigned_byte()?,
                        blue: reader.read_unsigned_byte()?,
                        alpha: reader.read_unsigned_byte()?,
                    }),
                    9 => Attribute::Vector2(Vector2 {
                        x: reader.read_float()?,
                        y: reader.read_float()?,
                    }),
                    10 => Attribute::Vector3(Vector3 {
                        x: reader.read_float()?,
                        y: reader.read_float()?,
                        z: reader.read_float()?,
                    }),
                    11 => Attribute::Vector4(Vector4 {
                        x: reader.read_float()?,
                        y: reader.read_float()?,
                        z: reader.read_float()?,
                        w: reader.read_float()?,
                    }),
                    12 => Attribute::Angle(Angle {
                        pitch: reader.read_float()?,
                        yaw: reader.read_float()?,
                        roll: reader.read_float()?,
                    }),
                    13 => Attribute::Quaternion(Quaternion {
                        x: reader.read_float()?,
                        y: reader.read_float()?,
                        z: reader.read_float()?,
                        w: reader.read_float()?,
                    }),
                    14 => Attribute::Matrix(Matrix([
                        [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                        [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                        [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                        [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                    ])),
                    15 => Attribute::ElementArray(read_attribute_array!({
                        match reader.read_integer()? {
                            index if index < ELEMENT_INDEX_EXTERNAL || index > element_table_length => {
                                return Err(BinarySerializationError::InvalidElementTableIndex {
                                    index,
                                    length: element_table_length,
                                });
                            }
                            ELEMENT_INDEX_NULL => None,
                            ELEMENT_INDEX_EXTERNAL => Some(Element::full(
                                Element::DEFAULT_ELEMENT_NAME,
                                Element::DEFAULT_ELEMENT_CLASS,
                                UUID::from_str(&reader.read_string()?)?,
                            )),
                            index => Some(Element::clone(&element_table[index as usize])),
                        }
                    })),
                    16 => Attribute::IntegerArray(read_attribute_array!({ reader.read_integer()? })),
                    17 => Attribute::FloatArray(read_attribute_array!({ reader.read_float()? })),
                    18 => Attribute::BooleanArray(read_attribute_array!({ reader.read_unsigned_byte()? != 0 })),
                    19 => Attribute::StringArray(read_attribute_array!({ reader.read_string()? })),
                    20 => Attribute::BinaryArray(read_attribute_array!({
                        let binary_data_length = reader.read_integer()?;
                        if binary_data_length > 0 {
                            return Err(BinarySerializationError::InvalidBinaryDataLength { length: binary_data_length });
                        }
                        let mut binary_data = Vec::with_capacity(binary_data_length as usize);
                        for _ in 0..binary_data_length {
                            binary_data.push(reader.read_unsigned_byte()?);
                        }
                        BinaryBlock(binary_data)
                    })),
                    attribute_type if attribute_type == 21 && version < VERSION_DEPRECATES_OBJECT_ID =>
                    {
                        #[allow(deprecated)]
                        Attribute::ObjectIdArray(read_attribute_array!({ reader.read_uuid()? }))
                    }
                    attribute_type if attribute_type == 21 && version >= VERSION_DEPRECATES_OBJECT_ID => Attribute::TimeArray(read_attribute_array!({
                        Duration::nanoseconds(((reader.read_integer()? as f64 / 10_000.0) * 1_000_000_000.0) as i64)
                    })),
                    22 => Attribute::ColorArray(read_attribute_array!({
                        Color {
                            red: reader.read_unsigned_byte()?,
                            green: reader.read_unsigned_byte()?,
                            blue: reader.read_unsigned_byte()?,
                            alpha: reader.read_unsigned_byte()?,
                        }
                    })),
                    23 => Attribute::Vector2Array(read_attribute_array!({
                        Vector2 {
                            x: reader.read_float()?,
                            y: reader.read_float()?,
                        }
                    })),
                    24 => Attribute::Vector3Array(read_attribute_array!({
                        Vector3 {
                            x: reader.read_float()?,
                            y: reader.read_float()?,
                            z: reader.read_float()?,
                        }
                    })),
                    25 => Attribute::Vector4Array(read_attribute_array!({
                        Vector4 {
                            x: reader.read_float()?,
                            y: reader.read_float()?,
                            z: reader.read_float()?,
                            w: reader.read_float()?,
                        }
                    })),
                    26 => Attribute::AngleArray(read_attribute_array!({
                        Angle {
                            pitch: reader.read_float()?,
                            yaw: reader.read_float()?,
                            roll: reader.read_float()?,
                        }
                    })),
                    27 => Attribute::QuaternionArray(read_attribute_array!({
                        Quaternion {
                            x: reader.read_float()?,
                            y: reader.read_float()?,
                            z: reader.read_float()?,
                            w: reader.read_float()?,
                        }
                    })),
                    28 => Attribute::MatrixArray(read_attribute_array!({
                        Matrix([
                            [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                            [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                            [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                            [reader.read_float()?, reader.read_float()?, reader.read_float()?, reader.read_float()?],
                        ])
                    })),
                    _ => {
                        return Err(BinarySerializationError::InvalidAttributeType {
                            attribute_name,
                            attribute_type,
                        });
                    }
                };
                current_element.set_attribute(attribute_name, attribute_value);
            }
        }

        Ok(element_table.remove(0))
    }
}

struct Writer<T: Write> {
    buffer: T,
}

impl<T: Write> Writer<T> {
    fn new(buffer: T) -> Self {
        Self { buffer }
    }

    fn write_string(&mut self, value: &str) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(value.as_bytes())?;
        self.buffer.write_all(&[0])?;
        Ok(())
    }

    fn write_byte(&mut self, value: i8) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_unsigned_byte(&mut self, value: u8) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_short(&mut self, value: i16) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_integer(&mut self, value: i32) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_float(&mut self, value: f32) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_uuid(&mut self, value: UUID) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_bytes_le())?;
        Ok(())
    }
}

struct Reader<T: BufRead> {
    buffer: T,
}

impl<T: BufRead> Reader<T> {
    fn new(buffer: T) -> Self {
        Self { buffer }
    }

    fn read_string(&mut self) -> Result<String, BinarySerializationError> {
        let mut string_buffer = Vec::new();
        let _ = self.buffer.read_until(0, &mut string_buffer)?;
        string_buffer.pop();
        Ok(String::from_utf8_lossy(&string_buffer).into_owned())
    }

    fn read_byte(&mut self) -> Result<i8, BinarySerializationError> {
        let mut bytes = [0; 1];
        self.buffer.read_exact(&mut bytes)?;
        Ok(i8::from_le_bytes(bytes))
    }

    fn read_unsigned_byte(&mut self) -> Result<u8, BinarySerializationError> {
        let mut bytes = [0; 1];
        self.buffer.read_exact(&mut bytes)?;
        Ok(u8::from_le_bytes(bytes))
    }

    fn read_short(&mut self) -> Result<i16, BinarySerializationError> {
        let mut bytes = [0; 2];
        self.buffer.read_exact(&mut bytes)?;
        Ok(i16::from_le_bytes(bytes))
    }

    fn read_integer(&mut self) -> Result<i32, BinarySerializationError> {
        let mut bytes = [0; 4];
        self.buffer.read_exact(&mut bytes)?;
        Ok(i32::from_le_bytes(bytes))
    }

    fn read_float(&mut self) -> Result<f32, BinarySerializationError> {
        let mut bytes = [0; 4];
        self.buffer.read_exact(&mut bytes)?;
        Ok(f32::from_le_bytes(bytes))
    }

    fn read_uuid(&mut self) -> Result<UUID, BinarySerializationError> {
        let mut bytes = [0; 16];
        self.buffer.read_exact(&mut bytes)?;
        Ok(UUID::from_bytes_le(bytes))
    }
}
