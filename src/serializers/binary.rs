use std::{
    alloc::{alloc, Layout},
    io::{BufRead, Error, Write},
    mem::{align_of, size_of},
    ptr::read as read_aligned,
    time::Duration,
};

use indexmap::IndexSet;
use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

use crate::{attribute::BinaryBlock, Attribute, Element, Header, Serializer};

#[derive(Debug, ThisError)]
pub enum BinarySerializationError {
    #[error("IO Error: {0}")]
    Io(#[from] Error),
    #[error("To Many Elements To Serialize")]
    TooManyElements,
    #[error("To Many Strings To Serialize")]
    TooManyStrings,
    #[error("Element Has Too Many Attributes To Serialize")]
    TooManyAttributes,
    #[error("Attribute Binary Data Too Long")]
    BinaryDataTooLong,
    #[error("Attribute Array Too Long")]
    AttributeArrayTooLong,
    #[error("Header Serializer Version Is Different")]
    InvalidEncodingVersion,
    #[error("Can't Serialize Deprecated Attribute")]
    DeprecatedAttribute,
    #[error("Header Serializer Is Different")]
    WrongEncoding,
    #[error("Invalid String Index")]
    InvalidStringIndex,
    #[error("Element Index Was Invalid")]
    MissingElement,
    #[error("Attributes Type Number Not Valid Attribute")]
    InvalidAttributeType,
}

struct BinaryWriter<T: Write> {
    buffer: T,
    string_table: IndexSet<String>,
}

impl<T: Write> BinaryWriter<T> {
    fn new(buffer: T) -> Self {
        Self {
            buffer,
            string_table: IndexSet::new(),
        }
    }

    fn write_byte(&mut self, value: i8) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(value)?;
        Ok(())
    }

    fn write_int(&mut self, value: i32) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_length(&mut self, value: usize) -> Result<(), BinarySerializationError> {
        if value > i32::MAX as usize {
            return Err(BinarySerializationError::AttributeArrayTooLong);
        }
        self.write_int(value as i32)
    }

    fn write_float(&mut self, value: f32) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    fn write_uuid(&mut self, value: UUID) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(value.as_bytes())?;
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), BinarySerializationError> {
        self.buffer.write_all(value.as_bytes())?;
        self.buffer.write_all(&[0])?;
        Ok(())
    }

    fn add_sting_to_table(&mut self, value: &str) {
        self.string_table.insert(value.to_string());
    }

    fn write_string_table(&mut self) -> Result<(), BinarySerializationError> {
        if self.string_table.len() > i32::MAX as usize {
            return Err(BinarySerializationError::TooManyStrings);
        }

        self.write_int(self.string_table.len() as i32)?;

        for string in &self.string_table {
            self.buffer.write_all(string.as_bytes())?;
            self.buffer.write_all(&[0])?;
        }

        Ok(())
    }

    fn write_string_index(&mut self, value: &str) -> Result<(), BinarySerializationError> {
        let index = self.string_table.get_index_of(value).unwrap() as i32;
        self.write_int(index)
    }
}

struct BinaryReader<T: BufRead> {
    buffer: T,
    string_table: Vec<String>,
}

impl<T: BufRead> BinaryReader<T> {
    fn new(buffer: T) -> Self {
        Self {
            buffer,
            string_table: Vec::new(),
        }
    }

    fn read_string(&mut self) -> Result<String, BinarySerializationError> {
        let mut string_buffer = Vec::new();
        let _ = self.buffer.read_until(0, &mut string_buffer)?;
        string_buffer.pop();
        let string = String::from_utf8_lossy(&string_buffer).into_owned();
        Ok(string)
    }

    fn read_string_array(&mut self, size: i32) -> Result<Vec<String>, BinarySerializationError> {
        let mut strings = Vec::with_capacity(size as usize);

        for _ in 0..size {
            strings.push(self.read_string()?)
        }

        Ok(strings)
    }

    fn read_string_table(&mut self, version: i32) -> Result<(), BinarySerializationError> {
        if version < 2 {
            return Ok(());
        }

        let string_table_count = if version >= 4 { self.read()? } else { self.read::<i16>()? as i32 };

        self.string_table = self.read_string_array(string_table_count)?;

        Ok(())
    }

    fn get_string(&mut self, version: i32) -> Result<String, BinarySerializationError> {
        if version < 2 {
            return self.read_string();
        }

        let string_index = if version >= 5 { self.read()? } else { self.read::<i16>()? as i32 };

        if string_index == -1 {
            return Ok(String::from("unnamed"));
        }

        let string = self
            .string_table
            .get(string_index as usize)
            .ok_or(BinarySerializationError::InvalidStringIndex)?;
        Ok(string.clone())
    }

