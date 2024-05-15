use std::{
    cell::RefCell,
    fs::File,
    io::{BufRead, BufReader, Read},
    mem::{size_of, ManuallyDrop},
    ptr::read_unaligned,
    rc::Rc,
    slice::from_raw_parts,
    str::FromStr,
    time::Duration,
};

use indexmap::IndexSet;
use uuid::Uuid as UUID;

use crate::{Angle, Attribute, Color, Element, Header, Matrix, Quaternion, SerializationError, Vector2, Vector3, Vector4};

use super::Serializer;

struct DataReader {
    data: BufReader<File>,
    version: i32,
    string_table: Vec<String>,
}

impl DataReader {
    fn new(data: BufReader<File>) -> Self {
        Self {
            data,
            version: 1,
            string_table: Vec::new(),
        }
    }

    fn read_string(&mut self) -> Result<String, SerializationError> {
        let mut string_buffer = Vec::new();
        let _ = self.data.read_until(0, &mut string_buffer)?;
        string_buffer.pop();
        let string = String::from_utf8_lossy(&string_buffer).into_owned();
        Ok(string)
    }

    fn read_string_array(&mut self, size: i32) -> Result<Vec<String>, SerializationError> {
        let mut strings = Vec::with_capacity(size as usize);

        for _ in 0..size {
            strings.push(self.read_string()?)
        }

        Ok(strings)
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

    fn read_id(&mut self) -> Result<UUID, SerializationError> {
        let mut buffer = [0; 16];
        let _ = self.data.read_exact(&mut buffer)?;
        let value = UUID::from_bytes_le(buffer);
        Ok(value)
    }

    fn read<T>(&mut self) -> Result<T, SerializationError> {
        let size = size_of::<T>();
        let mut buffer = vec![0; size];
        let _ = self.data.read_exact(&mut buffer)?;
        let value = unsafe { read_unaligned(buffer.as_ptr() as *const u8 as *const T) };
        Ok(value)
    }

    fn read_array<T>(&mut self, length: i32) -> Result<Vec<T>, SerializationError> {
        let size = size_of::<T>();
        let mut buffer = vec![0; size * length as usize];
        let _ = self.data.read_exact(&mut buffer)?;
        let mut data = ManuallyDrop::new(buffer);
        let ptr = data.as_mut_ptr();
        let len = data.len() / size;
        let cap = data.capacity() / size;
        let value = unsafe { Vec::from_raw_parts(ptr as *mut T, len, cap) };
        Ok(value)
    }
}

struct DataWriter {
    data: Vec<u8>,
    version: i32,
    string_table: IndexSet<String>,
}

impl DataWriter {
    fn new(version: i32) -> Self {
        Self {
            data: Vec::new(),
            version,
            string_table: IndexSet::new(),
        }
    }

    fn write_string_table(&mut self, root: &Rc<RefCell<Element>>) {
        if self.version < 2 {
            return;
        }

        let mut table = IndexSet::new();
        let mut checked = IndexSet::new();

        self.gather_strings(root, &mut table, &mut checked);

        if self.version >= 4 {
            self.write_int(table.len() as i32);
        } else {
            self.write_short(table.len() as i16);
        }

        for string in table.iter() {
            self.write_string(string);
        }

        self.string_table = table;
    }

