use regex::Regex;
use std::{collections, fs, path, str};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub enum Attribute {
    Unknown,
    ElementId(usize),
    Element(DmElement),
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Void(Vec<u8>),
    ObjectId(Uuid),
    Color { r: u8, g: u8, b: u8, a: u8 },
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4 { x: f32, y: f32, z: f32, w: f32 },
    QAngle { x: f32, y: f32, z: f32 },
    Quaternion { x: f32, y: f32, z: f32, w: f32 },
    Matrix([f32; 16]),
    ElementIdArray(Vec<usize>),
    ElementArray(Vec<DmElement>),
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BoolArray(Vec<bool>),
    StringArray(Vec<String>),
    VoidArray(Vec<Vec<u8>>),
    ObjectIdArray(Vec<Uuid>),
    ColorArray(Vec<Attribute>),
    Vector2Array(Vec<Vector2>),
    Vector3Array(Vec<Vector3>),
    Vector4Array(Vec<Attribute>),
    QAngleArray(Vec<Attribute>),
    QuaternionArray(Vec<Attribute>),
    MatrixArray(Vec<[f32; 16]>),
}

#[derive(Clone, Debug)]
pub struct DmElement {
    id: Uuid,
    element_name: String,
    name: String,
    attribute: collections::HashMap<String, Attribute>,
    elements: Vec<Attribute>,
}

impl DmElement {
    pub fn new(element_name: String, name: String, id: Option<Uuid>) -> Self {
        Self {
            id: id.unwrap_or(Uuid::new_v4()),
            element_name,
            name,
            attribute: collections::HashMap::new(),
            elements: Vec::new(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_element_name(&self) -> &str {
        &self.element_name
    }

    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        let attribute = self.attribute.get(name);

        if let Some(Attribute::ElementId(index)) = attribute {
            return self.elements.get(*index);
        }

        // TODO: Make get attribute support returning a element array
        // if let Some(Attribute::ElementIdArray(indexes)) = attribute {
        //     let elements: Vec<DmElement> = Vec::new();
        //     for index in indexes {
        //         let element = self.elements.get(*index).unwrap();

        //     }

        //     return Some(&Attribute::ElementArray(elements));
        // }

        attribute
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name
    }

    pub fn set_id(&mut self, id: Uuid) {
        self.id = id
    }

    pub fn add_attribute(&mut self, name: String, attribute: Attribute) {
        if let Attribute::Element(element) = attribute {
            let index = self.add_element(element);
            let element_attribute = Attribute::ElementId(index);
            self.attribute.insert(name, element_attribute);
            return;
        }

        if let Attribute::ElementArray(mut elements) = attribute {
            let mut element_attribute: Vec<usize> = Vec::new();

            for element in elements.drain(..) {
                let index = self.add_element(element);
                element_attribute.push(index);
            }

            self.attribute.insert(name, Attribute::ElementIdArray(element_attribute));
            return;
        }

        self.attribute.insert(name, attribute);
    }

    fn add_element(&mut self, element: DmElement) -> usize {
        self.elements.push(Attribute::Element(element));
        self.elements.len()
    }
}

#[derive(Clone, Debug)]
pub struct DmHeader {
    pub encoding_name: String,
    pub encoding_version: i32,
    pub format_name: String,
    pub format_version: i32,
}

impl DmHeader {
    const MAX_HEADER_LENGTH: usize = 168;

    fn from_bytes(data: &[u8]) -> Result<Self, String> {
        for (index, &value) in data.iter().enumerate() {
            if index >= Self::MAX_HEADER_LENGTH {
                return Err("Exceeded iteration limit without finding header!".to_string());
            }

            if value == b'\n' {
                return Self::from_string(str::from_utf8(&data[0..index]).unwrap());
            }
        }

        Err("Unexpected end of file!".to_string())
    }

    fn from_string(data: &str) -> Result<Self, String> {
        let header_match = Regex::new(r"<!-- dmx encoding (\w+) (\d+) format (\w+) (\d+) -->").unwrap();

        match header_match.captures(data) {
            Some(caps) => {
                let encoding_name = caps[1].to_string();
                let encoding_version = caps[2].parse::<i32>().unwrap(); // TODO: Handle this if not i32
                let format_name = caps[3].to_string();
                let format_version = caps[4].parse::<i32>().unwrap(); // TODO: Handle this if not i32

                Ok(Self {
                    encoding_name,
                    encoding_version,
                    format_name,
                    format_version,
                })
            }
            None => Err("String does not match the required format!".to_string()),
        }
    }
}

struct DataBufferReader {
    data: Vec<u8>,
    index: usize,
}

impl DataBufferReader {
    fn new(data: Vec<u8>) -> Self {
        Self { data, index: 0 }
    }
    // FIXME: Throw error if there is not eough bytes to read from!
    fn read_string(&mut self) -> &str {
        let start = self.index;
        loop {
            let byte = self.read_byte();
            if byte == 0 {
                break;
            }
        }
        str::from_utf8(&self.data[start..self.index - 1]).unwrap()
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.data[self.index];
        self.index += 1;
        byte
    }

    fn read_int(&mut self) -> i32 {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(&self.data[self.index..self.index + 4]);
        self.index += 4;
        i32::from_le_bytes(bytes)
    }

    fn read_float(&mut self) -> f32 {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(&self.data[self.index..self.index + 4]);
        self.index += 4;
        f32::from_le_bytes(bytes)
    }

    fn read_id(&mut self) -> Uuid {
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&self.data[self.index..self.index + 16]);
        self.index += 16;
        Uuid::from_bytes_le(bytes)
    }
}

trait Serializer {
    fn serialize(&self, root: DmElement, format_name: String, format_version: i32) -> Result<Vec<u8>, String>;
    fn unserialize(&self, data: Vec<u8>, encoding_version: i32) -> Result<DmElement, String>;
}

struct BinaraySerializer {}

impl Serializer for BinaraySerializer {
    fn serialize(&self, root: DmElement, format_name: String, format_version: i32) -> Result<Vec<u8>, String> {
        todo!("Implement the serialize for Binaray!")
    }