    fn read_uuid(&mut self) -> Result<UUID, BinarySerializationError> {
        let mut buffer = [0; 16];
        self.buffer.read_exact(&mut buffer)?;
        let value = UUID::from_bytes_le(buffer);
        Ok(value)
    }

    fn read<V>(&mut self) -> Result<V, BinarySerializationError> {
        let size = size_of::<V>();
        let mut buffer = vec![0; size];
        self.buffer.read_exact(&mut buffer)?;
        let value = unsafe { read_aligned(buffer.as_ptr() as *const V) };
        Ok(value)
    }

    fn read_array<V>(&mut self, length: i32) -> Result<Vec<V>, BinarySerializationError> {
        let size = size_of::<V>();
        let align = align_of::<V>();
        let layout = Layout::from_size_align(size * length as usize, align).unwrap();
        let buffer_ptr = unsafe { alloc(layout) };
        unsafe {
            let buffer_slice = std::slice::from_raw_parts_mut(buffer_ptr, size * length as usize);
            self.buffer.read_exact(buffer_slice)?;
        }
        let value = unsafe {
            let ptr = buffer_ptr as *mut V;
            Vec::from_raw_parts(ptr, length as usize, length as usize)
        };
        Ok(value)
    }
}

pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    type Error = BinarySerializationError;

    fn name() -> &'static str {
        "binary"
    }

    fn version() -> i32 {
        5
    }

    fn serialize(buffer: &mut impl Write, header: &Header, root: &Element) -> Result<(), Self::Error> {
        let mut writer = BinaryWriter::new(buffer);
        writer.write_string(&header.create_header(Self::name(), Self::version()))?;

        fn collect_elements(root: Element, elements: &mut IndexSet<Element>) {
            elements.insert(root.clone());

            for attribute in root.get_attributes().values() {
                match attribute {
                    Attribute::Element(value) => match value {
                        Some(element) => {
                            if !elements.insert(element.clone()) {
                                continue;
                            }
                            collect_elements(element.clone(), elements)
                        }
                        None => continue,
                    },
                    Attribute::ElementArray(values) => {
                        for value in values {
                            match value {
                                Some(element) => {
                                    if !elements.insert(element.clone()) {
                                        continue;
                                    }
                                    collect_elements(element.clone(), elements)
                                }
                                None => continue,
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut collected_elements = IndexSet::new();
        collect_elements(root.clone(), &mut collected_elements);

        if collected_elements.len() > i32::MAX as usize {
            return Err(BinarySerializationError::TooManyElements);
        }

        for element in &collected_elements {
            writer.add_sting_to_table(&element.get_class());
            writer.add_sting_to_table(&element.get_name());

            for (name, attribute) in element.get_attributes().iter() {
                writer.add_sting_to_table(name);

                if let Attribute::String(value) = attribute {
                    writer.add_sting_to_table(value);
                }
            }
        }

        writer.write_string_table()?;

        writer.write_int(collected_elements.len() as i32)?;

        for element in &collected_elements {
            writer.write_string_index(&element.get_class())?;
            writer.write_string_index(&element.get_name())?;
            writer.write_uuid(UUID::new_v4())?;
        }

        for element in &collected_elements {
            let attributes = element.get_attributes();
            if attributes.len() > i32::MAX as usize {
                return Err(BinarySerializationError::TooManyAttributes);
            }

            writer.write_int(attributes.len() as i32)?;

            for (name, attribute) in attributes.iter() {
                writer.write_string_index(name)?;

                match attribute {
                    Attribute::Element(value) => {
                        writer.write_byte(1)?;

                        let element_value = match value {
                            Some(element_value) => element_value,
                            None => {
                                writer.write_int(-1)?;
                                continue;
                            }
                        };

                        let index = collected_elements.get_index_of(element_value).unwrap();

                        writer.write_int(index as i32)?;
                    }
                    Attribute::Integer(value) => {
                        writer.write_byte(2)?;
                        writer.write_int(*value)?;
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

                        writer.write_string_index(value)?;
                    }
                    Attribute::Binary(value) => {
                        writer.write_byte(6)?;
                        if value.data.len() > i32::MAX as usize {
                            return Err(BinarySerializationError::BinaryDataTooLong);
                        }
                        writer.write_int(value.data.len() as i32)?;
                        writer.write_bytes(value.data.as_slice())?;
                    }
                    Attribute::ObjectId(_) => {
                        return Err(BinarySerializationError::DeprecatedAttribute);
                    }
                    Attribute::Time(value) => {
                        writer.write_byte(7)?;
                        writer.write_int((value.as_secs_f64() * 10_000f64) as i32)?;
                    }
                    Attribute::Color(value) => {
                        writer.write_byte(8)?;
                        writer.write_bytes(&[value.red, value.green, value.blue, value.alpha])?;
                    }
                    Attribute::Vector2(value) => {
                        writer.write_byte(9)?;
                        writer.write_bytes([value.x.to_le_bytes(), value.y.to_le_bytes()].concat().as_slice())?;
                    }
                    Attribute::Vector3(value) => {
                        writer.write_byte(10)?;
                        writer.write_bytes([value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes()].concat().as_slice())?;
                    }
                    Attribute::Vector4(value) => {
                        writer.write_byte(11)?;
                        writer.write_bytes(
                            [value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes(), value.w.to_le_bytes()]
                                .concat()
                                .as_slice(),
                        )?;
                    }
                    Attribute::Angle(value) => {
                        writer.write_byte(12)?;
                        writer.write_bytes(
                            [value.pitch.to_le_bytes(), value.yaw.to_le_bytes(), value.roll.to_le_bytes()]
                                .concat()
                                .as_slice(),
                        )?;
                    }
                    Attribute::Quaternion(value) => {
                        writer.write_byte(13)?;
                        writer.write_bytes(
                            [value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes(), value.w.to_le_bytes()]
                                .concat()
                                .as_slice(),
                        )?;
                    }
                    Attribute::Matrix(value) => {
                        writer.write_byte(14)?;
                        writer.write_bytes(
                            [
                                value.entries[0][0].to_le_bytes(),
                                value.entries[0][1].to_le_bytes(),
                                value.entries[0][2].to_le_bytes(),
                                value.entries[0][3].to_le_bytes(),
                                value.entries[1][0].to_le_bytes(),
                                value.entries[1][1].to_le_bytes(),
                                value.entries[1][2].to_le_bytes(),
                                value.entries[1][3].to_le_bytes(),
                                value.entries[2][0].to_le_bytes(),
                                value.entries[2][1].to_le_bytes(),
                                value.entries[2][2].to_le_bytes(),
                                value.entries[2][3].to_le_bytes(),
                                value.entries[3][0].to_le_bytes(),
                                value.entries[3][1].to_le_bytes(),
                                value.entries[3][2].to_le_bytes(),
                                value.entries[3][3].to_le_bytes(),
                            ]
                            .concat()
                            .as_slice(),
                        )?;
                    }
                    Attribute::ElementArray(value) => {
                        writer.write_byte(15)?;
                        writer.write_length(value.len())?;

                        for element in value {
                            let element_value = match element {
                                Some(element_value) => element_value,
                                None => {
                                    writer.write_int(-1)?;
                                    continue;
                                }
                            };

                            let index = collected_elements.get_index_of(element_value).unwrap();

                            writer.write_int(index as i32)?;
                        }
                    }
                    Attribute::IntegerArray(value) => {
                        writer.write_byte(16)?;
                        writer.write_length(value.len())?;
                        for integer in value {
                            writer.write_int(*integer)?;
                        }
                    }
                    Attribute::FloatArray(value) => {
                        writer.write_byte(17)?;
                        writer.write_length(value.len())?;
                        for float in value {
                            writer.write_float(*float)?;
                        }
                    }
                    Attribute::BooleanArray(value) => {
                        writer.write_byte(18)?;
                        writer.write_length(value.len())?;
                        for boolean in value {
                            writer.write_byte(*boolean as i8)?;
                        }
                    }
                    Attribute::StringArray(value) => {
                        writer.write_byte(19)?;
                        writer.write_length(value.len())?;
                        for string in value {
                            writer.write_string(string)?;
                        }
                    }
                    Attribute::BinaryArray(value) => {
                        writer.write_byte(20)?;
                        writer.write_length(value.len())?;
                        for binary in value {
                            if binary.data.len() > i32::MAX as usize {
                                return Err(BinarySerializationError::BinaryDataTooLong);
                            }
                            writer.write_int(binary.data.len() as i32)?;
                            writer.write_bytes(binary.data.as_slice())?;
                        }
                    }
                    Attribute::ObjectIdArray(_) => {
                        return Err(BinarySerializationError::DeprecatedAttribute);
                    }
                    Attribute::TimeArray(value) => {
                        writer.write_byte(21)?;
                        writer.write_length(value.len())?;
                        for time in value {
                            writer.write_int((time.as_secs_f64() * 10_000f64) as i32)?;
                        }
                    }
                    Attribute::ColorArray(value) => {
                        writer.write_byte(22)?;
                        writer.write_length(value.len())?;
                        for color in value {
                            writer.write_bytes(&[color.red, color.green, color.blue, color.alpha])?;
                        }
                    }
                    Attribute::Vector2Array(value) => {
                        writer.write_byte(23)?;
                        writer.write_length(value.len())?;
                        for vector in value {
                            writer.write_bytes([vector.x.to_le_bytes(), vector.y.to_le_bytes()].concat().as_slice())?;
                        }
                    }
                    Attribute::Vector3Array(value) => {
                        writer.write_byte(24)?;
                        writer.write_length(value.len())?;
                        for vector in value {
                            writer.write_bytes([vector.x.to_le_bytes(), vector.y.to_le_bytes(), vector.z.to_le_bytes()].concat().as_slice())?;
                        }
                    }
                    Attribute::Vector4Array(value) => {
                        writer.write_byte(25)?;
                        writer.write_length(value.len())?;
                        for vector in value {
                            writer.write_bytes(
                                [vector.x.to_le_bytes(), vector.y.to_le_bytes(), vector.z.to_le_bytes(), vector.w.to_le_bytes()]
                                    .concat()
                                    .as_slice(),
                            )?;
                        }
                    }
                    Attribute::AngleArray(value) => {
                        writer.write_byte(26)?;
                        writer.write_length(value.len())?;
                        for angle in value {
                            writer.write_bytes(
                                [angle.pitch.to_le_bytes(), angle.yaw.to_le_bytes(), angle.roll.to_le_bytes()]
                                    .concat()
                                    .as_slice(),
                            )?;
                        }
                    }
                    Attribute::QuaternionArray(value) => {
                        writer.write_byte(27)?;
                        writer.write_length(value.len())?;
                        for quaternion in value {
                            writer.write_bytes(
                                [
                                    quaternion.x.to_le_bytes(),
                                    quaternion.y.to_le_bytes(),
                                    quaternion.z.to_le_bytes(),
                                    quaternion.w.to_le_bytes(),
                                ]
                                .concat()
                                .as_slice(),
                            )?;
                        }
                    }
                    Attribute::MatrixArray(value) => {
                        writer.write_byte(28)?;
                        writer.write_length(value.len())?;
                        for matrix in value {
                            writer.write_bytes(
                                [
                                    matrix.entries[0][0].to_le_bytes(),
                                    matrix.entries[0][1].to_le_bytes(),
                                    matrix.entries[0][2].to_le_bytes(),
                                    matrix.entries[0][3].to_le_bytes(),
                                    matrix.entries[1][0].to_le_bytes(),
                                    matrix.entries[1][1].to_le_bytes(),
                                    matrix.entries[1][2].to_le_bytes(),
                                    matrix.entries[1][3].to_le_bytes(),
                                    matrix.entries[2][0].to_le_bytes(),
                                    matrix.entries[2][1].to_le_bytes(),
                                    matrix.entries[2][2].to_le_bytes(),
                                    matrix.entries[2][3].to_le_bytes(),
                                    matrix.entries[3][0].to_le_bytes(),
                                    matrix.entries[3][1].to_le_bytes(),
                                    matrix.entries[3][2].to_le_bytes(),
                                    matrix.entries[3][3].to_le_bytes(),
                                ]
                                .concat()
                                .as_slice(),
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(BinarySerializationError::WrongEncoding);
        }

        if version < 1 || version > Self::version() {
            return Err(BinarySerializationError::InvalidEncodingVersion);
        }

        let mut reader = BinaryReader::new(buffer);
        reader.read::<u8>()?; // Skip byte from header

        reader.read_string_table(version)?;
        let element_count = reader.read()?;
        let mut elements = Vec::with_capacity(element_count as usize);

        for _ in 0..element_count {
            let element_class = reader.get_string(version)?;
            let element_name = if version >= 4 { reader.get_string(version)? } else { reader.read_string()? };
            reader.read_uuid()?;

            elements.push(Element::create(element_name, element_class));
        }

        for element_index in 0..element_count {
            let attribute_count = reader.read()?;
            let mut element = elements.get(element_index as usize).unwrap().clone();

            for _ in 0..attribute_count {
                let attribute_name = reader.get_string(version)?;
                let attribute_type = reader.read::<i8>()?;

                let attribute_value = match attribute_type {
                    1 => {
                        let attribute_data_index = reader.read()?;
                        let attribute_data = match attribute_data_index {
                            -1 => None,
                            _ => match elements.get(attribute_data_index as usize) {
                                Some(element) => Some(element.clone()),
                                None => return Err(BinarySerializationError::MissingElement),
                            },
                        };
                        Attribute::Element(attribute_data)
                    }
                    2 => Attribute::Integer(reader.read()?),
                    3 => Attribute::Float(reader.read()?),
                    4 => Attribute::Boolean(reader.read()?),
                    5 => {
                        let attribute_data = if version >= 4 { reader.get_string(version)? } else { reader.read_string()? };
                        Attribute::String(attribute_data)
                    }
                    6 => {
                        let attribute_data_size = reader.read()?;
                        Attribute::Binary(BinaryBlock {
                            data: reader.read_array(attribute_data_size)?,
                        })
                    }
                    7 => {
                        if version < 3 {
                            Attribute::ObjectId(reader.read_uuid()?)
                        } else {
                            let attribute_data_value = reader.read::<i32>()?;
                            let element_data = Duration::from_secs_f64(attribute_data_value as f64 / 10_000f64);
                            Attribute::Time(element_data)
                        }
                    }
                    8 => Attribute::Color(reader.read()?),
                    9 => Attribute::Vector2(reader.read()?),
                    10 => Attribute::Vector3(reader.read()?),
                    11 => Attribute::Vector4(reader.read()?),
                    12 => Attribute::Angle(reader.read()?),
                    13 => Attribute::Quaternion(reader.read()?),
                    14 => Attribute::Matrix(reader.read()?),
                    15 => {
                        let attribute_array_count = reader.read()?;
                        let attribute_data_values = reader.read_array(attribute_array_count)?;
                        let mut attribute_data = Vec::with_capacity(attribute_data_values.len());

                        for index in attribute_data_values {
                            attribute_data.push(match index {
                                -1 => None,
                                _ => match elements.get(index as usize) {
                                    Some(element) => Some(element.clone()),
                                    None => return Err(BinarySerializationError::MissingElement),
                                },
                            })
                        }

                        Attribute::ElementArray(attribute_data)
                    }
                    16 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::IntegerArray(reader.read_array(attribute_array_count)?)
                    }
                    17 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::FloatArray(reader.read_array(attribute_array_count)?)
                    }
                    18 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::BooleanArray(reader.read_array(attribute_array_count)?)
                    }
                    19 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::StringArray(reader.read_string_array(attribute_array_count)?)
                    }
                    20 => {
                        let attribute_array_count = reader.read()?;
                        let mut attribute_data = Vec::with_capacity(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_size = reader.read()?;
                            attribute_data.push(BinaryBlock {
                                data: reader.read_array(attribute_data_size)?,
                            });
                        }

                        Attribute::BinaryArray(attribute_data)
                    }
                    21 => {
                        if version < 3 {
                            let attribute_array_count = reader.read()?;
                            let mut attribute_data = Vec::with_capacity(attribute_array_count as usize);
                            for _ in 0..attribute_array_count {
                                attribute_data.push(reader.read_uuid()?);
                            }
                            Attribute::ObjectIdArray(attribute_data)
                        } else {
                            let attribute_array_count = reader.read()?;
                            let attribute_data_values = reader.read_array::<i32>(attribute_array_count)?;
                            let attribute_data = attribute_data_values.iter().map(|x| Duration::from_secs_f64((*x as f64) / 10_000f64)).collect();
                            Attribute::TimeArray(attribute_data)
                        }
                    }
                    22 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::ColorArray(reader.read_array(attribute_array_count)?)
                    }
                    23 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::Vector2Array(reader.read_array(attribute_array_count)?)
                    }
                    24 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::Vector3Array(reader.read_array(attribute_array_count)?)
                    }
                    25 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::Vector4Array(reader.read_array(attribute_array_count)?)
                    }
                    26 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::AngleArray(reader.read_array(attribute_array_count)?)
                    }
                    27 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::QuaternionArray(reader.read_array(attribute_array_count)?)
                    }
                    28 => {
                        let attribute_array_count = reader.read()?;
                        Attribute::MatrixArray(reader.read_array(attribute_array_count)?)
                    }
                    _ => return Err(BinarySerializationError::InvalidAttributeType),
                };

                element.set_attribute(attribute_name, attribute_value);
            }
        }

        Ok(elements.remove(0))
    }
}
