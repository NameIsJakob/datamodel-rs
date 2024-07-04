use std::{
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader},
};

use indexmap::IndexMap;
use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

use crate::{Attribute, Element, Header, Serializer};

#[derive(Debug, Default)]
struct StringWriter {
    data: Vec<u8>,
    tab_index: usize,
}

impl StringWriter {
    fn write_raw(&mut self, string: &str) {
        self.data.extend(string.as_bytes());
    }

    fn write_tabs(&mut self) {
        if self.tab_index == 0 {
            return;
        }
        self.data.extend(vec![b'\t'; self.tab_index]);
    }

    fn write_string(&mut self, string: &str) {
        self.write_tabs();
        self.data.extend(string.as_bytes());
    }

    fn write_line(&mut self, string: &str) {
        self.write_tabs();
        self.data.extend(string.as_bytes());
        self.data.push(b'\n');
    }

    fn write_line_end(&mut self, string: &str) {
        self.data.extend(string.as_bytes());
        self.data.push(b'\n');
    }

    fn write_open_brace(&mut self) {
        self.write_tabs();
        self.data.push(b'{');
        self.data.push(b'\n');
        self.tab_index += 1;
    }

    fn write_close_brace(&mut self, array: bool, end_line: bool) {
        self.tab_index -= 1;
        self.write_tabs();
        self.data.push(b'}');
        if array {
            self.data.push(b',');
        }
        if end_line {
            self.data.push(b'\n');
        }
    }

    fn write_open_bracket(&mut self) {
        self.write_tabs();
        self.data.push(b'[');
        self.data.push(b'\n');
        self.tab_index += 1;
    }

    fn write_close_bracket(&mut self) {
        self.tab_index -= 1;
        self.write_tabs();
        self.data.push(b']');
        self.data.push(b'\n');
    }

