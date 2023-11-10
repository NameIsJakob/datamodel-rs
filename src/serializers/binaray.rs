use std::{borrow::BorrowMut, rc::Rc, str::from_utf8};

use indexmap::IndexMap;
use uuid::Uuid;

use crate::attribute::{Attribute, Color, Vector2, Vector3, Vector4};
use crate::serializing::Serializer;
use crate::DmElement;
use crate::DmHeader;
use crate::SerializingError;

struct DataBufferReader {
    data: Vec<u8>,
    index: usize,
}

impl DataBufferReader {
    fn new(data: Vec<u8>) -> Self {
        Self { data, index: 0 }
    }

    fn read_string(&mut self) -> Result<&str, SerializingError> {
        let start = self.index;
        let end = self.data[self.index..]
            .iter()
            .position(|&x| x == 0)
            .ok_or(SerializingError::new("Not enough bytes to read from!"))?;
        self.index += end + 1;
        Ok(from_utf8(&self.data[start..start + end]).unwrap())
    }

    fn read_byte(&mut self) -> Result<u8, SerializingError> {
        let byte = self.data.get(self.index).ok_or(SerializingError::new("Not enough bytes to read from!"))?;
        self.index += 1;
        Ok(*byte)
    }

    fn read_bytes(&mut self, num_bytes: usize) -> Result<&[u8], SerializingError> {
        let bytes = self
            .data
            .get(self.index..self.index + num_bytes)
            .ok_or(SerializingError::new("Not enough bytes to read from!"))?;
        self.index += num_bytes;
        Ok(bytes)
    }