    fn gather_strings(&self, element: &Rc<RefCell<Element>>, table: &mut IndexSet<String>, checked: &mut IndexSet<*const RefCell<Element>>) {
        let element_ptr = Rc::as_ptr(element);
        checked.insert(element_ptr);

        let element_class = element.borrow();

        if element_class.external {
            return;
        }

        if !table.contains(&element_class.class) {
            table.insert(element_class.class.clone());
        }

        if self.version >= 4 {
            if !table.contains(&element_class.name) {
                table.insert(element_class.name.clone());
            }
        }

        for (name, value) in element_class.get_attributes() {
            if !table.contains(name) {
                table.insert(name.clone());
            }

            match value {
                Attribute::Element(value) => match value {
                    Some(value) => {
                        let element_ptr = Rc::as_ptr(value);
                        if !checked.insert(element_ptr) {
                            continue;
                        }
                        self.gather_strings(&value, table, checked)
                    }
                    None => continue,
                },
                Attribute::String(value) => {
                    if self.version < 4 {
                        continue;
                    }
                    if !table.contains(value) {
                        table.insert(value.clone());
                    }
                }
                Attribute::ElementArray(value) => {
                    for element in value {
                        match element {
                            Some(value) => {
                                let element_ptr = Rc::as_ptr(value);
                                if !checked.insert(element_ptr) {
                                    continue;
                                }
                                self.gather_strings(value, table, checked)
                            }
                            None => continue,
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn write_to_table(&mut self, value: &str) {
        if self.version < 2 {
            self.write_string(value);
            return;
        }

        let index = self.string_table.get_index_of(value).unwrap();

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

    fn write_string(&mut self, value: &str) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }
}

pub struct BinarySerializer {}

impl Serializer for BinarySerializer {
    fn serialize(root: Rc<RefCell<Element>>, header: &Header) -> Result<Vec<u8>, SerializationError> {
        if header.encoding_string() != "binary" {
            return Err(SerializationError::WrongDeserializer);
        }

        let mut writer = DataWriter::new(header.encoding_version());

        writer.write_string(&header.to_string());

        writer.write_string_table(&root);

        fn collect_elements(root: Rc<RefCell<Element>>, elements: &mut IndexSet<*const RefCell<Element>>) {
            let ptr = Rc::into_raw(Rc::clone(&root));
            elements.insert(ptr);

            let element_class = root.borrow();

            if element_class.external {
                return;
            }

            for (_, value) in element_class.get_attributes() {
                match value {
                    Attribute::Element(value) => match value {
                        Some(value) => {
                            let element_ptr = Rc::as_ptr(value);
                            if !elements.insert(element_ptr) {
                                continue;
                            }
                            collect_elements(Rc::clone(value), elements)
                        }
                        None => continue,
                    },
                    Attribute::ElementArray(value) => {
                        for element in value {
                            match element {
                                Some(value) => {
                                    let element_ptr = Rc::as_ptr(value);
                                    if !elements.insert(element_ptr) {
                                        continue;
                                    }
                                    collect_elements(Rc::clone(value), elements)
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

        let mut elements = Vec::with_capacity(collected_elements.len());
        for ptr in &collected_elements {
            let rc_ptr = unsafe { Rc::from_raw(*ptr) };
            elements.push(rc_ptr);
        }

        writer.write_int(elements.len() as i32);

        for element in &elements {
            let borrowed = element.borrow();
            writer.write_to_table(&borrowed.class);
            if header.encoding_version() >= 4 {
                writer.write_to_table(&borrowed.name);
            } else {
                writer.write_string(&borrowed.name);
            }
            writer.write_id(borrowed.id);
        }

        for element in &elements {
            let borrowed = element.borrow();
            let attributes = borrowed.get_attributes();
            writer.write_int(attributes.len() as i32);

            for (name, attribute) in attributes {
                writer.write_to_table(name);

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

                        let element_borrow = element_value.borrow();

                        if element_borrow.external {
                            writer.write_int(-2);
                            writer.write_string(&element_borrow.id.to_string());
                            continue;
                        }

                        let index = collected_elements.get_index_of(&Rc::as_ptr(element_value)).unwrap();

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
                        writer.write_byte(*value as u8);
                    }
                    Attribute::String(value) => {
                        writer.write_byte(5);

                        if header.encoding_version() >= 4 {
                            writer.write_to_table(value);
                            continue;
                        }

                        writer.write_string(value);
                    }
                    Attribute::Binary(value) => {
                        writer.write_byte(6);
                        writer.write_int(value.len() as i32);
                        writer.write_bytes(&value);
                    }
                    Attribute::ObjectId(value) => {
                        if header.encoding_version() >= 4 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(7);
                        writer.write_id(*value);
                    }
                    Attribute::Time(value) => {
                        if header.encoding_version() < 3 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }

                        writer.write_byte(7);
                        writer.write_int((value.as_secs_f32() * 10_000f32) as i32);
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
                        writer.write_int(value.len() as i32);
                        for element in value {
                            let element_value = match element {
                                Some(element_value) => element_value,
                                None => {
                                    writer.write_int(-1);
                                    continue;
                                }
                            };

                            let element_borrow = element_value.borrow();

                            if element_borrow.external {
                                writer.write_int(-2);
                                writer.write_string(&element_borrow.id.to_string());
                                continue;
                            }

                            let index = collected_elements.get_index_of(&Rc::as_ptr(element_value)).unwrap();
                            writer.write_int(index as i32);
                        }
                    }
                    Attribute::IntegerArray(value) => {
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
                    Attribute::BooleanArray(value) => {
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
                        for value in value {
                            writer.write_string(value);
                        }
                    }
                    Attribute::BinaryArray(value) => {
                        writer.write_byte(20);
                        writer.write_int(value.len() as i32);
                        for value in value {
                            writer.write_int(value.len() as i32);
                            writer.write_bytes(&value);
                        }
                    }
                    Attribute::ObjectIdArray(value) => {
                        if header.encoding_version() >= 4 {
                            return Err(SerializationError::InvalidAttributeForVersion);
                        }
                        writer.write_byte(21);
                        writer.write_int(value.len() as i32);
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<UUID>();
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
                            writer.write_int((value.as_secs_f32() * 10_000f32) as i32);
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
                    Attribute::AngleArray(value) => {
                        writer.write_byte(26);
                        writer.write_int(value.len() as i32);
                        let ptr = value.as_ptr() as *const u8;
                        let len = value.len() * size_of::<Angle>();
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
                }
            }
        }

        Ok(writer.data)
    }

    fn deserialize(data: BufReader<File>) -> Result<Rc<RefCell<Element>>, SerializationError> {
        let mut reader = DataReader::new(data);

        let header = Header::from_string(reader.read_string()?)?;

        if header.encoding_string() != "binary" {
            return Err(SerializationError::WrongDeserializer);
        }

        reader.version = header.encoding_version();
        reader.read_string_table()?;

        let element_count = reader.read()?;
        let mut elements = Vec::with_capacity(element_count as usize);

        for _ in 0..element_count {
            let element_class = reader.get_string()?;
            let element_name = if header.encoding_version() >= 4 {
                reader.get_string()?
            } else {
                reader.read_string()?
            };
            let element_id = reader.read_id()?;

            elements.push(Some(Rc::new(RefCell::new(Element::create(element_name, element_class, element_id)))));
        }

        for element_index in 0..element_count {
            let attribute_count = reader.read()?;
            let mut element = elements.get(element_index as usize).unwrap().as_ref().unwrap().borrow_mut();
            element.reserve_attributes(attribute_count as usize);

            for _ in 0..attribute_count {
                let attribute_name = reader.get_string()?;
                let attribute_type = reader.read::<u8>()?;

                let attribute_value = match attribute_type {
                    1 => {
                        let attribute_data_index = reader.read()?;
                        let attribute_data = match attribute_data_index {
                            -1 => None,
                            -2 => {
                                let mut element = Element::create(
                                    String::from("unnamed"),
                                    String::from("DmElement"),
                                    UUID::from_str(&reader.read_string()?).map_err(|_| SerializationError::InvalidUUID)?,
                                );
                                element.external = true;
                                Some(Rc::new(RefCell::new(element)))
                            }
                            _ => match elements.get(attribute_data_index as usize) {
                                Some(element) => Some(Rc::clone(element.as_ref().unwrap())),
                                None => return Err(SerializationError::MissingElement),
                            },
                        };
                        Attribute::Element(attribute_data)
                    }
                    2 => Attribute::Integer(reader.read()?),
                    3 => Attribute::Float(reader.read()?),
                    4 => Attribute::Boolean(reader.read()?),
                    5 => {
                        let attribute_data = if header.encoding_version() >= 4 {
                            reader.get_string()?
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
                        if header.encoding_version() < 3 {
                            Attribute::ObjectId(reader.read_id()?)
                        } else {
                            let attribute_data_value = reader.read::<i32>()?;
                            let element_data = Duration::from_secs_f32(attribute_data_value as f32 / 10_000f32);
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
                                -2 => {
                                    let mut element = Element::create(
                                        String::from("unnamed"),
                                        String::from("DmElement"),
                                        UUID::from_str(&reader.read_string()?).map_err(|_| SerializationError::InvalidUUID)?,
                                    );
                                    element.external = true;
                                    Some(Rc::new(RefCell::new(element)))
                                }
                                _ => match elements.get(index as usize) {
                                    Some(element) => Some(Rc::clone(element.as_ref().unwrap())),
                                    None => return Err(SerializationError::MissingElement),
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
                        if header.encoding_version() < 3 {
                            let attribute_array_count = reader.read()?;
                            let mut attribute_data = Vec::with_capacity(attribute_array_count as usize);
                            for _ in 0..attribute_array_count {
                                attribute_data.push(reader.read_id()?);
                            }
                            Attribute::ObjectIdArray(attribute_data)
                        } else {
                            let attribute_array_count = reader.read()?;
                            let attribute_data_values = reader.read_array::<i32>(attribute_array_count)?;
                            let attribute_data = attribute_data_values.iter().map(|x| Duration::from_secs_f32((*x as f32) / 10_000f32)).collect();
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
                    _ => return Err(SerializationError::InvalidAttributeType),
                };

                element.set_attribute(attribute_name, attribute_value)
            }
        }

        Ok(elements.remove(0).unwrap())
    }
}
