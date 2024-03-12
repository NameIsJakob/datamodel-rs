use indexmap::IndexSet;
use std::{mem::size_of, ptr::read_unaligned, slice::from_raw_parts, str::FromStr, time::Duration};
use uuid::Uuid as UUID;

use crate::{
    attributes::{Attribute, BinaryData, Color, Matrix, ObjectId, QAngle, Quaternion, Vector2, Vector3, Vector4},
    elements::Element,
};

use super::{Header, SerializationError, Serializer};

struct DataReader {
    data: Vec<u8>,
    index: usize,
    version: u8,
    string_table: Vec<String>,
}

impl DataReader {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            index: 0,
            version: 1,
            string_table: Vec::new(),
        }
    }

    fn read_string(&mut self) -> Result<String, SerializationError> {
        let start = self.index;
        let end = self.data[start..].iter().position(|&x| x == 0).ok_or(SerializationError::ByteExhaustion)?;
        self.index += end + 1;
        let string = String::from_utf8_lossy(&self.data[start..start + end]).into_owned();
        Ok(string)
    }

    fn read_string_array(&mut self, size: i32) -> Result<Vec<String>, SerializationError> {
        let mut string_count = 0;
        let mut end_index = 0;

        for (index, &value) in self.data[self.index..].iter().enumerate() {
            if value != 0 {
                continue;
            }

            string_count += 1;

            if string_count == size {
                end_index = index;
                break;
            }
        }

        let string = String::from_utf8_lossy(&self.data[self.index..self.index + end_index]).into_owned();
        self.index += end_index + 1;

        Ok(string.split('\0').map(String::from).collect())
    }

    fn read_string_table(&mut self) -> Result<(), SerializationError> {
        if self.version < 2 {
            return Ok(());
        }

        let string_table_count = if self.version >= 4 { self.read()? } else { self.read::<i16>()? as i32 };

        self.string_table = self.read_string_array(string_table_count)?;

        Ok(())
    }

    fn get_string(&mut self) -> Result<String, SerializationError> {
        if self.version < 2 {
            return self.read_string();
        }

        let string_index = if self.version >= 5 { self.read()? } else { self.read::<i16>()? as i32 };
        let string = self.string_table.get(string_index as usize).ok_or(SerializationError::InvalidStringIndex)?;
        Ok(string.clone())
    }

    fn read<T>(&mut self) -> Result<T, SerializationError> {
        let size = size_of::<T>();

        if self.index + size > self.data.len() {
            return Err(SerializationError::ByteExhaustion);
        }

        let value = unsafe { read_unaligned(&self.data[self.index] as *const u8 as *const T) };
        self.index += size;

        Ok(value)
    }

    fn read_array<T>(&mut self, length: i32) -> Result<&[T], SerializationError> {
        let size = size_of::<T>() * length as usize;

        if self.index + size > self.data.len() {
            return Err(SerializationError::ByteExhaustion);
        }

        let bytes = &self.data[self.index..self.index + size];
        let ptr = bytes.as_ptr();

        self.index += size;
        let slice = unsafe { from_raw_parts(ptr as *const T, length as usize) };

        Ok(slice)
    }
}

struct DataWriter {
    data: Vec<u8>,
    version: u8,
    string_table: IndexSet<String>,
}

impl DataWriter {
    fn new(version: u8) -> Self {
        Self {
            data: Vec::new(),
            version,
            string_table: IndexSet::new(),
        }
    }

    fn write_string_table(&mut self, root: &Element) {
        if self.version < 2 {
            return;
        }

        let mut table: IndexSet<String> = IndexSet::new();

        self.gather_strings(root, &mut table);

        if self.version >= 4 {
            self.write_int(table.len() as i32);
        } else {
            self.write_short(table.len() as i16);
        }

        for string in table.iter() {
            self.write_string(string.clone());
        }

        self.string_table = table;
    }

    fn gather_strings(&mut self, element: &Element, table: &mut IndexSet<String>) {
        table.insert(element.class.clone());

        if self.version >= 4 {
            table.insert(element.name.clone());
        }

        for (name, value) in element.get_attributes() {
            table.insert(name.clone());

            if let Attribute::String(value) = value {
                if self.version < 4 {
                    continue;
                }

                table.insert(value.clone());
            }
        }

        for value in element.get_elements().values() {
            self.gather_strings(value, table);
        }
    }