    fn unserialize(&self, data: Vec<u8>, encoding_version: i32) -> Result<DmElement, String> {
        if encoding_version != 1 {
            return Err("Not Supported encoding version!".to_string());
        }

        let mut data_buffer = DataBufferReader::new(data);

        let header_data = data_buffer.read_string();

        // Should we do something with this? Should we valate that its binaray and correct version?
        let _header = DmHeader::from_string(header_data)?;

        let element_count = data_buffer.read_int();

        let mut elements: Vec<DmElement> = Vec::new();
        elements.reserve(element_count as usize);

        for _ in 0..element_count {
            let element_type = data_buffer.read_string().to_string();
            let element_name = data_buffer.read_string().to_string();
            let element_id = data_buffer.read_id();

            let element = DmElement::new(element_type, element_name, Some(element_id));

            elements.push(element);
        }

        for element_index in 0..element_count {
            let attribute_count = data_buffer.read_int();

            let mut attributes: collections::HashMap<String, Attribute> = collections::HashMap::new();

            for _ in 0..attribute_count {
                let attribute_name = data_buffer.read_string().to_string();

                let attribute_type = data_buffer.read_byte();

                // Is there a better way to do this?
                match attribute_type {
                    1 => {
                        let element_data_index = data_buffer.read_int();
                        attributes.insert(attribute_name, Attribute::ElementId(element_data_index as usize - 1));
                    }
                    2 => {
                        let attribute_data = data_buffer.read_int();
                        attributes.insert(attribute_name, Attribute::Int(attribute_data));
                    }
                    3 => {
                        let attribute_data = data_buffer.read_float();
                        attributes.insert(attribute_name, Attribute::Float(attribute_data));
                    }
                    4 => {
                        let attribute_data = data_buffer.read_byte();
                        attributes.insert(attribute_name, Attribute::Bool(attribute_data != 0));
                    }
                    5 => {
                        let attribute_data = data_buffer.read_string().to_string();
                        attributes.insert(attribute_name, Attribute::String(attribute_data));
                    }
                    6 => {
                        todo!()
                    }
                    7 => {
                        let attribute_data = data_buffer.read_id();
                        attributes.insert(attribute_name, Attribute::ObjectId(attribute_data));
                    }
                    8 => {
                        let attribute_data_r = data_buffer.read_byte();
                        let attribute_data_g = data_buffer.read_byte();
                        let attribute_data_b = data_buffer.read_byte();
                        let attribute_data_a = data_buffer.read_byte();

                        attributes.insert(
                            attribute_name,
                            Attribute::Color {
                                r: attribute_data_r,
                                g: attribute_data_g,
                                b: attribute_data_b,
                                a: attribute_data_a,
                            },
                        );
                    }
                    9 => {
                        let attribute_data_x = data_buffer.read_float();
                        let attribute_data_y = data_buffer.read_float();

                        attributes.insert(
                            attribute_name,
                            Attribute::Vector2(Vector2 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                            }),
                        );
                    }
                    10 => {
                        let attribute_data_x = data_buffer.read_float();
                        let attribute_data_y = data_buffer.read_float();
                        let attribute_data_z = data_buffer.read_float();

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
                        let attribute_data_x = data_buffer.read_float();
                        let attribute_data_y = data_buffer.read_float();
                        let attribute_data_z = data_buffer.read_float();
                        let attribute_data_w = data_buffer.read_float();

                        attributes.insert(
                            attribute_name,
                            Attribute::Vector4 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            },
                        );
                    }
                    12 => {
                        let attribute_data_x = data_buffer.read_float();
                        let attribute_data_y = data_buffer.read_float();
                        let attribute_data_z = data_buffer.read_float();

                        attributes.insert(
                            attribute_name,
                            Attribute::QAngle {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            },
                        );
                    }
                    13 => {
                        let attribute_data_x = data_buffer.read_float();
                        let attribute_data_y = data_buffer.read_float();
                        let attribute_data_z = data_buffer.read_float();
                        let attribute_data_w = data_buffer.read_float();

                        attributes.insert(
                            attribute_name,
                            Attribute::Quaternion {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            },
                        );
                    }
                    14 => {
                        let mut attribute_data: [f32; 16] = [0.0; 16];

                        for i in attribute_data.iter_mut() {
                            *i = data_buffer.read_float();
                        }

                        attributes.insert(attribute_name, Attribute::Matrix(attribute_data));
                    }
                    15 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<usize> = Vec::new();

                        for _ in 0..attribute_array_count {
                            let element_data_index = data_buffer.read_int();
                            attribute_data.push(element_data_index as usize - 1);
                        }

                        attributes.insert(attribute_name, Attribute::ElementIdArray(attribute_data));
                    }
                    16 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<i32> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_int = data_buffer.read_int();
                            attribute_data.push(attribute_data_int);
                        }