    fn read_int(&mut self) -> Result<i32, SerializingError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_float(&mut self) -> Result<f32, SerializingError> {
        let bytes = self.read_bytes(4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_id(&mut self) -> Result<Uuid, SerializingError> {
        let bytes = self.read_bytes(16)?;
        Ok(Uuid::from_bytes_le([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
            bytes[14], bytes[15],
        ]))
    }

    fn write_string(&mut self, value: &str) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }

    fn write_byte(&mut self, value: u8) {
        self.data.push(value);
    }

    fn write_bytes(&mut self, value: &[u8]) {
        self.data.extend(value);
    }

    fn write_int(&mut self, value: i32) {
        self.data.extend(value.to_le_bytes());
    }

    fn write_float(&mut self, value: f32) {
        self.data.extend(value.to_le_bytes());
    }

    fn write_id(&mut self, value: Uuid) {
        self.data.extend(value.to_bytes_le());
    }
}

pub struct BinaraySerializer {}

impl Serializer for BinaraySerializer {
    fn serialize(&self, root: Rc<DmElement>, header: &DmHeader) -> Result<Vec<u8>, SerializingError> {
        if header.encoding_version != 1 {
            return Err(SerializingError::new("Not Supported encoding version!"));
        }

        fn read_element(root: &Rc<DmElement>, elements: &mut Vec<Rc<DmElement>>) {
            let attributes = root.get_all_attributes();

            for (_, attribute) in attributes {
                match attribute {
                    Attribute::Element(element) => {
                        if !elements.iter().any(|x| Rc::ptr_eq(x, &element)) {
                            read_element(&element, elements);
                            elements.push(element)
                        }
                    }
                    Attribute::ElementArray(element_array) => {
                        for element in element_array {
                            if !elements.iter().any(|x| Rc::ptr_eq(x, &element)) {
                                read_element(&element, elements);
                                elements.push(element)
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut elements: Vec<Rc<DmElement>> = Vec::new();

        elements.push(root.clone());

        read_element(&root, &mut elements);

        let mut data_buffer = DataBufferReader::new(Vec::new());

        data_buffer.write_string(&format!(
            "<!-- dmx encoding {} {} format {} {} -->\n",
            header.encoding_name, header.encoding_version, header.format_name, header.format_version
        ));

        data_buffer.write_int(elements.len() as i32);

        for element in &elements {
            data_buffer.write_string(element.get_element_name());
            data_buffer.write_string(&element.get_name());
            data_buffer.write_id(*element.get_id());
        }

        for element in &elements {
            let attributes = element.get_all_attributes();
            data_buffer.write_int(attributes.len() as i32);

            for (name, attribute) in attributes {
                data_buffer.write_string(&name);

                match attribute {
                    Attribute::Unknown => return Err(SerializingError::new("Can not serialize unknown attribute!")),
                    Attribute::Element(value) => {
                        data_buffer.write_byte(1);

                        data_buffer.write_int(elements.iter().position(|x| Rc::ptr_eq(x, &value)).unwrap() as i32);
                    }
                    Attribute::Int(value) => {
                        data_buffer.write_byte(2);

                        data_buffer.write_int(value);
                    }
                    Attribute::Float(value) => {
                        data_buffer.write_byte(3);

                        data_buffer.write_float(value);
                    }
                    Attribute::Bool(value) => {
                        data_buffer.write_byte(4);

                        data_buffer.write_byte(value as u8);
                    }
                    Attribute::String(value) => {
                        data_buffer.write_byte(5);

                        data_buffer.write_string(&value);
                    }
                    Attribute::Void(value) => {
                        data_buffer.write_byte(6);

                        data_buffer.write_int(value.len() as i32);

                        data_buffer.write_bytes(&value);
                    }
                    Attribute::ObjectId(value) => {
                        data_buffer.write_byte(7);

                        data_buffer.write_id(value);
                    }
                    Attribute::Color(value) => {
                        data_buffer.write_byte(8);

                        data_buffer.write_byte(value.r);
                        data_buffer.write_byte(value.g);
                        data_buffer.write_byte(value.b);
                        data_buffer.write_byte(value.a);
                    }
                    Attribute::Vector2(value) => {
                        data_buffer.write_byte(9);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                    }
                    Attribute::Vector3(value) => {
                        data_buffer.write_byte(10);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                    }
                    Attribute::Vector4(value) => {
                        data_buffer.write_byte(11);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                        data_buffer.write_float(value.w);
                    }
                    Attribute::QAngle(value) => {
                        data_buffer.write_byte(12);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                    }
                    Attribute::Quaternion(value) => {
                        data_buffer.write_byte(13);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                        data_buffer.write_float(value.w);
                    }
                    Attribute::Matrix(value) => {
                        data_buffer.write_byte(14);

                        for index in value {
                            data_buffer.write_float(index);
                        }
                    }

                    Attribute::ElementArray(value) => {
                        data_buffer.write_byte(15);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_int(elements.iter().position(|x| Rc::ptr_eq(x, &item)).unwrap() as i32);
                        }
                    }
                    Attribute::IntArray(value) => {
                        data_buffer.write_byte(16);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_int(item);
                        }
                    }
                    Attribute::FloatArray(value) => {
                        data_buffer.write_byte(17);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_float(item);
                        }
                    }
                    Attribute::BoolArray(value) => {
                        data_buffer.write_byte(18);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_byte(item as u8);
                        }
                    }
                    Attribute::StringArray(value) => {
                        data_buffer.write_byte(19);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_string(&item);
                        }
                    }
                    Attribute::VoidArray(value) => {
                        data_buffer.write_byte(20);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_int(item.len() as i32);

                            data_buffer.write_bytes(&item);
                        }
                    }
                    Attribute::ObjectIdArray(value) => {
                        data_buffer.write_byte(21);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_id(item);
                        }
                    }
                    Attribute::ColorArray(value) => {
                        data_buffer.write_byte(22);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_byte(item.r);
                            data_buffer.write_byte(item.g);
                            data_buffer.write_byte(item.b);
                            data_buffer.write_byte(item.a);
                        }
                    }
                    Attribute::Vector2Array(value) => {
                        data_buffer.write_byte(23);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_float(item.x);
                            data_buffer.write_float(item.y);
                        }
                    }
                    Attribute::Vector3Array(value) => {
                        data_buffer.write_byte(24);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_float(item.x);
                            data_buffer.write_float(item.y);
                            data_buffer.write_float(item.z);
                        }
                    }
                    Attribute::Vector4Array(value) => {
                        data_buffer.write_byte(25);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_float(item.x);
                            data_buffer.write_float(item.y);
                            data_buffer.write_float(item.z);
                            data_buffer.write_float(item.w);
                        }
                    }
                    Attribute::QAngleArray(value) => {
                        data_buffer.write_byte(26);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_float(item.x);
                            data_buffer.write_float(item.y);
                            data_buffer.write_float(item.z);
                        }
                    }
                    Attribute::QuaternionArray(value) => {
                        data_buffer.write_byte(27);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            data_buffer.write_float(item.x);
                            data_buffer.write_float(item.y);
                            data_buffer.write_float(item.z);
                            data_buffer.write_float(item.w);
                        }
                    }
                    Attribute::MatrixArray(value) => {
                        data_buffer.write_byte(28);
                        data_buffer.write_int(value.len() as i32);

                        for item in value {
                            for index in item {
                                data_buffer.write_float(index);
                            }
                        }
                    }
                }
            }
        }

        Ok(data_buffer.data)
    }

    fn unserialize(&self, data: Vec<u8>) -> Result<Rc<DmElement>, SerializingError> {
        let mut data_buffer = DataBufferReader::new(data);

        let header_data = data_buffer.read_string()?;

        let header = DmHeader::from_string(header_data)?;

        if header.encoding_version != 1 {
            return Err(SerializingError::new("Not Supported encoding version!"));
        }

        let element_count = data_buffer.read_int()?;

        let mut elements: Vec<Rc<DmElement>> = Vec::new();
        elements.reserve(element_count as usize);

        for _ in 0..element_count {
            let element_type = data_buffer.read_string()?.to_string();
            let element_name = data_buffer.read_string()?.to_string();
            let element_id = data_buffer.read_id()?;

            let element = Rc::new(DmElement::new(element_type, element_name, Some(element_id)));

            elements.push(Rc::clone(&element));
        }

        for element_index in 0..element_count {
            let attribute_count = data_buffer.read_int()?;

            let mut attributes: IndexMap<String, Attribute> = IndexMap::new();

            for _ in 0..attribute_count {
                let attribute_name = data_buffer.read_string()?.to_string();

                let attribute_type = data_buffer.read_byte()?;

                // Is there a better way to do this?
                match attribute_type {
                    1 => {
                        let element_data_index = data_buffer.read_int()?;
                        attributes.insert(attribute_name, Attribute::Element(Rc::clone(&elements[element_data_index as usize])));
                    }
                    2 => {
                        let attribute_data = data_buffer.read_int()?;
                        attributes.insert(attribute_name, Attribute::Int(attribute_data));
                    }
                    3 => {
                        let attribute_data = data_buffer.read_float()?;
                        attributes.insert(attribute_name, Attribute::Float(attribute_data));
                    }
                    4 => {
                        let attribute_data = data_buffer.read_byte()?;
                        attributes.insert(attribute_name, Attribute::Bool(attribute_data != 0));
                    }
                    5 => {
                        let attribute_data = data_buffer.read_string()?.to_string();
                        attributes.insert(attribute_name, Attribute::String(attribute_data));
                    }
                    6 => {
                        let attribute_data_length = data_buffer.read_int()?;
                        let mut attribute_data: Vec<u8> = Vec::new();
                        attribute_data.reserve(attribute_data_length as usize);

                        attribute_data.extend_from_slice(data_buffer.read_bytes(attribute_data_length as usize)?);

                        attributes.insert(attribute_name, Attribute::Void(attribute_data));
                    }
                    7 => {
                        let attribute_data = data_buffer.read_id()?;
                        attributes.insert(attribute_name, Attribute::ObjectId(attribute_data));
                    }
                    8 => {
                        let attribute_data_r = data_buffer.read_byte()?;
                        let attribute_data_g = data_buffer.read_byte()?;
                        let attribute_data_b = data_buffer.read_byte()?;
                        let attribute_data_a = data_buffer.read_byte()?;

                        attributes.insert(
                            attribute_name,
                            Attribute::Color(Color {
                                r: attribute_data_r,
                                g: attribute_data_g,
                                b: attribute_data_b,
                                a: attribute_data_a,
                            }),
                        );
                    }
                    9 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;

                        attributes.insert(
                            attribute_name,
                            Attribute::Vector2(Vector2 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                            }),
                        );
                    }
                    10 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;

                        attributes.insert(
                            attribute_name,
                            Attribute::Vector3(Vector3 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            }),
                        );
                    }
                    11 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;
                        let attribute_data_w = data_buffer.read_float()?;

                        attributes.insert(
                            attribute_name,
                            Attribute::Vector4(Vector4 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            }),
                        );
                    }
                    12 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;

                        attributes.insert(
                            attribute_name,
                            Attribute::QAngle(Vector3 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            }),
                        );
                    }
                    13 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;
                        let attribute_data_w = data_buffer.read_float()?;

                        attributes.insert(
                            attribute_name,
                            Attribute::Quaternion(Vector4 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            }),
                        );
                    }
                    14 => {
                        let mut attribute_data: [f32; 16] = [0.0; 16];

                        for i in attribute_data.iter_mut() {
                            *i = data_buffer.read_float()?;
                        }

                        attributes.insert(attribute_name, Attribute::Matrix(attribute_data));
                    }
                    15 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Rc<DmElement>> = Vec::new();

                        for _ in 0..attribute_array_count {
                            let element_data_index = data_buffer.read_int()?;
                            attribute_data.push(Rc::clone(&elements[element_data_index as usize]));
                        }

                        attributes.insert(attribute_name, Attribute::ElementArray(attribute_data));
                    }
                    16 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<i32> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_int = data_buffer.read_int()?;
                            attribute_data.push(attribute_data_int);
                        }

                        attributes.insert(attribute_name, Attribute::IntArray(attribute_data));
                    }
                    17 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<f32> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_float = data_buffer.read_float()?;
                            attribute_data.push(attribute_data_float);
                        }

                        attributes.insert(attribute_name, Attribute::FloatArray(attribute_data));
                    }
                    18 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<bool> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_bool = data_buffer.read_byte()?;
                            attribute_data.push(attribute_data_bool != 0);
                        }

                        attributes.insert(attribute_name, Attribute::BoolArray(attribute_data));
                    }
                    19 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<String> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_string = data_buffer.read_string()?.to_string();
                            attribute_data.push(attribute_data_string);
                        }

                        attributes.insert(attribute_name, Attribute::StringArray(attribute_data));
                    }
                    20 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_array_data: Vec<Vec<u8>> = Vec::new();
                        attribute_array_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_length = data_buffer.read_int()?;
                            let mut attribute_data: Vec<u8> = Vec::new();
                            attribute_data.reserve(attribute_data_length as usize);

                            attribute_data.extend_from_slice(data_buffer.read_bytes(attribute_data_length as usize)?);

                            attribute_array_data.push(attribute_data);
                        }

                        attributes.insert(attribute_name, Attribute::VoidArray(attribute_array_data));
                    }
                    21 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Uuid> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_id = data_buffer.read_id()?;
                            attribute_data.push(attribute_data_id);
                        }

                        attributes.insert(attribute_name, Attribute::ObjectIdArray(attribute_data));
                    }
                    22 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Color> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_byte()?;
                            let attribute_data_y = data_buffer.read_byte()?;
                            let attribute_data_z = data_buffer.read_byte()?;
                            let attribute_data_w = data_buffer.read_byte()?;

                            attribute_data.push(Color {
                                r: attribute_data_x,
                                g: attribute_data_y,
                                b: attribute_data_z,
                                a: attribute_data_w,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::ColorArray(attribute_data));
                    }
                    23 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Vector2> = Vec::new();
                        // attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            attribute_data.push(Vector2 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::Vector2Array(attribute_data));
                    }
                    24 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Vector3> = Vec::new();
                        // attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            let attribute_data_z = data_buffer.read_float()?;

                            attribute_data.push(Vector3 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::Vector3Array(attribute_data));
                    }
                    25 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Vector4> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            let attribute_data_z = data_buffer.read_float()?;
                            let attribute_data_w = data_buffer.read_float()?;

                            attribute_data.push(Vector4 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::QuaternionArray(attribute_data));
                    }
                    26 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Vector3> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            let attribute_data_z = data_buffer.read_float()?;

                            attribute_data.push(Vector3 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::QAngleArray(attribute_data));
                    }
                    27 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Vector4> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            let attribute_data_z = data_buffer.read_float()?;
                            let attribute_data_w = data_buffer.read_float()?;

                            attribute_data.push(Vector4 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::QuaternionArray(attribute_data));
                    }
                    28 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<[f32; 16]> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let mut attribute_data_matrix: [f32; 16] = [0.0; 16];

                            for i in attribute_data_matrix.iter_mut() {
                                *i = data_buffer.read_float()?;
                            }

                            attribute_data.push(attribute_data_matrix);
                        }

                        attributes.insert(attribute_name, Attribute::MatrixArray(attribute_data));
                    }
                    _ => {
                        attributes.insert(attribute_name, Attribute::Unknown);
                    }
                }
            }

            let mut element = elements.get(element_index as usize).unwrap();

            for (name, attribute) in attributes {
                let borrow = element.borrow_mut();
                borrow.add_attribute(name, attribute);
            }
        }

        let root = elements.remove(0);

        Ok(root)
    }
}