    fn write_attributes(&mut self, element: &Element, elements: &IndexMap<Element, (UUID, usize)>) {
        for (name, attribute) in element.get_attributes().iter() {
            let attribute_type_name = get_attribute_type_name(attribute);

            match attribute {
                Attribute::Element(value) => match value {
                    Some(value) => {
                        let (id, count) = elements.get(value).unwrap();

                        if *count > 0 {
                            self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, id));
                            continue;
                        }

                        self.write_line(&format!("\"{}\" \"{}\"", name, value.get_class()));
                        self.write_open_brace();
                        self.write_line(&format!("\"id\" \"elementid\" \"{}\"", id));
                        self.write_line(&format!("\"name\" \"string\" \"{}\"", value.get_name()));
                        self.write_attributes(value, elements);
                        self.write_close_brace(false, false);
                        self.write_line("");
                    }
                    None => self.write_line(&format!("\"{}\" \"{}\" \"\"", name, attribute_type_name)),
                },
                Attribute::Integer(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Float(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Boolean(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, *value as u8)),
                Attribute::String(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Binary(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_line("\"");
                    let hex_string = value.iter().fold(String::new(), |mut output, byte| {
                        let _ = write!(output, "{byte:02X}");
                        output
                    });
                    let hexes = hex_string
                        .as_bytes()
                        .chunks(80)
                        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
                        .collect::<Vec<String>>();
                    for hex in hexes {
                        self.write_string("\t");
                        self.write_line_end(&hex);
                    }
                    self.write_line("\"");
                }
                Attribute::ObjectId(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Time(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value.as_secs_f64())),
                Attribute::Color(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Vector2(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Vector3(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Vector4(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Angle(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Quaternion(value) => self.write_line(&format!("\"{}\" \"{}\" \"{}\"", name, attribute_type_name, value)),
                Attribute::Matrix(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_line("\"");
                    let attribute_string = value.to_string();
                    let parts = attribute_string.split_whitespace().collect::<Vec<&str>>();
                    for chunk in parts.chunks(4) {
                        self.write_string("\t");
                        self.write_line_end(&format!("{} {} {} {}", chunk[0], chunk[1], chunk[2], chunk[3]));
                    }
                    self.write_line("\"");
                }
                Attribute::ElementArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            match element {
                                Some(value) => {
                                    let (id, count) = elements.get(value).unwrap();

                                    if *count > 0 {
                                        self.write_line(&format!("\"element\" \"{}\",", id));
                                        continue;
                                    }

                                    self.write_line(&format!("\"{}\"", value.get_class()));
                                    self.write_open_brace();
                                    self.write_line(&format!("\"id\" \"elementid\" \"{}\"", id));
                                    self.write_line(&format!("\"name\" \"string\" \"{}\"", value.get_name()));
                                    self.write_attributes(value, elements);
                                    self.write_close_brace(true, false);
                                    self.write_line("");
                                }
                                None => self.write_line("\"element\" \"\","),
                            }
                        }
                        match last {
                            Some(value) => {
                                let (id, count) = elements.get(value).unwrap();

                                if *count > 0 {
                                    self.write_line(&format!("\"element\" \"{}\"", id));
                                    self.write_close_bracket();
                                    continue;
                                }

                                self.write_line(&format!("\"{}\"", value.get_class()));
                                self.write_open_brace();
                                self.write_line(&format!("\"id\" \"elementid\" \"{}\"", id));
                                self.write_line(&format!("\"name\" \"string\" \"{}\"", value.get_name()));
                                self.write_attributes(value, elements);
                                self.write_close_brace(false, false);
                                self.write_line("");
                            }
                            None => self.write_line("\"element\" \"\""),
                        }
                    }
                    self.write_close_bracket();
                }
                Attribute::IntegerArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::FloatArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::BooleanArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", *element as u8));
                        }
                        self.write_line(&format!("\"{}\"", *last as u8));
                    }
                    self.write_close_bracket();
                }
                Attribute::StringArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::BinaryArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line("\"");
                            let hex_string = element.iter().fold(String::new(), |mut output, byte| {
                                let _ = write!(output, "{byte:02X}");
                                output
                            });
                            let hexes = hex_string
                                .as_bytes()
                                .chunks(80)
                                .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
                                .collect::<Vec<String>>();
                            for hex in hexes {
                                self.write_string("\t");
                                self.write_line_end(&hex);
                            }
                            self.write_line("\",");
                        }
                        self.write_line("\"");
                        let hex_string = last.iter().fold(String::new(), |mut output, byte| {
                            let _ = write!(output, "{byte:02X}");
                            output
                        });
                        let hexes = hex_string
                            .as_bytes()
                            .chunks(80)
                            .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
                            .collect::<Vec<String>>();
                        for hex in hexes {
                            self.write_string("\t");
                            self.write_line_end(&hex);
                        }
                        self.write_line("\"");
                    }
                    self.write_close_bracket();
                }
                Attribute::ObjectIdArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::TimeArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element.as_secs_f64()));
                        }
                        self.write_line(&format!("\"{}\"", last.as_secs_f64()));
                    }
                    self.write_close_bracket();
                }
                Attribute::ColorArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::Vector2Array(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::Vector3Array(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::Vector4Array(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::AngleArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::QuaternionArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line(&format!("\"{}\",", element));
                        }
                        self.write_line(&format!("\"{}\"", last));
                    }
                    self.write_close_bracket();
                }
                Attribute::MatrixArray(value) => {
                    self.write_line(&format!("\"{}\" \"{}\"", name, attribute_type_name));
                    self.write_open_bracket();
                    if let Some((last, values)) = value.split_last() {
                        for element in values {
                            self.write_line("\"");
                            let attribute_string = element.to_string();
                            let parts = attribute_string.split_whitespace().collect::<Vec<&str>>();
                            for chunk in parts.chunks(4) {
                                self.write_string("\t");
                                self.write_line_end(&format!("{} {} {} {}", chunk[0], chunk[1], chunk[2], chunk[3]));
                            }
                            self.write_line("\",");
                        }
                        self.write_line("\"");
                        let attribute_string = last.to_string();
                        let parts = attribute_string.split_whitespace().collect::<Vec<&str>>();
                        for chunk in parts.chunks(4) {
                            self.write_string("\t");
                            self.write_line_end(&format!("{} {} {} {}", chunk[0], chunk[1], chunk[2], chunk[3]));
                        }
                        self.write_line("\"");
                    }
                    self.write_close_bracket();
                }
            }
        }
    }
}

fn get_attribute_type_name(attribute: &Attribute) -> &'static str {
    match attribute {
        Attribute::Element(_) => "element",
        Attribute::Integer(_) => "int",
        Attribute::Float(_) => "float",
        Attribute::Boolean(_) => "bool",
        Attribute::String(_) => "string",
        Attribute::Binary(_) => "binary",
        Attribute::ObjectId(_) => "objectid",
        Attribute::Time(_) => "time",
        Attribute::Color(_) => "color",
        Attribute::Vector2(_) => "vector2",
        Attribute::Vector3(_) => "vector3",
        Attribute::Vector4(_) => "vector4",
        Attribute::Angle(_) => "qangle",
        Attribute::Quaternion(_) => "quaternion",
        Attribute::Matrix(_) => "matrix",
        Attribute::ElementArray(_) => "element_array",
        Attribute::IntegerArray(_) => "int_array",
        Attribute::FloatArray(_) => "float_array",
        Attribute::BooleanArray(_) => "bool_array",
        Attribute::StringArray(_) => "string_array",
        Attribute::BinaryArray(_) => "binary_array",
        Attribute::ObjectIdArray(_) => "objectid_array",
        Attribute::TimeArray(_) => "time_array",
        Attribute::ColorArray(_) => "color_array",
        Attribute::Vector2Array(_) => "vector2_array",
        Attribute::Vector3Array(_) => "vector3_array",
        Attribute::Vector4Array(_) => "vector4_array",
        Attribute::AngleArray(_) => "qangle_array",
        Attribute::QuaternionArray(_) => "quaternion_array",
        Attribute::MatrixArray(_) => "matrix_array",
    }
}