    fn write_to_table(&mut self, value: String) {
        if self.version < 2 {
            self.write_string(value);
            return;
        }

        let index = self.string_table.get_index_of(value.as_str()).unwrap();

        if self.version >= 5 {
            self.write_int(index as i32);
            return;
        }

        self.write_short(index as i16);
    }

    fn write_byte(&mut self, value: u8) {
        self.data.push(value);
    }

    fn write_bytes(&mut self, value: &[u8]) {
        self.data.extend(value);
    }

    fn write_short(&mut self, value: i16) {
        self.data.extend(value.to_le_bytes());
    }

    fn write_int(&mut self, value: i32) {
        self.data.extend(value.to_le_bytes());
    }

    fn write_float(&mut self, value: f32) {
        self.data.extend(value.to_le_bytes());
    }

    fn write_id(&mut self, value: UUID) {
        self.data.extend(value.to_bytes_le());
    }

    fn write_string(&mut self, value: String) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }
}

pub struct BinarySerializer {}

impl Serializer for BinarySerializer {
    fn serialize(root: &Element, header: &Header) -> Result<Vec<u8>, SerializationError> {
        if header.encoding_string() != "binary" {
            return Err(SerializationError::WrongDeserializer);
        }

        let mut writer = DataWriter::new(header.encoding_version());

        writer.write_string(header.to_string());

        writer.write_string_table(root);

        fn collect_elements<'a>(root: &'a Element, elements: &mut Vec<&'a Element>) {
            elements.push(root);
            for element in root.get_elements().values() {
                collect_elements(element, elements);
            }
        }

        let mut elements = Vec::new();
        collect_elements(root, &mut elements);

        writer.write_int(elements.len() as i32);

        for element in &elements {
            writer.write_to_table(element.class.clone());
            if header.encoding_version() >= 4 {
                writer.write_to_table(element.name.clone());
            } else {
                writer.write_string(element.name.clone());
            }
            writer.write_id(element.get_id());
        }