                        attributes.insert(attribute_name, Attribute::IntArray(attribute_data));
                    }
                    17 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<f32> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_float = data_buffer.read_float();
                            attribute_data.push(attribute_data_float);
                        }

                        attributes.insert(attribute_name, Attribute::FloatArray(attribute_data));
                    }
                    18 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<bool> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_bool = data_buffer.read_byte();
                            attribute_data.push(attribute_data_bool != 0);
                        }

                        attributes.insert(attribute_name, Attribute::BoolArray(attribute_data));
                    }
                    19 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<String> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_string = data_buffer.read_string().to_string();
                            attribute_data.push(attribute_data_string);
                        }

                        attributes.insert(attribute_name, Attribute::StringArray(attribute_data));
                    }
                    20 => {
                        todo!()
                    }
                    21 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<Uuid> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_id = data_buffer.read_id();
                            attribute_data.push(attribute_data_id);
                        }

                        attributes.insert(attribute_name, Attribute::ObjectIdArray(attribute_data));
                    }
                    22 => {
                        todo!()
                    }
                    23 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<Vector2> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float();
                            let attribute_data_y = data_buffer.read_float();
                            attribute_data.push(Vector2 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::Vector2Array(attribute_data));
                    }
                    24 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<Vector3> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float();
                            let attribute_data_y = data_buffer.read_float();
                            let attribute_data_z = data_buffer.read_float();

                            attribute_data.push(Vector3 {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            });
                        }

                        attributes.insert(attribute_name, Attribute::Vector3Array(attribute_data));
                    }
                    25 => {
                        todo!()
                    }
                    26 => {
                        todo!()
                    }
                    27 => {
                        todo!()
                    }
                    28 => {
                        let attribute_array_count = data_buffer.read_int();
                        let mut attribute_data: Vec<[f32; 16]> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let mut attribute_data_matrix: [f32; 16] = [0.0; 16];

                            for i in attribute_data_matrix.iter_mut() {
                                *i = data_buffer.read_float();
                            }

                            attribute_data.push(attribute_data_matrix);
                        }

                        attributes.insert(attribute_name, Attribute::MatrixArray(attribute_data));
                    }
                    _ => {
                        todo!("Implement a way to handel unknown attributes!")
                    }
                }
            }

            let element = elements.get_mut(element_index as usize).unwrap();

            element.attribute.extend(attributes);
        }

        let mut root = elements.remove(0);

        root.elements.append(&mut elements.into_iter().map(Attribute::Element).collect());

        Ok(root)
    }
}

fn get_serializer(header: &DmHeader) -> Result<Box<dyn Serializer>, String> {
    match header.encoding_name.as_str() {
        "binary" => Ok(Box::new(BinaraySerializer {})),
        _ => Err("Not Supported encoding!".to_string()),
    }
}

// TODO: Give this a proper error.
pub fn load_from_file<P: AsRef<path::Path>>(path: P) -> Result<DmElement, String> {
    let file_data = fs::read(path).unwrap(); // TODO: Validate the file exist and handle any read errors.

    let header = DmHeader::from_bytes(&file_data)?;

    let serializer = get_serializer(&header)?;

    serializer.unserialize(file_data, header.encoding_version)
}
