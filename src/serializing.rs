use std::{borrow::BorrowMut, collections::HashMap, rc::Rc, str::from_utf8};

use uuid::Uuid;

use crate::attribute::{Attribute, Color, Vector2, Vector3, Vector4};
use crate::element::DmElement;
use crate::{DmHeader, SerializingError};

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
        Ok((bytes[0] as i32) | ((bytes[1] as i32) << 8) | ((bytes[2] as i32) << 16) | ((bytes[3] as i32) << 24))
    }

    fn read_float(&mut self) -> Result<f32, SerializingError> {
        let bytes = self.read_bytes(4)?;
        Ok(((bytes[0] as u32) | ((bytes[1] as u32) << 8) | ((bytes[2] as u32) << 16) | ((bytes[3] as u32) << 24)) as f32)
    }

    fn read_id(&mut self) -> Result<Uuid, SerializingError> {
        let bytes = self.read_bytes(16)?;
        Ok(Uuid::from_bytes_le([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
            bytes[14], bytes[15],
        ]))
    }
}

pub trait Serializer {
    fn serialize(&self, root: &DmElement, header: &DmHeader) -> Result<Vec<u8>, SerializingError>;
    fn unserialize(&self, data: Vec<u8>) -> Result<DmElement, SerializingError>;
}

pub struct BinaraySerializer {}

impl Serializer for BinaraySerializer {
    fn serialize(&self, root: &DmElement, header: &DmHeader) -> Result<Vec<u8>, SerializingError> {
        todo!("Implement the serialize for Binaray!")
    }

    fn unserialize(&self, data: Vec<u8>) -> Result<DmElement, SerializingError> {
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

            let mut attributes: HashMap<String, Attribute> = HashMap::new();

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
                        attribute_data.reserve(attribute_array_count as usize);

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

            for (name, attribute) in attributes.drain() {
                let borrow = element.borrow_mut();
                borrow.add_attribute(name, attribute);
            }
        }

        let root = elements.remove(0);

        Ok(Rc::try_unwrap(root).unwrap())
    }
}

pub fn get_serializer(header: &DmHeader) -> Result<Box<dyn Serializer>, SerializingError> {
    match header.encoding_name.as_str() {
        "binary" => Ok(Box::new(BinaraySerializer {})),
        _ => Err(SerializingError::new("Not Supported encoding!")),
    }
}