#[derive(Debug, ThisError)]
pub enum KeyvaluesSerializationError {
    #[error("Header Serializer Is Different")]
    WrongEncoding,
    #[error("Header Serializer Version Is Different")]
    InvalidEncodingVersion,
}

pub struct KeyValuesSerializer;

impl Serializer for KeyValuesSerializer {
    type Error = ();

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error> {
        todo!()
    }

    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error> {
        todo!()
    }

    fn name() -> &'static str {
        "keyvalues"
    }

    fn version() -> i32 {
        1
    }
}

#[derive(Debug, ThisError)]
pub enum Keyvalues2SerializationError {
    #[error("Header Serializer Is Different")]
    WrongEncoding,
    #[error("Header Serializer Version Is Different")]
    InvalidEncodingVersion,
}

pub struct KeyValues2Serializer;

impl Serializer for KeyValues2Serializer {
    type Error = Keyvalues2SerializationError;

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error> {
        if header.get_encoding() != Self::name() {
            return Err(Keyvalues2SerializationError::InvalidEncodingVersion);
        }

        if header.encoding_version < 1 || header.encoding_version > Self::version() {
            return Err(Keyvalues2SerializationError::InvalidEncodingVersion);
        }

        fn collect_elements(root: Element, elements: &mut IndexMap<Element, (UUID, usize)>) {
            elements.insert(root.clone(), (UUID::new_v4(), if elements.is_empty() { 1 } else { 0 }));

            for attribute in root.get_attributes().values() {
                match attribute {
                    Attribute::Element(value) => match value {
                        Some(element) => match elements.get_mut(element) {
                            Some((_, count)) => *count += 1,
                            None => collect_elements(element.clone(), elements),
                        },
                        None => continue,
                    },
                    Attribute::ElementArray(values) => {
                        for value in values {
                            match value {
                                Some(element) => match elements.get_mut(element) {
                                    Some((_, count)) => *count += 1,
                                    None => collect_elements(element.clone(), elements),
                                },
                                None => continue,
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut collected_elements = IndexMap::new();
        collect_elements(root, &mut collected_elements);

        let mut writer = StringWriter::default();

        writer.write_raw(&header.to_string());

        for (element, (id, count)) in &collected_elements {
            if *count == 0 {
                continue;
            }
            writer.write_line(&format!("\"{}\"", element.get_class()));
            writer.write_open_brace();
            writer.write_line(&format!("\"id\" \"elementid\" \"{}\"", id));
            writer.write_line(&format!("\"name\" \"string\" \"{}\"", element.get_name()));
            writer.write_attributes(element, &collected_elements);
            writer.write_close_brace(false, true);
            writer.write_line("");
        }

        Ok(writer.data)
    }

    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error> {
        todo!()
    }

    fn name() -> &'static str {
        "keyvalues2"
    }

    fn version() -> i32 {
        1
    }
}

pub struct KeyValues2FlatSerializer;

impl Serializer for KeyValues2FlatSerializer {
    type Error = Keyvalues2SerializationError;

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error> {
        if header.get_encoding() != Self::name() {
            return Err(Keyvalues2SerializationError::WrongEncoding);
        }

        if header.encoding_version < 1 || header.encoding_version > Self::version() {
            return Err(Keyvalues2SerializationError::InvalidEncodingVersion);
        }

        fn collect_elements(root: Element, elements: &mut IndexMap<Element, (UUID, usize)>) {
            elements.insert(root.clone(), (UUID::new_v4(), 1));

            for attribute in root.get_attributes().values() {
                match attribute {
                    Attribute::Element(value) => match value {
                        Some(element) => {
                            if elements.contains_key(element) {
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
                                    if elements.contains_key(element) {
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

        let mut collected_elements = IndexMap::new();
        collect_elements(root, &mut collected_elements);

        let mut writer = StringWriter::default();

        writer.write_raw(&header.to_string());

        for (element, (id, _)) in &collected_elements {
            writer.write_line(&format!("\"{}\"", element.get_class()));
            writer.write_open_brace();
            writer.write_line(&format!("\"id\" \"elementid\" \"{}\"", id));
            writer.write_line(&format!("\"name\" \"string\" \"{}\"", element.get_name()));
            writer.write_attributes(element, &collected_elements);
            writer.write_close_brace(false, true);
            writer.write_line("");
        }

        Ok(writer.data)
    }

    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error> {
        KeyValues2Serializer::deserialize(data)
    }

    fn name() -> &'static str {
        "keyvalues2_flat"
    }

    fn version() -> i32 {
        1
    }
}
