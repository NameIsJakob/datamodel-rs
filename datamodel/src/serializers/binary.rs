use std::{
    io::{BufRead, Error, Write},
    str::FromStr,
};

use indexmap::IndexSet;
use thiserror::Error as ThisError;
use uuid::{Error as UUIDError, Uuid as UUID};

use crate::{
    ElementClass,
    attribute::{Angle, Attribute, AttributeInfo, AttributeType, AttributeValue, BinaryBlock, Color, Matrix, Quaternion, Time, Vector2, Vector3, Vector4},
    element::Element,
    serializing::{Header, Serializer},
};

#[derive(Debug, ThisError)]
pub enum BinarySerializationError {
    #[error("Read Buffer Error: \"{0}\"")]
    BufferError(#[from] Error),
    #[error("Gotten Invalid Version {}: Valid Versions {} - {}", version, 1, BinarySerializer::version())]
    InvalidVersion { version: i32 },
    #[error("Too Many Strings For Table: Found {} Max {}", count, max)]
    TooManyStrings { count: usize, max: usize },
    #[error("Too Many Element For Table: Found {} Max {}", count, MAX_ARRAY_SIZE)]
    TooManyElements { count: usize },
    #[error("Attribute \"name\" In Element \"{}\" Is Not Type String", element.get_id())]
    InvalidNameAttribute { element: Element },
    #[error("Element \"{}\" Has Too Many Attributes: Has {} Max {}", element.get_id(), count, MAX_ARRAY_SIZE)]
    TooManyAttributes { element: Element, count: usize },
    #[error("Attribute \"id\" In Element \"{}\" Can't Be Type ObjectId", element.get_id())]
    InvalidIdAttribute { element: Element },
    #[error("Attribute \"{}\" In Element \"{}\" Data Is Too Long: Has {} Max {}", attribute, element.get_id(), count, MAX_ARRAY_SIZE)]
    BinaryDataTooLong { attribute: String, element: Element, count: usize },
    #[error("Attribute \"{}\" In Element \"{}\" Is A Type That The Version Doesn't Support: Supported Versions {} - {}", attribute, element.get_id(), min, max)]
    InvalidVersionForAttribute { attribute: String, element: Element, min: i32, max: i32 },
    #[error("Attribute \"{}\" In Element \"{}\" Array Is Too Long: Has {} Max {}", attribute, element.get_id(), count, MAX_ARRAY_SIZE)]
    AttributeArrayTooLong { attribute: String, element: Element, count: usize },
    #[error("Deserialize Encoding Is Wrong Encoding")]
    WrongEncoding,
    #[error("Array Length Was Invalid Length")]
    InvalidArraySize,
    #[error("String Table Index Was Invalid")]
    InvalidStringTableIndex,
    #[error("Prefix Element Had Element Attribute Which Is Invalid")]
    InvalidPrefixElementAttribute,
    #[error("Unknown Attribute Id: Got {}", attribute_id)]
    UnknownAttribute { attribute_id: i8 },
    #[error("Invalid Element Table Index: Got {} Size {}", index, size)]
    InvalidElementTableIndex { index: i32, size: usize },
    #[error("Failed To Parse UUID, Error \"{0}\"")]
    UUIDParseError(#[from] UUIDError),
    #[error("No Elements Where Serialized")]
    NoElements,
}

pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    type Error = BinarySerializationError;

    fn name() -> &'static str {
        "binary"
    }

    fn version() -> i32 {
        9
    }

