use super::{DmHeader, Serializer, SerializingError};
use crate::{attributes::DMAttribute, Binary, Color, DmElement, Matrix, QAngle, Quaternion, Vector2, Vector3, Vector4};
use indexmap::IndexMap;
use std::{str::from_utf8, time::Duration};
use uuid::Uuid as UUID;

struct DataBufferReader {
    data: Vec<u8>,
    index: usize,
    version: i32,
    string_table: Vec<String>,
}

impl DataBufferReader {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            index: 0,
            version: 1,
            string_table: Vec::new(),
        }
    }

    fn set_version(&mut self, version: i32) {
        self.version = version;
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

    fn read_short(&mut self) -> Result<i16, SerializingError> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_int(&mut self) -> Result<i32, SerializingError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_float(&mut self) -> Result<f32, SerializingError> {
        let bytes = self.read_bytes(4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_id(&mut self) -> Result<UUID, SerializingError> {
        let bytes = self.read_bytes(16)?;
        Ok(UUID::from_bytes_le([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
            bytes[14], bytes[15],
        ]))
    }

    fn read_string_table(&mut self) -> Result<(), SerializingError> {
        if self.version == 1 {
            return Ok(());
        }

        let string_table_count = if self.version >= 4 { self.read_int()? } else { self.read_short()? as i32 };
        for _ in 0..string_table_count {
            let string = self.read_string()?.to_string();
            self.string_table.push(string);
        }

        Ok(())
    }

    fn get_string(&mut self) -> Result<&str, SerializingError> {
        if self.version == 1 {
            return self.read_string();
        }

        let string_index = if self.version == 5 { self.read_int()? } else { self.read_short()? as i32 };

        Ok(self
            .string_table
            .get(string_index as usize)
            .ok_or(SerializingError::new("String index out of bounds!"))?)
    }
}

struct DataBufferWriter {
    data: Vec<u8>,
    version: i32,
    string_table: IndexMap<String, usize>,
}

impl DataBufferWriter {
    fn new(version: i32) -> Self {
        Self {
            data: Vec::new(),
            version,
            string_table: IndexMap::new(),
        }
    }

    fn write_string(&mut self, value: String) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }

    fn write_sting_to_table(&mut self, value: String) {
        if self.version == 1 {
            self.write_string(value);
            return;
        }

        let string_index = match self.string_table.get(&value) {
            Some(index) => *index,
            None => {
                panic!("String not found in string table!")
            }
        };

        if self.version == 5 {
            self.write_int(string_index as i32);
            return;
        }

        self.write_short(string_index as i16);
    }

    fn write_string_table(&mut self, element: &DmElement) {
        if self.version == 1 {
            return;
        }

        let mut table: IndexMap<String, usize> = IndexMap::new();

        self.read_element_to_table(element, &mut table);

        if self.version >= 4 {
            self.write_int(table.len() as i32);
        } else {
            self.write_short(table.len() as i16);
        }

        for string in table.keys() {
            self.write_string(string.to_string());
        }

        self.string_table = table;
    }

    fn read_element_to_table(&mut self, element: &DmElement, table: &mut IndexMap<String, usize>) {
        if !table.contains_key(element.get_class()) {
            table.insert(element.get_class().to_string(), table.len());
        }

        if self.version >= 4 && !table.contains_key(element.get_name()) {
            table.insert(element.get_name().to_string(), table.len());
        }

        for (name, value) in element.get_attributes() {
            if !table.contains_key(name) {
                table.insert(name.clone(), table.len());
            }

            if let DMAttribute::String(value) = value {
                if self.version < 4 {
                    continue;
                }

                if !table.contains_key(value) {
                    table.insert(value.clone(), table.len());
                }
            }
        }

        for (_, value) in element.get_elements() {
            self.read_element_to_table(value, table);
        }
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
}

pub struct BinarySerializer {}

impl Serializer for BinarySerializer {
    fn serialize(root: &DmElement, header: &DmHeader) -> Result<Vec<u8>, SerializingError> {
        if header.encoding_name != "binary" {
            return Err(SerializingError::new("Wrong Encoding For Deserialize!"));
        }
        if header.encoding_version < 1 || header.encoding_version > 5 {
            return Err(SerializingError::new("Not Supported encoding version! Only 1 through 5 are supported!"));
        }

        let mut data_buffer = DataBufferWriter::new(header.encoding_version);

        data_buffer.write_string(format!(
            "<!-- dmx encoding {} {} format {} {} -->\n",
            header.encoding_name, header.encoding_version, header.format_name, header.format_version
        ));

        data_buffer.write_string_table(root);

        fn collect_elements<'a>(root: &'a DmElement, elements: &mut Vec<&'a DmElement>) {
            elements.push(root);
            for (_, element) in root.get_elements() {
                collect_elements(element, elements);
            }
        }

        let mut elements: Vec<&DmElement> = Vec::new();
        collect_elements(root, &mut elements);

        data_buffer.write_int(elements.len() as i32);

        for element in &elements {
            data_buffer.write_sting_to_table(element.get_class().to_string());
            if header.encoding_version >= 4 {
                data_buffer.write_sting_to_table(element.get_name().to_string());
            } else {
                data_buffer.write_string(element.get_name().to_string());
            }
            data_buffer.write_id(*element.get_id());
        }

        for element in &elements {
            let attributes = element.get_attributes();

            data_buffer.write_int(attributes.len() as i32);

            for (name, attribute) in attributes {
                data_buffer.write_sting_to_table(name.to_string());

                match attribute {
                    DMAttribute::Element(value) => {
                        data_buffer.write_byte(1);

                        let index = elements.iter().position(|x| x.get_id() == value).unwrap_or(usize::MAX);
                        data_buffer.write_int(index as i32);
                    }
                    DMAttribute::Int(value) => {
                        data_buffer.write_byte(2);

                        data_buffer.write_int(*value);
                    }
                    DMAttribute::Float(value) => {
                        data_buffer.write_byte(3);

                        data_buffer.write_float(*value);
                    }
                    DMAttribute::Bool(value) => {
                        data_buffer.write_byte(4);

                        data_buffer.write_byte(if *value { 1 } else { 0 });
                    }
                    DMAttribute::String(value) => {
                        data_buffer.write_byte(5);

                        if header.encoding_version >= 4 {
                            data_buffer.write_sting_to_table(value.to_string());
                        } else {
                            data_buffer.write_string(value.to_string());
                        }
                    }
                    DMAttribute::Binary(value) => {
                        data_buffer.write_byte(6);

                        data_buffer.write_int(value.data.len() as i32);
                        data_buffer.write_bytes(&value.data);
                    }
                    DMAttribute::Id(value) => {
                        if header.encoding_version < 3 {
                            data_buffer.write_byte(7);

                            data_buffer.write_id(*value);
                        }
                    }
                    DMAttribute::Time(value) => {
                        if header.encoding_version >= 3 {
                            data_buffer.write_byte(7);

                            data_buffer.write_int((value.as_secs_f64() * 10000f64) as i32);
                        }
                    }
                    DMAttribute::Color(value) => {
                        data_buffer.write_byte(8);

                        data_buffer.write_byte(value.r);
                        data_buffer.write_byte(value.g);
                        data_buffer.write_byte(value.b);
                        data_buffer.write_byte(value.a);
                    }
                    DMAttribute::Vector2(value) => {
                        data_buffer.write_byte(9);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                    }
                    DMAttribute::Vector3(value) => {
                        data_buffer.write_byte(10);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                    }
                    DMAttribute::Vector4(value) => {
                        data_buffer.write_byte(11);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                        data_buffer.write_float(value.w);
                    }
                    DMAttribute::QAngle(value) => {
                        data_buffer.write_byte(12);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                    }
                    DMAttribute::Quaternion(value) => {
                        data_buffer.write_byte(13);

                        data_buffer.write_float(value.x);
                        data_buffer.write_float(value.y);
                        data_buffer.write_float(value.z);
                        data_buffer.write_float(value.w);
                    }
                    DMAttribute::Matrix(value) => {
                        data_buffer.write_byte(14);

                        for i in value.entries.iter() {
                            data_buffer.write_float(*i);
                        }
                    }
                    DMAttribute::ElementArray(value) => {
                        data_buffer.write_byte(15);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            let index = elements.iter().position(|x| x.get_id() == i).unwrap_or(usize::MAX);
                            data_buffer.write_int(index as i32);
                        }
                    }
                    DMAttribute::IntArray(value) => {
                        data_buffer.write_byte(16);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_int(*i);
                        }
                    }
                    DMAttribute::FloatArray(value) => {
                        data_buffer.write_byte(17);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_float(*i);
                        }
                    }
                    DMAttribute::BoolArray(value) => {
                        data_buffer.write_byte(18);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_byte(if *i { 1 } else { 0 });
                        }
                    }
                    DMAttribute::StringArray(value) => {
                        data_buffer.write_byte(19);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            if header.encoding_version >= 4 {
                                data_buffer.write_sting_to_table(i.to_string());
                            } else {
                                data_buffer.write_string(i.to_string());
                            }
                        }
                    }
                    DMAttribute::BinaryArray(value) => {
                        data_buffer.write_byte(20);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_int(i.data.len() as i32);
                            data_buffer.write_bytes(&i.data);
                        }
                    }
                    DMAttribute::IdArray(value) => {
                        if header.encoding_version < 3 {
                            data_buffer.write_byte(21);

                            data_buffer.write_int(value.len() as i32);
                            for i in value.iter() {
                                data_buffer.write_id(*i);
                            }
                        }
                    }
                    DMAttribute::TimeArray(value) => {
                        if header.encoding_version >= 3 {
                            data_buffer.write_byte(21);

                            data_buffer.write_int(value.len() as i32);
                            for i in value.iter() {
                                data_buffer.write_int((i.as_secs_f64() * 10000f64) as i32);
                            }
                        }
                    }
                    DMAttribute::ColorArray(value) => {
                        data_buffer.write_byte(22);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_byte(i.r);
                            data_buffer.write_byte(i.g);
                            data_buffer.write_byte(i.b);
                            data_buffer.write_byte(i.a);
                        }
                    }
                    DMAttribute::Vector2Array(value) => {
                        data_buffer.write_byte(23);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_float(i.x);
                            data_buffer.write_float(i.y);
                        }
                    }
                    DMAttribute::Vector3Array(value) => {
                        data_buffer.write_byte(24);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_float(i.x);
                            data_buffer.write_float(i.y);
                            data_buffer.write_float(i.z);
                        }
                    }
                    DMAttribute::Vector4Array(value) => {
                        data_buffer.write_byte(25);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_float(i.x);
                            data_buffer.write_float(i.y);
                            data_buffer.write_float(i.z);
                            data_buffer.write_float(i.w);
                        }
                    }
                    DMAttribute::QAngleArray(value) => {
                        data_buffer.write_byte(26);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_float(i.x);
                            data_buffer.write_float(i.y);
                            data_buffer.write_float(i.z);
                        }
                    }
                    DMAttribute::QuaternionArray(value) => {
                        data_buffer.write_byte(27);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            data_buffer.write_float(i.x);
                            data_buffer.write_float(i.y);
                            data_buffer.write_float(i.z);
                            data_buffer.write_float(i.w);
                        }
                    }
                    DMAttribute::MatrixArray(value) => {
                        data_buffer.write_byte(28);

                        data_buffer.write_int(value.len() as i32);
                        for i in value.iter() {
                            for j in i.entries.iter() {
                                data_buffer.write_float(*j);
                            }
                        }
                    }
                }
            }
        }

        Ok(data_buffer.data)
    }

    fn deserialize(data: Vec<u8>) -> Result<DmElement, SerializingError> {
        let mut data_buffer = DataBufferReader::new(data);

        let header_data = data_buffer.read_string()?;
        let header = DmHeader::from_string(header_data)?;
        if header.encoding_name != "binary" {
            return Err(SerializingError::new("Wrong Encoding For Deserialize!"));
        }
        if header.encoding_version < 1 || header.encoding_version > 5 {
            return Err(SerializingError::new("Not Supported encoding version! Only 1 through 5 are supported!"));
        }

        data_buffer.set_version(header.encoding_version);
        data_buffer.read_string_table()?;

        let element_count = data_buffer.read_int()?;
        let mut elements: Vec<DmElement> = Vec::new();
        elements.reserve(element_count as usize);

        for _ in 0..element_count {
            let element_class = data_buffer.get_string()?.to_string();
            let element_name = if header.encoding_version >= 4 {
                data_buffer.get_string()?.to_string()
            } else {
                data_buffer.read_string()?.to_string()
            };
            let element_id = data_buffer.read_id()?;

            let mut element = DmElement::new(element_class, element_name);
            element.set_id(element_id);

            elements.push(element);
        }

        for element_index in 0..element_count {
            let attribute_count = data_buffer.read_int()?;

            for _ in 0..attribute_count {
                let attribute_name = data_buffer.get_string()?.to_string();
                let attribute_type = data_buffer.read_byte()?;

                match attribute_type {
                    1 => {
                        let attribute_data_index = data_buffer.read_int()?;
                        let attribute_data = *elements.get(attribute_data_index as usize).unwrap().get_id();

                        let element = elements.get_mut(element_index as usize).unwrap();
                        element.set_element_by_id(attribute_name, attribute_data);
                    }

                    2 => {
                        let attribute_data = data_buffer.read_int()?;

                        let element = elements.get_mut(element_index as usize).unwrap();
                        element.set_attribute(attribute_name, attribute_data);
                    }

                    3 => {
                        let attribute_data = data_buffer.read_float()?;

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    4 => {
                        let attribute_data = data_buffer.read_byte()?;

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data != 0);
                    }

                    5 => {
                        let attribute_data = if header.encoding_version >= 4 {
                            data_buffer.get_string()?.to_string()
                        } else {
                            data_buffer.read_string()?.to_string()
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    6 => {
                        let attribute_data_length = data_buffer.read_int()?;
                        let mut attribute_data: Vec<u8> = Vec::new();
                        attribute_data.reserve(attribute_data_length as usize);
                        attribute_data.extend_from_slice(data_buffer.read_bytes(attribute_data_length as usize)?);

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, Binary { data: attribute_data });
                    }

                    7 => {
                        if header.encoding_version < 3 {
                            let attribute_data = data_buffer.read_id()?;

                            let element_data = elements.get_mut(element_index as usize).unwrap();
                            element_data.set_attribute(attribute_name, attribute_data);
                            continue;
                        }
                        let attribute_data = data_buffer.read_int()?;

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, Duration::from_micros((attribute_data as f64 / 10000f64) as u64));
                    }

                    8 => {
                        let attribute_data_r = data_buffer.read_byte()?;
                        let attribute_data_g = data_buffer.read_byte()?;
                        let attribute_data_b = data_buffer.read_byte()?;
                        let attribute_data_a = data_buffer.read_byte()?;
                        let attribute_data = Color {
                            r: attribute_data_r,
                            g: attribute_data_g,
                            b: attribute_data_b,
                            a: attribute_data_a,
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    9 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data = Vector2 {
                            x: attribute_data_x,
                            y: attribute_data_y,
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    10 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;
                        let attribute_data = Vector3 {
                            x: attribute_data_x,
                            y: attribute_data_y,
                            z: attribute_data_z,
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    11 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;
                        let attribute_data_w = data_buffer.read_float()?;
                        let attribute_data = Vector4 {
                            x: attribute_data_x,
                            y: attribute_data_y,
                            z: attribute_data_z,
                            w: attribute_data_w,
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    12 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;
                        let attribute_data = QAngle {
                            x: attribute_data_x,
                            y: attribute_data_y,
                            z: attribute_data_z,
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    13 => {
                        let attribute_data_x = data_buffer.read_float()?;
                        let attribute_data_y = data_buffer.read_float()?;
                        let attribute_data_z = data_buffer.read_float()?;
                        let attribute_data_w = data_buffer.read_float()?;
                        let attribute_data = Quaternion {
                            x: attribute_data_x,
                            y: attribute_data_y,
                            z: attribute_data_z,
                            w: attribute_data_w,
                        };

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    14 => {
                        let mut attribute_data: [f32; 16] = [0.0; 16];

                        for i in attribute_data.iter_mut() {
                            *i = data_buffer.read_float()?;
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, Matrix { entries: attribute_data });
                    }

                    15 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<UUID> = Vec::new();

                        for _ in 0..attribute_array_count {
                            let attribute_data_index = data_buffer.read_int()?;
                            attribute_data.push(*elements.get(attribute_data_index as usize).unwrap().get_id());
                        }

                        let element = elements.get_mut(element_index as usize).unwrap();
                        element.set_element_array_by_id(attribute_name, attribute_data);
                    }

                    16 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<i32> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_int = data_buffer.read_int()?;
                            attribute_data.push(attribute_data_int);
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    17 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<f32> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_float = data_buffer.read_float()?;
                            attribute_data.push(attribute_data_float);
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    18 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<bool> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_bool = data_buffer.read_byte()?;
                            attribute_data.push(attribute_data_bool != 0);
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    19 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<String> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_string = data_buffer.read_string()?.to_string();
                            attribute_data.push(attribute_data_string);
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    20 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_array_data: Vec<Binary> = Vec::new();
                        attribute_array_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_length = data_buffer.read_int()?;
                            let mut attribute_data: Vec<u8> = Vec::new();
                            attribute_data.reserve(attribute_data_length as usize);

                            attribute_data.extend_from_slice(data_buffer.read_bytes(attribute_data_length as usize)?);

                            attribute_array_data.push(Binary { data: attribute_data });
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_array_data);
                    }

                    21 => {
                        if header.encoding_version < 3 {
                            let attribute_array_count = data_buffer.read_int()?;
                            let mut attribute_data: Vec<UUID> = Vec::new();
                            attribute_data.reserve(attribute_array_count as usize);

                            for _ in 0..attribute_array_count {
                                let attribute_data_id = data_buffer.read_id()?;
                                attribute_data.push(attribute_data_id);
                            }

                            let element_data = elements.get_mut(element_index as usize).unwrap();
                            element_data.set_attribute(attribute_name, attribute_data);
                            continue;
                        }

                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Duration> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_time = data_buffer.read_int()?;
                            attribute_data.push(Duration::from_micros((attribute_data_time as f64 / 10000f64) as u64));
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
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

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
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

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
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

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
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

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    26 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<QAngle> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            let attribute_data_z = data_buffer.read_float()?;

                            attribute_data.push(QAngle {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                            });
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    27 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Quaternion> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let attribute_data_x = data_buffer.read_float()?;
                            let attribute_data_y = data_buffer.read_float()?;
                            let attribute_data_z = data_buffer.read_float()?;
                            let attribute_data_w = data_buffer.read_float()?;

                            attribute_data.push(Quaternion {
                                x: attribute_data_x,
                                y: attribute_data_y,
                                z: attribute_data_z,
                                w: attribute_data_w,
                            });
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    28 => {
                        let attribute_array_count = data_buffer.read_int()?;
                        let mut attribute_data: Vec<Matrix> = Vec::new();
                        attribute_data.reserve(attribute_array_count as usize);

                        for _ in 0..attribute_array_count {
                            let mut attribute_data_matrix: [f32; 16] = [0.0; 16];

                            for i in attribute_data_matrix.iter_mut() {
                                *i = data_buffer.read_float()?;
                            }

                            attribute_data.push(Matrix {
                                entries: attribute_data_matrix,
                            });
                        }

                        let element_data = elements.get_mut(element_index as usize).unwrap();
                        element_data.set_attribute(attribute_name, attribute_data);
                    }

                    _ => {
                        return Err(SerializingError::new("Unknown attribute type!"));
                    }
                }
            }
        }

        loop {
            if elements.len() == 1 {
                return Ok(elements.remove(0));
            }

            let element = elements.pop().unwrap();

            if let Some(index) = elements.iter().position(|x| x.has_element_attribute(*element.get_id())) {
                elements.get_mut(index).unwrap().add_element(element);
            }
        }
    }
}
