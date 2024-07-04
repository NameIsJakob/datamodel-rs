use std::{
    fs::File,
    io::{BufRead, BufReader, Error, Read},
    mem::{size_of, ManuallyDrop},
    ptr::read_unaligned,
    slice::from_raw_parts,
    time::Duration,
};

use indexmap::IndexSet;
use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

use crate::{
    attributes::{Angle, Color, Matrix, Quaternion, Vector2, Vector3, Vector4},
    serializing::FileHeaderError,
    Attribute, AttributeError, Element, Header, Serializer,
};

#[derive(Debug, ThisError)]
pub enum BinarySerializationError {
    #[error("Header Serializer Is Different")]
    WrongEncoding,
    #[error("Header Serializer Version Is Different")]
    InvalidEncodingVersion,
    #[error("Attributes In Header Is Not Supported By Serializer Version")]
    InvalidAttributeForVersion,
    #[error("To Many Elements To Serialize")]
    TooManyElements,
    #[error("Too Many Unique String To Serialize")]
    TooManyStrings,
    #[error("Element Has Too Many Attributes To Serialize")]
    TooManyAttributes,
    #[error("Attribute Array To Large To Serialize")]
    ArrayTooBig,
    #[error("Failed To Read File")]
    ReadFileError(#[from] Error),
    #[error("Read Invalid String Index")]
    InvalidStringIndex,
    #[error("Failed To Read Header")]
    InvalidHeader(#[from] FileHeaderError),
    #[error("Attributes Type Number Not Valid Attribute")]
    InvalidAttributeType,
    #[error("Failed To Add Attribute To Element")]
    FailedToAddAttribute(#[from] AttributeError),
    #[error("Element Index Was Invalid")]
    MissingElement,
}

#[derive(Debug, Default)]
struct DataWriter {
    data: Vec<u8>,
    string_table: IndexSet<String>,
}

impl DataWriter {
    fn write_byte(&mut self, value: i8) {
        self.data.extend(value.to_le_bytes())
    }

    fn write_bytes(&mut self, value: &[u8]) {
        self.data.extend(value)
    }

    fn write_short(&mut self, value: i16) {
        self.data.extend(value.to_le_bytes())
    }

    fn write_int(&mut self, value: i32) {
        self.data.extend(value.to_le_bytes())
    }

    fn write_length(&mut self, value: usize) -> Result<(), BinarySerializationError> {
        if value > i32::MAX as usize {
            return Err(BinarySerializationError::ArrayTooBig);
        }
        self.write_int(value as i32);
        Ok(())
    }

    fn write_float(&mut self, value: f32) {
        self.data.extend(value.to_le_bytes())
    }

    fn write_id(&mut self, value: UUID) {
        self.data.extend(value.to_bytes_le())
    }

    fn write_string(&mut self, value: &str) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }

    fn write_to_table(&mut self, value: &str, version: i32) -> Result<(), BinarySerializationError> {
        if version < 2 {
            self.write_string(value);
            return Ok(());
        }

        let index = self.string_table.insert_full(value.to_string()).0;

        if version >= 5 {
            if index > i32::MAX as usize {
                return Err(BinarySerializationError::TooManyStrings);
            }
            self.write_int(index as i32);
            return Ok(());
        }

        if index > i16::MAX as usize {
            return Err(BinarySerializationError::TooManyStrings);
        }
        self.write_short(index as i16);
        Ok(())
    }

    fn create_string_table(&mut self, version: i32) -> Option<Self> {
        if version < 2 {
            return None;
        }

        let mut string_table_writer = Self::default();

        if version >= 4 {
            string_table_writer.write_int(self.string_table.len() as i32);
        } else {
            string_table_writer.write_short(self.string_table.len() as i16);
        }

        for string in self.string_table.iter() {
            string_table_writer.write_string(string);
        }

        Some(string_table_writer)
    }
}

#[derive(Debug)]
struct DataReader {
    data: BufReader<File>,
    string_table: Vec<String>,
}

impl DataReader {
    fn new(data: BufReader<File>) -> Self {
        Self {
            data,
            string_table: Vec::new(),
        }
    }

    fn read_string(&mut self) -> Result<String, BinarySerializationError> {
        let mut string_buffer = Vec::new();
        let _ = self.data.read_until(0, &mut string_buffer)?;
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
        let string = self
            .string_table
            .get(string_index as usize)
            .ok_or(BinarySerializationError::InvalidStringIndex)?;
        Ok(string.clone())
    }

    fn read_id(&mut self) -> Result<UUID, BinarySerializationError> {
        let mut buffer = [0; 16];
        self.data.read_exact(&mut buffer)?;
        let value = UUID::from_bytes_le(buffer);
        Ok(value)
    }

    fn read<T>(&mut self) -> Result<T, BinarySerializationError> {
        let size = size_of::<T>();
        let mut buffer = vec![0; size];
        self.data.read_exact(&mut buffer)?;
        let value = unsafe { read_unaligned(buffer.as_ptr() as *const T) };
        Ok(value)
    }

    fn read_array<T>(&mut self, length: i32) -> Result<Vec<T>, BinarySerializationError> {
        let size = size_of::<T>();
        let mut buffer = vec![0; size * length as usize];
        self.data.read_exact(&mut buffer)?;
        let mut data = ManuallyDrop::new(buffer);
        let ptr = data.as_mut_ptr();
        let len = data.len() / size;
        let cap = data.capacity() / size;
        let value = unsafe { Vec::from_raw_parts(ptr as *mut T, len, cap) };
        Ok(value)
    }
}

pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    type Error = BinarySerializationError;

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error> {
        if header.get_encoding() != Self::name() {
            return Err(BinarySerializationError::WrongEncoding);
        }

        if header.encoding_version < 1 || header.encoding_version > Self::version() {
            return Err(BinarySerializationError::InvalidEncodingVersion);
        }

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
        collect_elements(root, &mut collected_elements);

        let mut writer = DataWriter::default();

        if collected_elements.len() > i32::MAX as usize {
            return Err(BinarySerializationError::TooManyElements);
        }
        writer.write_int(collected_elements.len() as i32);

        for element in &collected_elements {
            writer.write_to_table(&element.get_class(), header.encoding_version)?;
            if header.encoding_version >= 4 {
                writer.write_to_table(&element.get_name(), header.encoding_version)?;
            } else {
                writer.write_string(&element.get_class());
            }
            writer.write_id(UUID::new_v4());
        }

        for element in &collected_elements {
            let attributes = element.get_attributes();
            if attributes.len() > i32::MAX as usize {
                return Err(BinarySerializationError::TooManyAttributes);
            }
            writer.write_int(attributes.len() as i32);

            for (name, attribute) in attributes.iter() {
                writer.write_to_table(name, header.encoding_version)?;

                match attribute {
                    Attribute::Element(value) => {
                        writer.write_byte(1);

                        let element_value = match value {
                            Some(element_value) => element_value,
                            None => {
                                writer.write_int(-1);
                                continue;
                            }
                        };

                        let index = collected_elements.get_index_of(element_value).unwrap();

                        writer.write_int(index as i32);
                    }
                    Attribute::Integer(value) => {
                        writer.write_byte(2);
                        writer.write_int(*value);
                    }
                    Attribute::Float(value) => {
                        writer.write_byte(3);
                        writer.write_float(*value);
                    }
                    Attribute::Boolean(value) => {
                        writer.write_byte(4);
                        writer.write_byte(*value as i8);
                    }
                    Attribute::String(value) => {
                        writer.write_byte(5);

                        if header.encoding_version >= 4 {
                            writer.write_to_table(value, header.encoding_version)?;
                            continue;
                        }

                        writer.write_string(value);
                    }
                    Attribute::Binary(value) => {
                        writer.write_byte(6);
                        writer.write_length(value.len())?;
                        writer.write_bytes(value);
                    }
                    Attribute::ObjectId(value) => {
                        if header.encoding_version >= 3 {
                            return Err(BinarySerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(7);
                        writer.write_id(*value);
                    }
                    Attribute::Time(value) => {
                        if header.encoding_version < 3 {
                            return Err(BinarySerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(7);
                        writer.write_int((value.as_secs_f64() * 10_000f64) as i32);
                    }
                    Attribute::Color(value) => {
                        writer.write_byte(8);
                        writer.write_bytes(vec![value.r, value.g, value.b, value.a].as_slice());
                    }
                    Attribute::Vector2(value) => {
                        writer.write_byte(9);
                        writer.write_bytes([value.x.to_le_bytes(), value.y.to_le_bytes()].concat().as_slice());
                    }
                    Attribute::Vector3(value) => {
                        writer.write_byte(10);
                        writer.write_bytes([value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes()].concat().as_slice());
                    }
                    Attribute::Vector4(value) => {
                        writer.write_byte(11);
                        writer.write_bytes(
                            [value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes(), value.w.to_le_bytes()]
                                .concat()
                                .as_slice(),
                        );
                    }
                    Attribute::Angle(value) => {
                        writer.write_byte(12);
                        writer.write_bytes([value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes()].concat().as_slice());
                    }
                    Attribute::Quaternion(value) => {
                        writer.write_byte(13);
                        writer.write_bytes(
                            [value.x.to_le_bytes(), value.y.to_le_bytes(), value.z.to_le_bytes(), value.w.to_le_bytes()]
                                .concat()
                                .as_slice(),
                        );
                    }
                    Attribute::Matrix(value) => {
                        writer.write_byte(14);
                        let ptr = value.elements.as_ptr() as *const u8;
                        let len = size_of::<Matrix>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::ElementArray(value) => {
                        writer.write_byte(15);
                        writer.write_length(value.len())?;
                        for element in value {
                            let element_value = match element {
                                Some(element_value) => element_value,
                                None => {
                                    writer.write_int(-1);
                                    continue;
                                }
                            };

                            let index = collected_elements.get_index_of(element_value).unwrap();

                            writer.write_int(index as i32);
                        }
                    }
                    Attribute::IntegerArray(value) => {
                        writer.write_byte(16);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<i32>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::FloatArray(value) => {
                        writer.write_byte(17);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<f32>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::BooleanArray(value) => {
                        writer.write_byte(18);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<bool>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::StringArray(value) => {
                        writer.write_byte(19);
                        writer.write_length(value.len())?;
                        for value in value {
                            writer.write_string(value);
                        }
                    }
                    Attribute::BinaryArray(value) => {
                        writer.write_byte(20);
                        if value.len() > i32::MAX as usize {
                            return Err(BinarySerializationError::ArrayTooBig);
                        }
                        writer.write_length(value.len())?;
                        for value in value {
                            writer.write_length(value.len())?;
                            writer.write_bytes(value);
                        }
                    }
                    Attribute::ObjectIdArray(value) => {
                        if header.encoding_version >= 3 {
                            return Err(BinarySerializationError::InvalidAttributeForVersion);
                        }
                        writer.write_byte(21);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<UUID>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::TimeArray(value) => {
                        if header.encoding_version < 3 {
                            return Err(BinarySerializationError::InvalidAttributeForVersion);
                        }
                        writer.write_byte(21);
                        writer.write_length(value.len())?;
                        for value in value {
                            writer.write_int((value.as_secs_f64() * 10_000f64) as i32);
                        }
                    }
                    Attribute::ColorArray(value) => {
                        writer.write_byte(22);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Color>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::Vector2Array(value) => {
                        writer.write_byte(23);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Vector2>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::Vector3Array(value) => {
                        writer.write_byte(24);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Vector3>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::Vector4Array(value) => {
                        writer.write_byte(25);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Vector4>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::AngleArray(value) => {
                        writer.write_byte(26);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Angle>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::QuaternionArray(value) => {
                        writer.write_byte(27);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Quaternion>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                    Attribute::MatrixArray(value) => {
                        writer.write_byte(28);
                        writer.write_length(value.len())?;
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Matrix>();
                        let data = unsafe { from_raw_parts(ptr, len) };
                        writer.write_bytes(data);
                    }
                }
            }
        }

        let mut file_header = DataWriter::default();
        file_header.write_string(&header.to_string());

        if let Some(table) = writer.create_string_table(header.encoding_version) {
            file_header.data.extend(table.data)
        }

        file_header.data.extend(writer.data);

        Ok(file_header.data)
    }

    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error> {
        let mut reader = DataReader::new(data);

        let header = Header::from_string(reader.read_string()?)?;

        if header.get_encoding() != Self::name() {
            return Err(BinarySerializationError::WrongEncoding);
        }

        if header.encoding_version < 1 || header.encoding_version > Self::version() {
            return Err(BinarySerializationError::InvalidEncodingVersion);
        }

        reader.read_string_table(header.encoding_version)?;
        let element_count = reader.read()?;
        let mut elements = Vec::with_capacity(element_count as usize);

        for _ in 0..element_count {
            let element_class = reader.get_string(header.encoding_version)?;
            let element_name = if header.encoding_version >= 4 {
                reader.get_string(header.encoding_version)?
            } else {
                reader.read_string()?
            };
            reader.read_id()?;

            elements.push(Element::create(element_name, element_class));
        }

        for element_index in 0..element_count {
            let attribute_count = reader.read()?;
            let mut element = elements.get(element_index as usize).unwrap().clone();
            for _ in 0..attribute_count {
                let attribute_name = reader.get_string(header.encoding_version)?;
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
                        let attribute_data = if header.encoding_version >= 4 {
                            reader.get_string(header.encoding_version)?
                        } else {
                            reader.read_string()?
                        };
                        Attribute::String(attribute_data)
                    }
                    6 => {
                        let attribute_data_size = reader.read()?;
                        Attribute::Binary(reader.read_array(attribute_data_size)?)
                    }
                    7 => {
                        if header.encoding_version < 3 {
                            Attribute::ObjectId(reader.read_id()?)
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
                            attribute_data.push(reader.read_array(attribute_data_size)?);
                        }

                        Attribute::BinaryArray(attribute_data)
                    }
                    21 => {
                        if header.encoding_version < 3 {
                            let attribute_array_count = reader.read()?;
                            let mut attribute_data = Vec::with_capacity(attribute_array_count as usize);
                            for _ in 0..attribute_array_count {
                                attribute_data.push(reader.read_id()?);
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

                element.set_attribute(attribute_name, attribute_value)?;
            }
        }

        Ok((header, elements.remove(0)))
    }

    fn name() -> &'static str {
        "binary"
    }

    fn version() -> i32 {
        5
    }
}