    fn serialize_version(buffer: &mut impl Write, header: &Header, root: &Element, version: i32) -> Result<(), Self::Error> {
        if !(1..=Self::version()).contains(&version) {
            return Err(BinarySerializationError::InvalidVersion { version });
        }

        let mut writer = Writer::new(buffer);
        writer.write_string(&header.create_header(Self::name(), version))?;

        if version >= VERSION_PREFIX_ELEMENT {
            writer.write_integer(0)?;
        }

        let collected_elements = collect_elements(root);
        let collected_strings = collect_strings(&collected_elements, version);

        let max_string_table_length = if version >= VERSION_GLOBAL_STRING_TABLE {
            MAX_ARRAY_SIZE
        } else {
            MAX_SHORT_ARRAY_SIZE
        };
        if collected_strings.len() > max_string_table_length {
            return Err(BinarySerializationError::TooManyStrings {
                count: collected_strings.len(),
                max: max_string_table_length,
            });
        }
        if version >= VERSION_STRING_TABLE {
            if version >= VERSION_GLOBAL_STRING_TABLE {
                writer.write_integer(collected_strings.len() as i32)?;
            } else {
                writer.write_short(collected_strings.len() as i16)?;
            }

            for string in &collected_strings {
                writer.write_string(string)?;
            }
        }

        if collected_elements.len() > MAX_ARRAY_SIZE {
            return Err(BinarySerializationError::TooManyElements {
                count: collected_elements.len(),
            });
        }
        writer.write_integer(collected_elements.len() as i32)?;
        for element in &collected_elements {
            if version >= VERSION_STRING_TABLE {
                writer.write_string_index(element.get_class().as_str(), version, &collected_strings)?;
            } else {
                writer.write_string(element.get_class().as_str())?;
            }

            if (VERSION_LINK_TYPE..VERSION_DEPRECATE_LINK_TYPE).contains(&version) {
                writer.write_integer(-1)?;
            }

            if let Some(element_name_attribute) = element.get_attribute("name") {
                if let AttributeValue::String(element_name) = &*element_name_attribute.get_inner() {
                    if version >= VERSION_GLOBAL_STRING_TABLE {
                        writer.write_string_index(element_name, version, &collected_strings)?;
                    } else {
                        writer.write_string(element_name)?;
                    }
                } else {
                    return Err(BinarySerializationError::InvalidNameAttribute {
                        element: Element::clone(element),
                    });
                }
            } else if version >= VERSION_GLOBAL_STRING_TABLE {
                writer.write_string_index("", version, &collected_strings)?;
            } else {
                writer.write_string("")?;
            }

            writer.write_uuid(*element.get_id())?;
        }

        for element in &collected_elements {
            let element_attributes = element.get_attributes();
            let attribute_count = element_attributes.len() - element_attributes.contains_key("name") as usize;
            if attribute_count > MAX_ARRAY_SIZE {
                return Err(BinarySerializationError::TooManyAttributes {
                    element: Element::clone(element),
                    count: attribute_count,
                });
            }
            writer.write_integer(attribute_count as i32)?;

            for (attribute_name, attribute_value) in element_attributes.iter() {
                if attribute_name == "name" {
                    continue;
                }

                if attribute_name == "id" && attribute_value.get_type() == AttributeType::ObjectId {
                    return Err(BinarySerializationError::InvalidIdAttribute {
                        element: Element::clone(element),
                    });
                }

                if version >= VERSION_STRING_TABLE {
                    writer.write_string_index(attribute_name.as_str(), version, &collected_strings)?;
                } else {
                    writer.write_string(attribute_name.as_str())?;
                }

                fn attribute_array_id(version: i32, attribute_id: i8) -> i8 {
                    attribute_id
                        + if version >= VERSION_UNSIGNED_INTEGERS {
                            ATTRIBUTE_UNSIGNED_INTEGERS_ARRAY_OFFSET
                        } else {
                            ATTRIBUTE_INITIAL_ARRAY_OFFSET
                        }
                }

                fn check_array_length(count: usize, attribute_name: &str, element: &Element) -> Result<(), BinarySerializationError> {
                    if count > MAX_ARRAY_SIZE {
                        return Err(BinarySerializationError::AttributeArrayTooLong {
                            attribute: attribute_name.to_string(),
                            element: Element::clone(element),
                            count,
                        });
                    }
                    Ok(())
                }

                match &*attribute_value.get_inner() {
                    AttributeValue::Element(value) => {
                        writer.write_byte(ATTRIBUTE_ELEMENT_ID)?;
                        let element_value = match value {
                            Some(element_value) => element_value,
                            None => {
                                writer.write_integer(ELEMENT_INDEX_NULL)?;
                                continue;
                            }
                        };
                        writer.write_integer(collected_elements.get_index_of(element_value).unwrap() as i32)?;
                    }
                    AttributeValue::Integer(value) => {
                        writer.write_byte(ATTRIBUTE_INTEGER_ID)?;
                        writer.write_integer(*value)?;
                    }
                    AttributeValue::Float(value) => {
                        writer.write_byte(ATTRIBUTE_FLOAT_ID)?;
                        writer.write_float(*value)?;
                    }
                    AttributeValue::Boolean(value) => {
                        writer.write_byte(ATTRIBUTE_BOOLEAN_ID)?;
                        writer.write_unsigned_byte(*value as u8)?;
                    }
                    AttributeValue::String(value) => {
                        writer.write_byte(ATTRIBUTE_STRING_ID)?;
                        if version >= VERSION_GLOBAL_STRING_TABLE {
                            writer.write_string_index(value, version, &collected_strings)?;
                        } else {
                            writer.write_string(value)?;
                        }
                    }
                    AttributeValue::Binary(value) => {
                        writer.write_byte(ATTRIBUTE_BINARY_ID)?;
                        if value.0.len() > MAX_ARRAY_SIZE {
                            return Err(BinarySerializationError::BinaryDataTooLong {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                count: value.0.len(),
                            });
                        }
                        writer.write_integer(value.0.len() as i32)?;
                        writer.write_unsigned_bytes(&value.0)?;
                    }
                    AttributeValue::ObjectId(value) => {
                        if version >= VERSION_DEPRECATE_OBJECT_ID {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: 1,
                                max: VERSION_STRING_TABLE,
                            });
                        }
                        writer.write_byte(ATTRIBUTE_OBJECTID_ID)?;
                        writer.write_uuid(*value)?;
                    }
                    AttributeValue::Time(value) => {
                        if version < VERSION_DEPRECATE_OBJECT_ID {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: VERSION_DEPRECATE_OBJECT_ID,
                                max: Self::version(),
                            });
                        }
                        writer.write_byte(ATTRIBUTE_TIME_ID)?;
                        writer.write_integer(value.0)?;
                    }
                    AttributeValue::Color(value) => {
                        writer.write_byte(ATTRIBUTE_COLOR_ID)?;
                        writer.write_integer(i32::from_le_bytes([value.red, value.green, value.blue, value.alpha]))?;
                    }
                    AttributeValue::Vector2(value) => {
                        writer.write_byte(ATTRIBUTE_VECTOR2_ID)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                    }
                    AttributeValue::Vector3(value) => {
                        writer.write_byte(ATTRIBUTE_VECTOR3_ID)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                        writer.write_float(value.z)?;
                    }
                    AttributeValue::Vector4(value) => {
                        writer.write_byte(ATTRIBUTE_VECTOR4_ID)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                        writer.write_float(value.z)?;
                        writer.write_float(value.w)?;
                    }
                    AttributeValue::Angle(value) => {
                        writer.write_byte(ATTRIBUTE_ANGLE_ID)?;
                        writer.write_float(value.pitch)?;
                        writer.write_float(value.yaw)?;
                        writer.write_float(value.roll)?;
                    }
                    AttributeValue::Quaternion(value) => {
                        writer.write_byte(ATTRIBUTE_QUATERNION_ID)?;
                        writer.write_float(value.x)?;
                        writer.write_float(value.y)?;
                        writer.write_float(value.z)?;
                        writer.write_float(value.w)?;
                    }
                    AttributeValue::Matrix(value) => {
                        writer.write_byte(ATTRIBUTE_MATRIX_ID)?;
                        let bytes = value.0.iter().flatten().flat_map(|entry| entry.to_le_bytes()).collect::<Vec<u8>>();
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::ULong(value) => {
                        if version < VERSION_UNSIGNED_INTEGERS {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: VERSION_UNSIGNED_INTEGERS,
                                max: Self::version(),
                            });
                        }
                        writer.write_byte(ATTRIBUTE_ULONG_ID)?;
                        writer.write_unsigned_long(*value)?;
                    }
                    AttributeValue::UByte(value) => {
                        if version < VERSION_UNSIGNED_INTEGERS {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: VERSION_UNSIGNED_INTEGERS,
                                max: Self::version(),
                            });
                        }
                        writer.write_byte(ATTRIBUTE_UBYTE_ID)?;
                        writer.write_unsigned_byte(*value)?;
                    }
                    AttributeValue::ElementArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_ELEMENT_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let bytes = values
                            .iter()
                            .flat_map(|value| match value {
                                Some(element) => (collected_elements.get_index_of(element).unwrap() as i32).to_le_bytes(),
                                None => (-1i32).to_le_bytes(),
                            })
                            .collect::<Vec<u8>>();
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::IntegerArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_INTEGER_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let bytes = values.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>();
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::FloatArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_FLOAT_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let bytes = values.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>();
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::BooleanArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_BOOLEAN_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let bytes = values.iter().map(|value| *value as u8).collect::<Vec<u8>>();
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::StringArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_STRING_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            writer.write_string(value)?;
                        }
                    }
                    AttributeValue::BinaryArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_BINARY_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        for value in values {
                            if value.0.len() > MAX_ARRAY_SIZE {
                                return Err(BinarySerializationError::BinaryDataTooLong {
                                    attribute: attribute_name.clone(),
                                    element: Element::clone(element),
                                    count: value.0.len(),
                                });
                            }
                            writer.write_integer(value.0.len() as i32)?;
                            writer.write_unsigned_bytes(&value.0)?;
                        }
                    }
                    AttributeValue::ObjectIdArray(values) => {
                        if version >= VERSION_DEPRECATE_OBJECT_ID {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: 1,
                                max: VERSION_STRING_TABLE,
                            });
                        }
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_OBJECTID_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<UUID>());
                        for value in values {
                            bytes.extend(value.to_bytes_le());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::TimeArray(values) => {
                        if version < VERSION_DEPRECATE_OBJECT_ID {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: VERSION_DEPRECATE_OBJECT_ID,
                                max: Self::version(),
                            });
                        }
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_TIME_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Time>());
                        for value in values {
                            bytes.extend(value.0.to_le_bytes());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::ColorArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_COLOR_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Color>());
                        for value in values {
                            bytes.push(value.red);
                            bytes.push(value.green);
                            bytes.push(value.blue);
                            bytes.push(value.alpha);
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::Vector2Array(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_VECTOR2_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Vector2>());
                        for value in values {
                            bytes.extend(value.x.to_le_bytes());
                            bytes.extend(value.y.to_le_bytes());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::Vector3Array(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_VECTOR3_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Vector3>());
                        for value in values {
                            bytes.extend(value.x.to_le_bytes());
                            bytes.extend(value.y.to_le_bytes());
                            bytes.extend(value.z.to_le_bytes());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::Vector4Array(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_VECTOR4_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Vector4>());
                        for value in values {
                            bytes.extend(value.x.to_le_bytes());
                            bytes.extend(value.y.to_le_bytes());
                            bytes.extend(value.z.to_le_bytes());
                            bytes.extend(value.w.to_le_bytes());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::AngleArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_ANGLE_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Angle>());
                        for value in values {
                            bytes.extend(value.pitch.to_le_bytes());
                            bytes.extend(value.yaw.to_le_bytes());
                            bytes.extend(value.roll.to_le_bytes());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::QuaternionArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_QUATERNION_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Quaternion>());
                        for value in values {
                            bytes.extend(value.x.to_le_bytes());
                            bytes.extend(value.y.to_le_bytes());
                            bytes.extend(value.z.to_le_bytes());
                            bytes.extend(value.w.to_le_bytes());
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::MatrixArray(values) => {
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_MATRIX_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let mut bytes: Vec<u8> = Vec::with_capacity(values.len() * size_of::<Matrix>());
                        for value in values {
                            for entry in value.0.iter().flatten() {
                                bytes.extend(entry.to_le_bytes());
                            }
                        }
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::ULongArray(values) => {
                        if version < VERSION_UNSIGNED_INTEGERS {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: VERSION_UNSIGNED_INTEGERS,
                                max: Self::version(),
                            });
                        }
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_ULONG_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        let bytes = values.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>();
                        writer.write_unsigned_bytes(&bytes)?;
                    }
                    AttributeValue::UByteArray(values) => {
                        if version < VERSION_UNSIGNED_INTEGERS {
                            return Err(BinarySerializationError::InvalidVersionForAttribute {
                                attribute: attribute_name.clone(),
                                element: Element::clone(element),
                                min: VERSION_UNSIGNED_INTEGERS,
                                max: Self::version(),
                            });
                        }
                        writer.write_byte(attribute_array_id(version, ATTRIBUTE_UBYTE_ID))?;
                        check_array_length(values.len(), attribute_name, element)?;
                        writer.write_integer(values.len() as i32)?;
                        writer.write_unsigned_bytes(values)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if !(1..=Self::version()).contains(&version) {
            return Err(BinarySerializationError::InvalidVersion { version });
        }

        if encoding != Self::name() {
            return Err(BinarySerializationError::WrongEncoding);
        }

        let mut reader = Reader::new(buffer);
        reader.read_string()?;

        if version >= VERSION_PREFIX_ELEMENT && reader.read_integer()? != 0 {
            let attribute_count = array_size_check(reader.read_integer()?)?;
            for _ in 0..attribute_count {
                reader.read_string()?;
                let attribute_type = reader.read_byte()?;
                if attribute_type == ATTRIBUTE_ELEMENT_ID
                    || (version < VERSION_UNSIGNED_INTEGERS && attribute_type == ATTRIBUTE_ELEMENT_ID + ATTRIBUTE_INITIAL_ARRAY_OFFSET)
                    || (version >= VERSION_UNSIGNED_INTEGERS && attribute_type == ATTRIBUTE_ELEMENT_ID + ATTRIBUTE_UNSIGNED_INTEGERS_ARRAY_OFFSET)
                {
                    return Err(BinarySerializationError::InvalidPrefixElementAttribute);
                }
                if attribute_type == ATTRIBUTE_STRING_ID {
                    reader.read_string()?;
                    continue;
                }
                reader.read_attribute(version, attribute_type)?;
            }
        }

        let string_table_size = if version >= VERSION_GLOBAL_STRING_TABLE {
            array_size_check(reader.read_integer()?)?
        } else if version >= VERSION_STRING_TABLE {
            array_size_check(reader.read_short()? as i32)?
        } else {
            0
        };
        let mut string_table = Vec::with_capacity(string_table_size);
        for _ in 0..string_table_size {
            string_table.push(reader.read_string()?);
        }

        let element_size = array_size_check(reader.read_integer()?)?;
        let mut elements = Vec::with_capacity(element_size);
        for _ in 0..element_size {
            let element_class = if version >= VERSION_LARGE_STRING_INDEX {
                get_string_table_index(reader.read_integer()?, &string_table)?
            } else if version >= VERSION_STRING_TABLE {
                get_string_table_index(reader.read_short()? as i32, &string_table)?
            } else {
                reader.read_string()?
            };

            if (VERSION_LINK_TYPE..VERSION_DEPRECATE_LINK_TYPE).contains(&version) {
                reader.read_integer()?;
            }

            let element_name = if version >= VERSION_LARGE_STRING_INDEX {
                get_string_table_index(reader.read_integer()?, &string_table)?
            } else if version >= VERSION_GLOBAL_STRING_TABLE {
                get_string_table_index(reader.read_short()? as i32, &string_table)?
            } else {
                reader.read_string()?
            };

            let element_id = reader.read_uuid()?;
            let mut new_element = Element::full(element_class, element_id);
            new_element.set_attribute("name", element_name.into_attribute());
            elements.push(new_element);
        }

        for element_index in 0..element_size {
            let attribute_count = array_size_check(reader.read_integer()?)?;
            let mut current_element = Element::clone(&elements[element_index]);
            for _ in 0..attribute_count {
                let attribute_name = if version >= VERSION_LARGE_STRING_INDEX {
                    get_string_table_index(reader.read_integer()?, &string_table)?
                } else if version >= VERSION_STRING_TABLE {
                    get_string_table_index(reader.read_short()? as i32, &string_table)?
                } else {
                    reader.read_string()?
                };
                let attribute_type = reader.read_byte()?;
                let attribute_value = if attribute_type == ATTRIBUTE_ELEMENT_ID {
                    (match reader.read_integer()? {
                        index if index < ELEMENT_INDEX_EXTERNAL || index > element_size as i32 => {
                            return Err(BinarySerializationError::InvalidElementTableIndex { index, size: element_size });
                        }
                        ELEMENT_INDEX_NULL => None,
                        ELEMENT_INDEX_EXTERNAL => Some(Element::full(Element::class_name(), UUID::from_str(&reader.read_string()?)?)),
                        index => Some(Element::clone(&elements[index as usize])),
                    })
                    .into_attribute()
                } else if (version < VERSION_UNSIGNED_INTEGERS && attribute_type == ATTRIBUTE_ELEMENT_ID + ATTRIBUTE_INITIAL_ARRAY_OFFSET)
                    || (version >= VERSION_UNSIGNED_INTEGERS && attribute_type == ATTRIBUTE_ELEMENT_ID + ATTRIBUTE_UNSIGNED_INTEGERS_ARRAY_OFFSET)
                {
                    let array_size = array_size_check(reader.read_integer()?)?;
                    let mut attribute_array = Vec::with_capacity(array_size);
                    for _ in 0..array_size {
                        attribute_array.push(match reader.read_integer()? {
                            index if index < ELEMENT_INDEX_EXTERNAL || index > element_size as i32 => {
                                return Err(BinarySerializationError::InvalidElementTableIndex { index, size: element_size });
                            }
                            ELEMENT_INDEX_NULL => None,
                            ELEMENT_INDEX_EXTERNAL => Some(Element::full(Element::class_name(), UUID::from_str(&reader.read_string()?)?)),
                            index => Some(Element::clone(&elements[index as usize])),
                        });
                    }
                    attribute_array.into_attribute()
                } else if attribute_type == ATTRIBUTE_STRING_ID {
                    (if version >= VERSION_LARGE_STRING_INDEX {
                        get_string_table_index(reader.read_integer()?, &string_table)?
                    } else if version >= VERSION_GLOBAL_STRING_TABLE {
                        get_string_table_index(reader.read_short()? as i32, &string_table)?
                    } else {
                        reader.read_string()?
                    })
                    .into_attribute()
                } else {
                    reader.read_attribute(version, attribute_type)?
                };
                current_element.set_attribute(attribute_name, attribute_value);
            }
        }

        if elements.is_empty() {
            return Err(BinarySerializationError::NoElements);
        }

        Ok(elements.remove(0))
    }
}

const VERSION_STRING_TABLE: i32 = 2;
const VERSION_DEPRECATE_OBJECT_ID: i32 = 3;
const VERSION_GLOBAL_STRING_TABLE: i32 = 4;
const VERSION_LARGE_STRING_INDEX: i32 = 5;
const VERSION_LINK_TYPE: i32 = 6;
const VERSION_PREFIX_ELEMENT: i32 = 7;
const VERSION_DEPRECATE_LINK_TYPE: i32 = 8;
const VERSION_UNSIGNED_INTEGERS: i32 = 9;

const MAX_SHORT_ARRAY_SIZE: usize = (i16::MAX as usize) + 1;
const MAX_ARRAY_SIZE: usize = (i32::MAX as usize) + 1;

const ATTRIBUTE_ELEMENT_ID: i8 = 1;
const ATTRIBUTE_INTEGER_ID: i8 = 2;
const ATTRIBUTE_FLOAT_ID: i8 = 3;
const ATTRIBUTE_BOOLEAN_ID: i8 = 4;
const ATTRIBUTE_STRING_ID: i8 = 5;
const ATTRIBUTE_BINARY_ID: i8 = 6;
const ATTRIBUTE_OBJECTID_ID: i8 = 7;
const ATTRIBUTE_TIME_ID: i8 = 7;
const ATTRIBUTE_COLOR_ID: i8 = 8;
const ATTRIBUTE_VECTOR2_ID: i8 = 9;
const ATTRIBUTE_VECTOR3_ID: i8 = 10;
const ATTRIBUTE_VECTOR4_ID: i8 = 11;
const ATTRIBUTE_ANGLE_ID: i8 = 12;
const ATTRIBUTE_QUATERNION_ID: i8 = 13;
const ATTRIBUTE_MATRIX_ID: i8 = 14;
const ATTRIBUTE_ULONG_ID: i8 = 15;
const ATTRIBUTE_UBYTE_ID: i8 = 16;

const ATTRIBUTE_INITIAL_ARRAY_OFFSET: i8 = 14;
const ATTRIBUTE_UNSIGNED_INTEGERS_ARRAY_OFFSET: i8 = 32;

const ELEMENT_INDEX_NULL: i32 = -1;
const ELEMENT_INDEX_EXTERNAL: i32 = -2;

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

    fn write_string_index(&mut self, value: &str, version: i32, collected_strings: &IndexSet<String>) -> Result<(), BinarySerializationError> {
        if version >= VERSION_LARGE_STRING_INDEX {
            if value.is_empty() {
                return self.write_integer(-1);
            }

            return self.write_integer(collected_strings.get_index_of(value).unwrap() as i32);
        }

        if value.is_empty() {
            return self.write_short(-1);
        }
        self.write_short(collected_strings.get_index_of(value).unwrap() as i16)
    }

    fn write_byte(&mut self, value: i8) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_unsigned_byte(&mut self, value: u8) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_unsigned_bytes(&mut self, value: &[u8]) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(value)?;
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

    fn write_unsigned_long(&mut self, value: u64) -> Result<(), BinarySerializationError> {
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

fn collect_elements(root: &Element) -> IndexSet<Element> {
    let mut collected_elements = IndexSet::new();
    let mut collection_stack = Vec::new();
    collected_elements.insert(Element::clone(root));
    collection_stack.push(Element::clone(root));

    while let Some(collecting_element) = collection_stack.pop() {
        for attribute in collecting_element.get_attributes().values() {
            match &*attribute.get_inner() {
                AttributeValue::Element(value) => {
                    if let Some(element) = value
                        && collected_elements.insert(Element::clone(element))
                    {
                        collection_stack.push(Element::clone(element));
                    }
                }
                AttributeValue::ElementArray(values) => {
                    values.iter().flatten().for_each(|value| {
                        if collected_elements.insert(Element::clone(value)) {
                            collection_stack.push(Element::clone(value));
                        }
                    });
                }
                _ => {}
            }
        }
    }

    collected_elements
}

fn collect_strings(collected_elements: &IndexSet<Element>, version: i32) -> IndexSet<String> {
    if version < VERSION_STRING_TABLE {
        return IndexSet::new();
    }

    let mut collected_strings = IndexSet::new();
    for element in collected_elements {
        collected_strings.insert(element.get_class().clone());

        for (attribute_name, attribute_value) in element.get_attributes().iter() {
            collected_strings.insert(attribute_name.clone());
            if version >= VERSION_GLOBAL_STRING_TABLE
                && let AttributeValue::String(value) = &*attribute_value.get_inner()
            {
                collected_strings.insert(value.clone());
            }
        }
    }
    collected_strings.swap_remove("");

    collected_strings
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

    fn read_unsigned_bytes(&mut self, size: usize) -> Result<Vec<u8>, BinarySerializationError> {
        let mut bytes = vec![0; size];
        self.buffer.read_exact(&mut bytes)?;
        Ok(bytes)
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

    fn read_unsigned_long(&mut self) -> Result<u64, BinarySerializationError> {
        let mut bytes = [0; 8];
        self.buffer.read_exact(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
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

    fn read_attribute(&mut self, version: i32, attribute_type: i8) -> Result<Attribute, BinarySerializationError> {
        if version >= VERSION_UNSIGNED_INTEGERS {
            if attribute_type <= ATTRIBUTE_UNSIGNED_INTEGERS_ARRAY_OFFSET {
                return self.read_single_attribute(version, attribute_type);
            }

            let array_type = attribute_type - ATTRIBUTE_UNSIGNED_INTEGERS_ARRAY_OFFSET;
            let array_size = array_size_check(self.read_integer()?)?;
            return self.read_array_attribute(version, array_type, array_size);
        }

        if attribute_type <= ATTRIBUTE_INITIAL_ARRAY_OFFSET {
            return self.read_single_attribute(version, attribute_type);
        }

        let array_type = attribute_type - ATTRIBUTE_INITIAL_ARRAY_OFFSET;
        let array_size = array_size_check(self.read_integer()?)?;
        self.read_array_attribute(version, array_type, array_size)
    }

    fn read_single_attribute(&mut self, version: i32, attribute_type: i8) -> Result<Attribute, BinarySerializationError> {
        match attribute_type {
            ATTRIBUTE_INTEGER_ID => Ok(self.read_integer()?.into_attribute()),
            ATTRIBUTE_FLOAT_ID => Ok(self.read_float()?.into_attribute()),
            ATTRIBUTE_BOOLEAN_ID => Ok((self.read_unsigned_byte()? != 0).into_attribute()),
            ATTRIBUTE_BINARY_ID => {
                let data_size = array_size_check(self.read_integer()?)?;
                Ok((BinaryBlock(self.read_unsigned_bytes(data_size)?)).into_attribute())
            }
            ATTRIBUTE_OBJECTID_ID if version < VERSION_DEPRECATE_OBJECT_ID => Ok(self.read_uuid()?.into_attribute()),
            ATTRIBUTE_TIME_ID if version >= VERSION_DEPRECATE_OBJECT_ID => Ok(Time(self.read_integer()?).into_attribute()),
            ATTRIBUTE_COLOR_ID => Ok(Color {
                red: self.read_unsigned_byte()?,
                green: self.read_unsigned_byte()?,
                blue: self.read_unsigned_byte()?,
                alpha: self.read_unsigned_byte()?,
            }
            .into_attribute()),
            ATTRIBUTE_VECTOR2_ID => Ok(Vector2 {
                x: self.read_float()?,
                y: self.read_float()?,
            }
            .into_attribute()),
            ATTRIBUTE_VECTOR3_ID => Ok(Vector3 {
                x: self.read_float()?,
                y: self.read_float()?,
                z: self.read_float()?,
            }
            .into_attribute()),
            ATTRIBUTE_VECTOR4_ID => Ok(Vector4 {
                x: self.read_float()?,
                y: self.read_float()?,
                z: self.read_float()?,
                w: self.read_float()?,
            }
            .into_attribute()),
            ATTRIBUTE_ANGLE_ID => Ok(Angle {
                pitch: self.read_float()?,
                yaw: self.read_float()?,
                roll: self.read_float()?,
            }
            .into_attribute()),
            ATTRIBUTE_QUATERNION_ID => Ok(Quaternion {
                x: self.read_float()?,
                y: self.read_float()?,
                z: self.read_float()?,
                w: self.read_float()?,
            }
            .into_attribute()),
            ATTRIBUTE_MATRIX_ID => Ok(Matrix([
                [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
            ])
            .into_attribute()),
            ATTRIBUTE_ULONG_ID if version >= VERSION_UNSIGNED_INTEGERS => Ok(self.read_unsigned_long()?.into_attribute()),
            ATTRIBUTE_UBYTE_ID if version >= VERSION_UNSIGNED_INTEGERS => Ok(self.read_unsigned_byte()?.into_attribute()),
            _ => Err(BinarySerializationError::UnknownAttribute { attribute_id: attribute_type }),
        }
    }

    fn read_array_attribute(&mut self, version: i32, attribute_type: i8, size: usize) -> Result<Attribute, BinarySerializationError> {
        match attribute_type {
            ATTRIBUTE_INTEGER_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(self.read_integer()?);
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_FLOAT_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(self.read_float()?);
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_BOOLEAN_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(self.read_unsigned_byte()? != 0);
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_STRING_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(self.read_string()?);
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_BINARY_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    let data_size = array_size_check(self.read_integer()?)?;
                    attribute_array.push(BinaryBlock(self.read_unsigned_bytes(data_size)?));
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_OBJECTID_ID if version < VERSION_DEPRECATE_OBJECT_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(self.read_uuid()?);
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_TIME_ID if version >= VERSION_DEPRECATE_OBJECT_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Time(self.read_integer()?));
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_COLOR_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Color {
                        red: self.read_unsigned_byte()?,
                        green: self.read_unsigned_byte()?,
                        blue: self.read_unsigned_byte()?,
                        alpha: self.read_unsigned_byte()?,
                    });
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_VECTOR2_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Vector2 {
                        x: self.read_float()?,
                        y: self.read_float()?,
                    });
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_VECTOR3_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Vector3 {
                        x: self.read_float()?,
                        y: self.read_float()?,
                        z: self.read_float()?,
                    });
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_VECTOR4_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Vector4 {
                        x: self.read_float()?,
                        y: self.read_float()?,
                        z: self.read_float()?,
                        w: self.read_float()?,
                    });
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_ANGLE_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Angle {
                        pitch: self.read_float()?,
                        yaw: self.read_float()?,
                        roll: self.read_float()?,
                    });
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_QUATERNION_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Quaternion {
                        x: self.read_float()?,
                        y: self.read_float()?,
                        z: self.read_float()?,
                        w: self.read_float()?,
                    });
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_MATRIX_ID => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(Matrix([
                        [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                        [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                        [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                        [self.read_float()?, self.read_float()?, self.read_float()?, self.read_float()?],
                    ]));
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_ULONG_ID if version >= VERSION_UNSIGNED_INTEGERS => {
                let mut attribute_array = Vec::with_capacity(size);
                for _ in 0..size {
                    attribute_array.push(self.read_unsigned_long()?);
                }
                Ok(attribute_array.into_attribute())
            }
            ATTRIBUTE_UBYTE_ID if version >= VERSION_UNSIGNED_INTEGERS => Ok(self.read_unsigned_bytes(size)?.into_attribute()),
            _ => Err(BinarySerializationError::UnknownAttribute { attribute_id: attribute_type }),
        }
    }
}

fn array_size_check(size: i32) -> Result<usize, BinarySerializationError> {
    if size < 0 {
        return Err(BinarySerializationError::InvalidArraySize);
    }
    Ok(size as usize)
}

fn get_string_table_index(index: i32, table: &[String]) -> Result<String, BinarySerializationError> {
    if index == -1 {
        return Ok(String::new());
    }
    if index < 0 || index as usize >= table.len() {
        return Err(BinarySerializationError::InvalidStringTableIndex);
    }
    Ok(table[index as usize].clone())
}