        for element in &elements {
            writer.write_int(element.get_attributes().len() as i32);

            for (name, attribute) in element.get_attributes() {
                writer.write_to_table(name.clone());

                match attribute {
                    Attribute::Element(value) => {
                        writer.write_byte(1);

                        if value.is_nil() {
                            writer.write_int(-1);
                            continue;
                        }

                        let index = elements.iter().position(|x| x.get_id() == *value);

                        match index {
                            Some(index) => writer.write_int(index as i32),
                            None => {
                                writer.write_int(-2);
                                writer.write_string(value.to_string());
                            }
                        }
                    }

                    Attribute::Int(value) => {
                        writer.write_byte(2);
                        writer.write_int(*value);
                    }

                    Attribute::Float(value) => {
                        writer.write_byte(3);
                        writer.write_float(*value);
                    }

                    Attribute::Bool(value) => {
                        writer.write_byte(4);
                        writer.write_byte(*value as u8);
                    }

                    Attribute::String(value) => {
                        writer.write_byte(5);

                        if header.encoding_version() >= 4 {
                            writer.write_to_table(value.to_string());
                            continue;
                        }

                        writer.write_string(value.to_string());
                    }

                    Attribute::Binary(value) => {
                        writer.write_byte(6);
                        writer.write_int(value.data.len() as i32);
                        writer.write_bytes(&value.data);
                    }

                    Attribute::Id(value) => {
                        if header.encoding_version() >= 4 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(7);
                        writer.write_id(value.id);
                    }

                    Attribute::Time(value) => {
                        if header.encoding_version() < 3 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(7);
                        writer.write_int((value.as_millis() as f32 * 10_000_f32) as i32);
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

                    Attribute::QAngle(value) => {
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
                        writer.write_bytes(value.entries.map(|x| x.to_le_bytes()).concat().as_slice());
                    }

                    Attribute::ElementArray(value) => {
                        writer.write_byte(15);
                        writer.write_int(value.len() as i32);

                        for element in value {
                            if element.is_nil() {
                                writer.write_int(-1);
                                continue;
                            }

                            let index = elements.iter().position(|x| x.get_id() == *element);

                            match index {
                                Some(index) => writer.write_int(index as i32),
                                None => {
                                    writer.write_int(-2);
                                    writer.write_string(element.to_string());
                                }
                            }
                        }
                    }

                    Attribute::IntArray(value) => {
                        writer.write_byte(16);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<i32>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::FloatArray(value) => {
                        writer.write_byte(17);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<f32>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::BoolArray(value) => {
                        writer.write_byte(18);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<bool>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::StringArray(value) => {
                        writer.write_byte(19);
                        writer.write_int(value.len() as i32);

                        if header.encoding_version() >= 4 {
                            for value in value {
                                writer.write_to_table(value.to_string());
                            }
                            continue;
                        }

                        for value in value {
                            writer.write_string(value.to_string());
                        }
                    }

                    Attribute::BinaryArray(value) => {
                        writer.write_byte(20);
                        writer.write_int(value.len() as i32);

                        for value in value {
                            writer.write_int(value.data.len() as i32);
                            writer.write_bytes(&value.data);
                        }
                    }

                    Attribute::IdArray(value) => {
                        if header.encoding_version() >= 4 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(21);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<ObjectId>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::TimeArray(value) => {
                        if header.encoding_version() < 3 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(21);
                        writer.write_int(value.len() as i32);

                        for value in value {
                            writer.write_int((value.as_millis() as f32 * 10_000_f32) as i32);
                        }
                    }

                    Attribute::ColorArray(value) => {
                        writer.write_byte(22);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Color>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::Vector2Array(value) => {
                        writer.write_byte(23);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Vector2>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::Vector3Array(value) => {
                        writer.write_byte(24);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Vector3>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::Vector4Array(value) => {
                        writer.write_byte(25);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Vector4>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::QAngleArray(value) => {
                        writer.write_byte(26);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<QAngle>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::QuaternionArray(value) => {
                        writer.write_byte(27);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Quaternion>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    Attribute::MatrixArray(value) => {
                        writer.write_byte(28);
                        writer.write_int(value.len() as i32);

                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Matrix>();

                        let data = unsafe { from_raw_parts(ptr, len) };

                        writer.write_bytes(data);
                    }

                    _ => continue,
                }
            }
        }

        Ok(writer.data)
    }

    fn deserialize(data: Vec<u8>) -> Result<Element, SerializationError> {
        let mut reader = DataReader::new(data);

        let header = Header::from_string(reader.read_string()?.as_str())?;

        if header.encoding_string() != "binary" {
            return Err(SerializationError::WrongDeserializer);
        }

        reader.version = header.encoding_version();

        reader.read_string_table()?;

        let element_count: i32 = reader.read()?;
        let mut elements: Vec<Element> = Vec::with_capacity(element_count as usize);

        for _ in 0..element_count {
            let element_class = reader.get_string()?;
            let element_name = if header.encoding_version() >= 4 {
                reader.get_string()?
            } else {
                reader.read_string()?
            };
            let element_id: UUID = reader.read()?;

            elements.push(Element::create(element_name, element_class, element_id));
        }

        for element_index in 0..element_count {
            let attribute_count: i32 = reader.read()?;

            for _ in 0..attribute_count {
                let attribute_name = reader.get_string()?;
                let attribute_type: u8 = reader.read()?;

                let attribute_value: Attribute = match attribute_type {
                    1 => {
                        let attribute_data_index: i32 = reader.read()?;
                        let attribute_data = match attribute_data_index {
                            -1 => UUID::nil(),
                            -2 => UUID::from_str(reader.read_string()?.as_str()).map_err(|_| SerializationError::InvalidUUID)?,
                            _ => match elements.get(attribute_data_index as usize) {
                                Some(element) => element.get_id(),
                                None => {
                                    return Err(SerializationError::InvalidElementIndex);
                                }
                            },
                        };
                        Attribute::Element(attribute_data)
                    }

                    2 => {
                        let attribute_data: i32 = reader.read()?;
                        Attribute::Int(attribute_data)
                    }

                    3 => {
                        let attribute_data: f32 = reader.read()?;
                        Attribute::Float(attribute_data)
                    }

                    4 => {
                        let attribute_data: bool = reader.read()?;
                        Attribute::Bool(attribute_data)
                    }

                    5 => {
                        let attribute_data = if header.encoding_version() >= 4 {
                            reader.get_string()?
                        } else {
                            reader.read_string()?
                        };
                        Attribute::String(attribute_data)
                    }

                    6 => {
                        let attribute_data_size: i32 = reader.read()?;
                        let attribute_data: Vec<u8> = reader.read_array(attribute_data_size)?.to_vec();
                        Attribute::Binary(BinaryData { data: attribute_data })
                    }

                    7 => {
                        if header.encoding_version() < 3 {
                            let attribute_data: UUID = reader.read()?;
                            Attribute::Id(ObjectId { id: attribute_data })
                        } else {
                            let attribute_data_value: i32 = reader.read()?;

                            let element_data = Duration::from_millis((attribute_data_value as f32 / 10_000_f32) as u64);
                            Attribute::Time(element_data)
                        }
                    }

                    8 => {
                        let attribute_data: Color = reader.read()?;
                        Attribute::Color(attribute_data)
                    }

                    9 => {
                        let attribute_data: Vector2 = reader.read()?;
                        Attribute::Vector2(attribute_data)
                    }

                    10 => {
                        let attribute_data: Vector3 = reader.read()?;
                        Attribute::Vector3(attribute_data)
                    }

                    11 => {
                        let attribute_data: Vector4 = reader.read()?;
                        Attribute::Vector4(attribute_data)
                    }

                    12 => {
                        let attribute_data: QAngle = reader.read()?;
                        Attribute::QAngle(attribute_data)
                    }

                    13 => {
                        let attribute_data: Quaternion = reader.read()?;
                        Attribute::Quaternion(attribute_data)
                    }

                    14 => {
                        let attribute_data: Matrix = reader.read()?;
                        Attribute::Matrix(attribute_data)
                    }

                    15 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data_values: Vec<i32> = reader.read_array(attribute_array_count)?.to_vec();
                        let attribute_data: Vec<UUID> = attribute_data_values
                            .iter()
                            .filter_map(|x| match x {
                                // FIXME: This should not just ignore the value if an error occurs
                                -1 => Some(UUID::nil()),
                                -2 => {
                                    let string = reader.read_string().ok()?;
                                    let id = UUID::from_str(string.as_str()).ok()?;
                                    Some(id)
                                }
                                _ => elements.get(*x as usize).map(|element| element.get_id()),
                            })
                            .collect();
                        Attribute::ElementArray(attribute_data)
                    }

                    16 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<i32> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::IntArray(attribute_data)
                    }

                    17 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<f32> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::FloatArray(attribute_data)
                    }

                    18 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<bool> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::BoolArray(attribute_data)
                    }

                    19 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data = reader.read_string_array(attribute_array_count)?;
                        Attribute::StringArray(attribute_data)
                    }

                    20 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let mut attribute_data: Vec<BinaryData> = Vec::with_capacity(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_size = reader.read::<i32>()?;
                            let attribute_data_values: Vec<u8> = reader.read_array(attribute_data_size)?.to_vec();
                            attribute_data.push(BinaryData { data: attribute_data_values });
                        }

                        Attribute::BinaryArray(attribute_data)
                    }

                    21 => {
                        if header.encoding_version() < 3 {
                            let attribute_array_count: i32 = reader.read()?;
                            let attribute_data_values: Vec<UUID> = reader.read_array(attribute_array_count)?.to_vec();
                            let attribute_data = attribute_data_values.into_iter().map(|x| ObjectId { id: x }).collect();
                            Attribute::IdArray(attribute_data)
                        } else {
                            let attribute_array_count: i32 = reader.read()?;
                            let attribute_data_values: Vec<i32> = reader.read_array(attribute_array_count)?.to_vec();
                            let attribute_data: Vec<Duration> = attribute_data_values
                                .iter()
                                .map(|x| Duration::from_millis(((*x as f32) / 10_000_f32) as u64))
                                .collect();
                            Attribute::TimeArray(attribute_data)
                        }
                    }

                    22 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<Color> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::ColorArray(attribute_data)
                    }

                    23 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<Vector2> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::Vector2Array(attribute_data)
                    }

                    24 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<Vector3> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::Vector3Array(attribute_data)
                    }

                    25 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<Vector4> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::Vector4Array(attribute_data)
                    }

                    26 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<QAngle> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::QAngleArray(attribute_data)
                    }

                    27 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<Quaternion> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::QuaternionArray(attribute_data)
                    }

                    28 => {
                        let attribute_array_count: i32 = reader.read()?;
                        let attribute_data: Vec<Matrix> = reader.read_array(attribute_array_count)?.to_vec();
                        Attribute::MatrixArray(attribute_data)
                    }

                    _ => {
                        return Err(SerializationError::InvalidAttributeType);
                    }
                };

                let element = elements.get_mut(element_index as usize).unwrap();
                element.add_attribute(attribute_name, attribute_value);
            }
        }

        loop {
            if elements.len() == 1 {
                return Ok(elements.remove(0));
            }

            let element = elements.pop().unwrap();

            if let Some(index) = elements.iter().position(|x| x.has_element_attribute(element.get_id())) {
                elements.get_mut(index).unwrap().add_element(element);
            }
        }
    }
}
