use std::{
    io::{BufRead, Error, Write},
    str::{FromStr, SplitWhitespace},
    time::Duration,
};

use indexmap::IndexMap;
use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

use crate::{
    attribute::{Angle, BinaryBlock, Color, Matrix, Quaternion, Vector2, Vector3, Vector4},
    Attribute, Element, Header, Serializer,
};

#[derive(Debug, ThisError)]
pub enum Keyvalues2SerializationError {
    #[error("IO Error: {0}")]
    Io(#[from] Error),
    #[error("Can't Serialize Deprecated Attribute")]
    DeprecatedAttribute,
    #[error("Header Serializer Version Is Different")]
    InvalidEncodingVersion,
    #[error("Header Serializer Is Different")]
    WrongEncoding,
    #[error("Found Unknown Token: {0} Line: {1}")]
    UnknownToken(char, usize),
    #[error("Found Unknown Escape Character: {0} Line: {1}")]
    UnknownEscapeCharacter(char, usize),
    #[error("Invalid Comment Delimiter On Line: {0}")]
    InvalidCommentDelimiter(usize),
    #[error("Invalid Token On Line: {0}")]
    InvalidToken(usize),
    #[error("Unfinished Attribute In Element")]
    UnfinishedAttribute,
    #[error("Failed To Parse Integer On Line: {0}")]
    FailedToParseInteger(usize),
    #[error("Failed To Parse Float On Line: {0}")]
    FailedToParseFloat(usize),
    #[error("Failed To Parse UUID On Line: {0}")]
    FailedToParseUUID(usize),
    #[error("Wrong Attribute Type On Line: {0}")]
    AttributeType(usize),
    #[error("Duplicate Element Id: {0}")]
    DuplicateElementId(UUID),
    #[error("Unknown Attribute Type On Line: {0}")]
    UnknownAttribute(usize),
    #[error("Invalid Attribute On Line: {0}")]
    InvalidAttribute(usize),
}

struct StringWriter<T: Write> {
    buffer: T,
    tab_index: usize,
}

impl<T: Write> StringWriter<T> {
    fn new(buffer: T) -> Self {
        Self { buffer, tab_index: 0 }
    }

    fn write_tabs(&mut self) -> Result<(), Keyvalues2SerializationError> {
        if self.tab_index == 0 {
            return Ok(());
        }
        self.buffer.write_all(&vec![b'\t'; self.tab_index])?;
        Ok(())
    }

    fn write_raw(&mut self, string: &str) -> Result<(), Keyvalues2SerializationError> {
        self.buffer.write_all(string.as_bytes())?;
        Ok(())
    }

    fn write_string(&mut self, string: &str) -> Result<(), Keyvalues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(string.as_bytes())?;
        Ok(())
    }

    fn write_line(&mut self, string: &str) -> Result<(), Keyvalues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(string.as_bytes())?;
        self.buffer.write_all(b"\n")?;
        Ok(())
    }

    fn write_open_brace(&mut self) -> Result<(), Keyvalues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(b"{\n")?;
        self.tab_index += 1;
        Ok(())
    }

    fn write_close_brace(&mut self) -> Result<(), Keyvalues2SerializationError> {
        self.tab_index -= 1;
        self.write_tabs()?;
        self.buffer.write_all(b"}\n")?;
        Ok(())
    }

    fn write_open_bracket(&mut self) -> Result<(), Keyvalues2SerializationError> {
        self.write_tabs()?;
        self.buffer.write_all(b"[\n")?;
        self.tab_index += 1;
        Ok(())
    }

    fn write_close_bracket(&mut self) -> Result<(), Keyvalues2SerializationError> {
        self.tab_index -= 1;
        self.write_tabs()?;
        self.buffer.write_all(b"]\n")?;
        Ok(())
    }

    fn write_attributes(&mut self, root: &Element, elements: &IndexMap<Element, (UUID, usize)>) -> Result<(), Keyvalues2SerializationError> {
        for (name, attribute) in root.get_attributes().iter() {
            let attribute_type_name = Self::get_attribute_type_name(attribute);

            match attribute {
                Attribute::Element(value) => {
                    if let Some(element) = value {
                        let (id, count) = elements.get(element).unwrap();

                        if *count > 0 {
                            self.write_line(&format!("{:?} {:?} \"{}\"", name, attribute_type_name, id))?;
                            continue;
                        }

                        self.write_line(&format!("{:?} {:?}", name, element.get_class()))?;
                        self.write_open_brace()?;
                        self.write_line(&format!("\"id\" \"elementid\" \"{}\"", id))?;
                        self.write_line(&format!("\"name\" \"string\" {:?}", element.get_name()))?;
                        self.write_attributes(element, elements)?;
                        self.write_close_brace()?;

                        continue;
                    }

                    self.write_line(&format!("{:?} \"{}\" \"\"", name, attribute_type_name))?;
                }
                Attribute::Integer(value) => self.write_line(&format!("{:?} \"{}\" \"{}\"", name, attribute_type_name, value))?,
                Attribute::Float(value) => self.write_line(&format!("{:?} \"{}\" \"{}\"", name, attribute_type_name, value))?,
                Attribute::Boolean(value) => self.write_line(&format!("{:?} \"{}\" \"{}\"", name, attribute_type_name, *value as u8))?,
                Attribute::String(value) => self.write_line(&format!("{:?} \"{}\" {:?}", name, attribute_type_name, value))?,
                Attribute::Binary(value) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_line("\"")?;
                    self.tab_index += 1;
                    for chunk in value.data.chunks(40) {
                        self.write_string(&format!(
                            "{}\n",
                            chunk.iter().fold(String::with_capacity(chunk.len() * 2), |mut output, byte| {
                                output.push_str(&format!("{:02X}", byte));
                                output
                            })
                        ))?;
                    }
                    self.tab_index -= 1;
                    self.write_line("\"")?;
                }
                Attribute::ObjectId(_) => return Err(Keyvalues2SerializationError::DeprecatedAttribute),
                Attribute::Time(value) => self.write_line(&format!("{:?} \"{}\" \"{}\"", name, attribute_type_name, value.as_secs_f64()))?,
                Attribute::Color(value) => self.write_line(&format!(
                    "\"{}\" \"{}\" \"{} {} {} {}\"",
                    name, attribute_type_name, value.red, value.green, value.blue, value.alpha
                ))?,
                Attribute::Vector2(value) => self.write_line(&format!("{:?} \"{}\" \"{} {}\"", name, attribute_type_name, value.x, value.y))?,
                Attribute::Vector3(value) => self.write_line(&format!("{:?} \"{}\" \"{} {} {}\"", name, attribute_type_name, value.x, value.y, value.z))?,
                Attribute::Vector4(value) => self.write_line(&format!(
                    "\"{}\" \"{}\" \"{} {} {} {}\"",
                    name, attribute_type_name, value.x, value.y, value.z, value.w
                ))?,
                Attribute::Angle(value) => self.write_line(&format!(
                    "\"{}\" \"{}\" \"{} {} {}\"",
                    name, attribute_type_name, value.roll, value.pitch, value.yaw
                ))?,
                Attribute::Quaternion(value) => self.write_line(&format!(
                    "\"{}\" \"{}\" \"{} {} {} {}\"",
                    name, attribute_type_name, value.x, value.y, value.z, value.w
                ))?,
                Attribute::Matrix(value) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_line("\"")?;
                    self.tab_index += 1;
                    for row in &value.entries {
                        self.write_string(&format!("{} {} {} {}\n", row[0], row[1], row[2], row[3]))?;
                    }
                    self.tab_index -= 1;
                    self.write_line("\"")?;
                }
                Attribute::ElementArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            if let Some(element) = value {
                                let (id, count) = elements.get(element).unwrap();

                                if *count > 0 {
                                    self.write_line(&format!("\"element\" \"{}\",", id))?;
                                    continue;
                                }

                                self.write_line(&format!("{:?}", element.get_class()))?;
                                self.write_open_brace()?;
                                self.write_line(&format!("\"id\" \"elementid\" \"{}\"", id))?;
                                self.write_line(&format!("\"name\" \"string\" {:?}", element.get_name()))?;
                                self.write_attributes(element, elements)?;
                                self.tab_index -= 1;
                                self.write_line("},")?;

                                continue;
                            };

                            self.write_line("\"element\" \"\",")?;
                        }

                        if let Some(element) = last {
                            let (id, count) = elements.get(element).unwrap();

                            if *count > 0 {
                                self.write_line(&format!("\"element\" \"{}\"", id))?;
                            } else {
                                self.write_line(&format!("{:?}", element.get_class()))?;
                                self.write_open_brace()?;
                                self.write_line(&format!("\"id\" \"elementid\" \"{}\"", id))?;
                                self.write_line(&format!("\"name\" \"string\" {:?}", element.get_name()))?;
                                self.write_attributes(element, elements)?;
                                self.write_close_brace()?;
                            }
                        } else {
                            self.write_line("\"element\" \"\"")?;
                        }
                    }
                    self.write_close_bracket()?;
                }
                Attribute::IntegerArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{}\",", value))?;
                        }
                        self.write_line(&format!("\"{}\"", last))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::FloatArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{}\",", value))?;
                        }
                        self.write_line(&format!("\"{}\"", last))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::BooleanArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{}\",", *value as u8))?;
                        }
                        self.write_line(&format!("\"{}\"", *last as u8))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::StringArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("{:?},", value))?;
                        }
                        self.write_line(&format!("{:?}", last))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::BinaryArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line("\"")?;
                            self.tab_index += 1;
                            for chunk in value.data.chunks(40) {
                                self.write_string(&format!(
                                    "{}\n",
                                    chunk.iter().fold(String::with_capacity(chunk.len() * 2), |mut output, byte| {
                                        output.push_str(&format!("{:02X}", byte));
                                        output
                                    })
                                ))?;
                            }
                            self.tab_index -= 1;
                            self.write_line("\",")?;
                        }
                        self.write_line("\"")?;
                        self.tab_index += 1;
                        for chunk in last.data.chunks(40) {
                            self.write_string(&format!(
                                "{}\n",
                                chunk.iter().fold(String::with_capacity(chunk.len() * 2), |mut output, byte| {
                                    output.push_str(&format!("{:02X}", byte));
                                    output
                                })
                            ))?;
                        }
                        self.tab_index -= 1;
                        self.write_line("\"")?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::ObjectIdArray(_) => return Err(Keyvalues2SerializationError::DeprecatedAttribute),
                Attribute::TimeArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{}\",", value.as_secs_f64()))?;
                        }
                        self.write_line(&format!("\"{}\"", last.as_secs_f64()))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::ColorArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{} {} {} {}\",", value.red, value.green, value.blue, value.alpha))?;
                        }
                        self.write_line(&format!("\"{} {} {} {}\"", last.red, last.green, last.blue, last.alpha))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::Vector2Array(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{} {}\",", value.x, value.y))?;
                        }
                        self.write_line(&format!("\"{} {}\"", last.x, last.y))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::Vector3Array(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{} {} {}\",", value.x, value.y, value.z))?;
                        }
                        self.write_line(&format!("\"{} {} {}\"", last.x, last.y, last.z))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::Vector4Array(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{} {} {} {}\",", value.x, value.y, value.z, value.w))?;
                        }
                        self.write_line(&format!("\"{} {} {} {}\"", last.x, last.y, last.z, last.w))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::AngleArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{} {} {}\",", value.roll, value.pitch, value.yaw))?;
                        }
                        self.write_line(&format!("\"{} {} {}\"", last.roll, last.pitch, last.yaw))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::QuaternionArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line(&format!("\"{} {} {} {}\",", value.x, value.y, value.z, value.w))?;
                        }
                        self.write_line(&format!("\"{} {} {} {}\"", last.x, last.y, last.z, last.w))?;
                    }
                    self.write_close_bracket()?;
                }
                Attribute::MatrixArray(values) => {
                    self.write_line(&format!("{:?} \"{}\"", name, attribute_type_name))?;
                    self.write_open_bracket()?;
                    if let Some((last, values)) = values.split_last() {
                        for value in values {
                            self.write_line("\"")?;
                            self.tab_index += 1;
                            for row in &value.entries {
                                self.write_string(&format!("{} {} {} {}\n", row[0], row[1], row[2], row[3]))?;
                            }
                            self.tab_index -= 1;
                            self.write_line("\",")?;
                        }
                        self.write_line("\"")?;
                        self.tab_index += 1;
                        for row in last.entries {
                            self.write_string(&format!("{} {} {} {}\n", row[0], row[1], row[2], row[3]))?;
                        }
                        self.tab_index -= 1;
                        self.write_line("\"")?;
                    }
                    self.write_close_bracket()?;
                }
            }
        }
        Ok(())
    }

    fn get_attribute_type_name(attribute: &Attribute) -> &'static str {
        match attribute {
            Attribute::Element(_) => "element",
            Attribute::Integer(_) => "int",
            Attribute::Float(_) => "float",
            Attribute::Boolean(_) => "bool",
            Attribute::String(_) => "string",
            Attribute::Binary(_) => "binary",
            Attribute::ObjectId(_) => "elementid",
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
            Attribute::ObjectIdArray(_) => "elementid_array",
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
}

struct StringReader<B: BufRead> {
    buffer: B,
    line_count: usize,
    current_line: String,
    cursor_position: usize,
}

impl<B: BufRead> StringReader<B> {
    fn new(buffer: B) -> Self {
        Self {
            buffer,
            line_count: 0,
            current_line: String::new(),
            cursor_position: 0,
        }
    }

    fn next_token(&mut self) -> Result<Option<StringToken>, Keyvalues2SerializationError> {
        if self.current_line.len() == self.cursor_position {
            let new_line = match self.next_line()? {
                Some(line) => line,
                None => return Ok(None),
            };
            self.current_line = new_line;
            self.cursor_position = 0;
            self.line_count += 1;
        }

        let mut line_characters = self.current_line.chars().skip(self.cursor_position);
        let mut token = None;

        let mut escaped = false;
        let mut commented = false;

        loop {
            let current_character = line_characters.next();
            self.cursor_position += 1;
            match current_character {
                Some('"') => {
                    if let Some(StringToken::String(ref mut string_token)) = token {
                        if escaped {
                            string_token.push('\"');
                            escaped = false;
                            continue;
                        }
                        break;
                    }

                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if token.is_none() {
                        token = Some(StringToken::String(String::new()));
                        continue;
                    }
                }
                Some('{') => {
                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter('{', self.line_count));
                    }

                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if token.is_none() {
                        token = Some(StringToken::OpenBrace);
                        break;
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        string_token.push('{');
                    }
                }
                Some('}') => {
                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter('}', self.line_count));
                    }

                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if token.is_none() {
                        token = Some(StringToken::CloseBrace);
                        break;
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        string_token.push('}');
                    }
                }
                Some('[') => {
                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter('[', self.line_count));
                    }

                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if token.is_none() {
                        token = Some(StringToken::OpenBracket);
                        break;
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        string_token.push('[');
                    }
                }
                Some(']') => {
                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter(']', self.line_count));
                    }

                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if token.is_none() {
                        token = Some(StringToken::CloseBracket);
                        break;
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        string_token.push(']');
                    }
                }
                Some(',') => {
                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter(',', self.line_count));
                    }

                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        string_token.push(',');
                    }
                }
                Some('\\') => {
                    if let Some(StringToken::String(ref mut string_token)) = token {
                        if !escaped {
                            escaped = true;
                            continue;
                        }
                        string_token.push('\\');
                        escaped = false;
                        continue;
                    }

                    return Err(Keyvalues2SerializationError::UnknownToken('\\', self.line_count));
                }
                Some('/') => {
                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter('/', self.line_count));
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        string_token.push('/');
                        continue;
                    }

                    if commented {
                        let new_line = match self.next_line()? {
                            Some(line) => line,
                            None => return Ok(None),
                        };
                        self.current_line = new_line;
                        self.cursor_position = 0;
                        self.line_count += 1;
                        line_characters = self.current_line.chars().skip(self.cursor_position);
                        commented = false;
                        continue;
                    }

                    commented = true;
                }
                Some(character) => {
                    if commented {
                        return Err(Keyvalues2SerializationError::InvalidCommentDelimiter(self.line_count));
                    }

                    if let Some(StringToken::String(ref mut string_token)) = token {
                        if escaped {
                            match character {
                                'n' => string_token.push('n'),
                                't' => string_token.push('t'),
                                'v' => string_token.push('v'),
                                'b' => string_token.push('b'),
                                'r' => string_token.push('r'),
                                'f' => string_token.push('f'),
                                'a' => string_token.push('a'),
                                '?' => string_token.push('?'),
                                '\'' => string_token.push('\''),
                                _ => return Err(Keyvalues2SerializationError::UnknownEscapeCharacter(character, self.line_count)),
                            }
                            escaped = false;
                            continue;
                        }
                        string_token.push(character);
                        continue;
                    }

                    if escaped {
                        return Err(Keyvalues2SerializationError::UnknownEscapeCharacter(character, self.line_count));
                    }

                    if character.is_whitespace() {
                        continue;
                    }

                    return Err(Keyvalues2SerializationError::UnknownToken(character, self.line_count));
                }
                None => {
                    let new_line = match self.next_line()? {
                        Some(line) => line,
                        None => return Ok(None),
                    };
                    self.current_line = new_line;
                    self.cursor_position = 0;
                    self.line_count += 1;
                    line_characters = self.current_line.chars().skip(self.cursor_position);
                }
            }
        }

        Ok(token)
    }

    fn next_line(&mut self) -> Result<Option<String>, Keyvalues2SerializationError> {
        let mut line = String::new();
        let byte_count = self.buffer.read_line(&mut line)?;
        if byte_count == 0 {
            return Ok(None);
        }
        Ok(Some(line))
    }
}

enum StringToken {
    String(String),
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
}

fn read_attribute<T: BufRead>(
    reader: &mut StringReader<T>,
    attribute_type: &str,
    attribute_value: StringToken,
    elements: &mut IndexMap<UUID, Element>,
) -> Result<Attribute, Keyvalues2SerializationError> {
    match attribute_type {
        "element" => match attribute_value {
            StringToken::String(value) => {
                if value.is_empty() {
                    return Ok(Attribute::Element(None));
                }

                let id = UUID::from_str(&value).map_err(|_| Keyvalues2SerializationError::FailedToParseUUID(reader.line_count))?;

                match elements.entry(id) {
                    indexmap::map::Entry::Occupied(occupied_entry) => Ok(Attribute::Element(Some(occupied_entry.get().clone()))),
                    indexmap::map::Entry::Vacant(vacant_entry) => {
                        let element = Element::default();
                        vacant_entry.insert(element.clone());
                        Ok(Attribute::Element(Some(element)))
                    }
                }
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "int" => match attribute_value {
            StringToken::String(value) => match value.parse() {
                Ok(value) => Ok(Attribute::Integer(value)),
                Err(_) => Err(Keyvalues2SerializationError::FailedToParseInteger(reader.line_count)),
            },
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "float" => match attribute_value {
            StringToken::String(value) => match value.parse() {
                Ok(value) => Ok(Attribute::Float(value)),
                Err(_) => Err(Keyvalues2SerializationError::FailedToParseFloat(reader.line_count)),
            },
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "bool" => match attribute_value {
            StringToken::String(value) => match value.parse::<u8>() {
                Ok(value) => Ok(Attribute::Boolean(value != 0)),
                Err(_) => Err(Keyvalues2SerializationError::FailedToParseInteger(reader.line_count)),
            },
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "string" => match attribute_value {
            StringToken::String(value) => Ok(Attribute::String(value)),
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "binary" => match attribute_value {
            StringToken::String(value) => {
                let mut block = BinaryBlock::default();

                for byte in value.chars().filter(|c| !c.is_whitespace()).collect::<Vec<char>>().chunks(2) {
                    let byte = byte.iter().collect::<String>();
                    block
                        .data
                        .push(u8::from_str_radix(&byte, 16).map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?);
                }

                Ok(Attribute::Binary(block))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "elementid" => match attribute_value {
            StringToken::String(value) => match value.parse() {
                Ok(value) => Ok(Attribute::ObjectId(value)),
                Err(_) => Err(Keyvalues2SerializationError::FailedToParseUUID(reader.line_count)),
            },
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "time" => match attribute_value {
            StringToken::String(value) => match value.parse() {
                Ok(value) => Ok(Attribute::Time(Duration::from_secs_f64(value))),
                Err(_) => Err(Keyvalues2SerializationError::FailedToParseFloat(reader.line_count)),
            },
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "color" => match attribute_value {
            StringToken::String(value) => {
                let mut color = Color::default();

                let mut tokens = value.split_whitespace();

                color.red = tokens
                    .next()
                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                    .parse()
                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                color.green = tokens
                    .next()
                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                    .parse()
                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                color.blue = tokens
                    .next()
                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                    .parse()
                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                color.alpha = tokens
                    .next()
                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                    .parse()
                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                Ok(Attribute::Color(color))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "vector2" => match attribute_value {
            StringToken::String(value) => {
                let mut tokens = value.split_whitespace();

                Ok(Attribute::Vector2(Vector2 {
                    x: get_float_token(reader, &mut tokens)?,
                    y: get_float_token(reader, &mut tokens)?,
                }))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "vector3" => match attribute_value {
            StringToken::String(value) => {
                let mut tokens = value.split_whitespace();

                Ok(Attribute::Vector3(Vector3 {
                    x: get_float_token(reader, &mut tokens)?,
                    y: get_float_token(reader, &mut tokens)?,
                    z: get_float_token(reader, &mut tokens)?,
                }))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "vector4" => match attribute_value {
            StringToken::String(value) => {
                let mut tokens = value.split_whitespace();

                Ok(Attribute::Vector4(Vector4 {
                    x: get_float_token(reader, &mut tokens)?,
                    y: get_float_token(reader, &mut tokens)?,
                    z: get_float_token(reader, &mut tokens)?,
                    w: get_float_token(reader, &mut tokens)?,
                }))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "qangle" => match attribute_value {
            StringToken::String(value) => {
                let mut tokens = value.split_whitespace();

                Ok(Attribute::Angle(Angle {
                    pitch: get_float_token(reader, &mut tokens)?,
                    yaw: get_float_token(reader, &mut tokens)?,
                    roll: get_float_token(reader, &mut tokens)?,
                }))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "quaternion" => match attribute_value {
            StringToken::String(value) => {
                let mut tokens = value.split_whitespace();

                Ok(Attribute::Quaternion(Quaternion {
                    x: get_float_token(reader, &mut tokens)?,
                    y: get_float_token(reader, &mut tokens)?,
                    z: get_float_token(reader, &mut tokens)?,
                    w: get_float_token(reader, &mut tokens)?,
                }))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "matrix" => match attribute_value {
            StringToken::String(value) => {
                let mut tokens = value.split_whitespace();

                Ok(Attribute::Matrix(Matrix {
                    entries: [
                        [
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                        ],
                        [
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                        ],
                        [
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                        ],
                        [
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                            get_float_token(reader, &mut tokens)?,
                        ],
                    ],
                }))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "element_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(StringToken::String(attribute_class)) => {
                            let element_token = reader.next_token()?;

                            values.push(match element_token {
                                Some(StringToken::String(element_id)) => {
                                    if element_id.is_empty() {
                                        return Ok(Attribute::Element(None));
                                    }

                                    let id = UUID::from_str(&element_id).map_err(|_| Keyvalues2SerializationError::FailedToParseUUID(reader.line_count))?;

                                    match elements.entry(id) {
                                        indexmap::map::Entry::Occupied(occupied_entry) => Some(occupied_entry.get().clone()),
                                        indexmap::map::Entry::Vacant(vacant_entry) => {
                                            let element = Element::default();
                                            vacant_entry.insert(element.clone());
                                            Some(element)
                                        }
                                    }
                                }
                                Some(StringToken::OpenBrace) => Some(read_element(reader, attribute_class, elements)?),
                                Some(_) => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                                None => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                            });
                        }
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::ElementArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "int_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => match value.parse() {
                                Ok(value) => values.push(value),
                                Err(_) => return Err(Keyvalues2SerializationError::FailedToParseInteger(reader.line_count)),
                            },
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::IntegerArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "float_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => match value.parse() {
                                Ok(value) => values.push(value),
                                Err(_) => return Err(Keyvalues2SerializationError::FailedToParseFloat(reader.line_count)),
                            },
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::FloatArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "bool_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => match value.parse::<u8>() {
                                Ok(value) => values.push(value != 0),
                                Err(_) => return Err(Keyvalues2SerializationError::FailedToParseInteger(reader.line_count)),
                            },
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::BooleanArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "string_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => values.push(value),
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::StringArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "binary_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut block = BinaryBlock::default();

                                for byte in value.chars().filter(|c| !c.is_whitespace()).collect::<Vec<char>>().chunks(2) {
                                    let byte = byte.iter().collect::<String>();
                                    block.data.push(
                                        u8::from_str_radix(&byte, 16).map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?,
                                    );
                                }

                                values.push(block);
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::BinaryArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "elementid_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => match value.parse() {
                                Ok(value) => values.push(value),
                                Err(_) => return Err(Keyvalues2SerializationError::FailedToParseUUID(reader.line_count)),
                            },
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::ObjectIdArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "time_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => match value.parse() {
                                Ok(value) => values.push(Duration::from_secs_f64(value)),
                                Err(_) => return Err(Keyvalues2SerializationError::FailedToParseFloat(reader.line_count)),
                            },
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::TimeArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "color_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut color = Color::default();

                                let mut tokens = value.split_whitespace();

                                color.red = tokens
                                    .next()
                                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                                    .parse()
                                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                                color.green = tokens
                                    .next()
                                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                                    .parse()
                                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                                color.blue = tokens
                                    .next()
                                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                                    .parse()
                                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                                color.alpha = tokens
                                    .next()
                                    .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
                                    .parse()
                                    .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))?;

                                values.push(color);
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::ColorArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "vector2_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut tokens = value.split_whitespace();

                                values.push(Vector2 {
                                    x: get_float_token(reader, &mut tokens)?,
                                    y: get_float_token(reader, &mut tokens)?,
                                });
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::Vector2Array(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "vector3_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut tokens = value.split_whitespace();

                                values.push(Vector3 {
                                    x: get_float_token(reader, &mut tokens)?,
                                    y: get_float_token(reader, &mut tokens)?,
                                    z: get_float_token(reader, &mut tokens)?,
                                });
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::Vector3Array(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "vector4_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut tokens = value.split_whitespace();

                                values.push(Vector4 {
                                    x: get_float_token(reader, &mut tokens)?,
                                    y: get_float_token(reader, &mut tokens)?,
                                    z: get_float_token(reader, &mut tokens)?,
                                    w: get_float_token(reader, &mut tokens)?,
                                });
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::Vector4Array(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "qangle_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut tokens = value.split_whitespace();

                                values.push(Angle {
                                    pitch: get_float_token(reader, &mut tokens)?,
                                    yaw: get_float_token(reader, &mut tokens)?,
                                    roll: get_float_token(reader, &mut tokens)?,
                                });
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::AngleArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "quaternion_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut tokens = value.split_whitespace();

                                values.push(Quaternion {
                                    x: get_float_token(reader, &mut tokens)?,
                                    y: get_float_token(reader, &mut tokens)?,
                                    z: get_float_token(reader, &mut tokens)?,
                                    w: get_float_token(reader, &mut tokens)?,
                                });
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::QuaternionArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        "matrix_array" => match attribute_value {
            StringToken::OpenBracket => {
                let mut values = Vec::new();

                loop {
                    match reader.next_token()? {
                        Some(StringToken::CloseBracket) => break,
                        Some(value) => match value {
                            StringToken::String(value) => {
                                let mut tokens = value.split_whitespace();

                                values.push(Matrix {
                                    entries: [
                                        [
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                        ],
                                        [
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                        ],
                                        [
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                        ],
                                        [
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                            get_float_token(reader, &mut tokens)?,
                                        ],
                                    ],
                                });
                            }
                            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                    }
                }

                Ok(Attribute::MatrixArray(values))
            }
            _ => Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        },
        _ => match attribute_value {
            StringToken::OpenBrace => Ok(Attribute::Element(Some(read_element(reader, attribute_type.to_string(), elements)?))),
            _ => Err(Keyvalues2SerializationError::UnknownAttribute(reader.line_count)),
        },
    }
}

fn get_float_token<T: BufRead>(reader: &mut StringReader<T>, stream: &mut SplitWhitespace<'_>) -> Result<f32, Keyvalues2SerializationError> {
    stream
        .next()
        .ok_or(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))?
        .parse()
        .map_err(|_| Keyvalues2SerializationError::FailedToParseInteger(reader.line_count))
}

fn read_element<T: BufRead>(
    reader: &mut StringReader<T>,
    class: String,
    elements: &mut IndexMap<UUID, Element>,
) -> Result<Element, Keyvalues2SerializationError> {
    let mut attributes = IndexMap::new();
    let mut element_id = None;
    let mut element_name = None;

    while let Some(token) = reader.next_token()? {
        match token {
            StringToken::String(attribute_name) => {
                let attribute_type = match reader.next_token()? {
                    Some(StringToken::String(attribute_type)) => attribute_type,
                    _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                };

                let attribute_value = match reader.next_token()? {
                    Some(value) => value,
                    None => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                };

                if &attribute_name == "name" {
                    if attribute_type != "string" {
                        return Err(Keyvalues2SerializationError::AttributeType(reader.line_count));
                    }

                    element_name = Some(match attribute_value {
                        StringToken::String(value) => value,
                        _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                    });

                    continue;
                }

                if &attribute_name == "id" {
                    if attribute_type != "elementid" {
                        return Err(Keyvalues2SerializationError::AttributeType(reader.line_count));
                    }

                    let id = match attribute_value {
                        StringToken::String(value) => match UUID::from_str(&value) {
                            Ok(id) => id,
                            Err(_) => return Err(Keyvalues2SerializationError::FailedToParseUUID(reader.line_count)),
                        },
                        _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                    };

                    match element_id {
                        Some(id) => {
                            return Err(Keyvalues2SerializationError::DuplicateElementId(id));
                        }
                        None => element_id = Some(id),
                    }
                    continue;
                }

                let created_attribute = read_attribute(reader, &attribute_type, attribute_value, elements)?;

                attributes.insert(attribute_name, created_attribute);
            }
            StringToken::CloseBrace => {
                let element_id = match element_id {
                    Some(id) => id,
                    None => UUID::new_v4(),
                };

                let mut element = match elements.entry(element_id) {
                    indexmap::map::Entry::Occupied(occupied_entry) => occupied_entry.get().clone(),
                    indexmap::map::Entry::Vacant(vacant_entry) => {
                        let element = Element::default();
                        vacant_entry.insert(element.clone());
                        element
                    }
                };

                if let Some(name) = element_name {
                    element.set_name(name);
                }

                element.set_class(class);

                for (name, attribute) in attributes {
                    element.set_attribute(name, attribute);
                }

                return Ok(element);
            }
            _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
        }
    }

    Err(Keyvalues2SerializationError::InvalidAttribute(reader.line_count))
}

pub struct KeyValues2Serializer;

impl Serializer for KeyValues2Serializer {
    type Error = Keyvalues2SerializationError;

    fn name() -> &'static str {
        "keyvalues2"
    }

    fn version() -> i32 {
        1
    }

    fn serialize(buffer: &mut impl Write, header: &Header, root: &Element) -> Result<(), Self::Error> {
        let mut writer = StringWriter::new(buffer);
        writer.write_raw(&header.create_header(Self::name(), Self::version()))?;

        fn collect_elements(root: Element, elements: &mut IndexMap<Element, (UUID, usize)>) {
            elements.insert(root.clone(), (UUID::new_v4(), if elements.is_empty() { 1 } else { 0 }));

            for attribute in root.get_attributes().values() {
                match attribute {
                    Attribute::Element(value) => match value {
                        Some(element) => {
                            if let Some((_, count)) = elements.get_mut(element) {
                                *count += 1;
                                continue;
                            }
                            collect_elements(element.clone(), elements);
                        }
                        None => continue,
                    },
                    Attribute::ElementArray(values) => {
                        for value in values {
                            match value {
                                Some(element) => {
                                    if let Some((_, count)) = elements.get_mut(element) {
                                        *count += 1;
                                        continue;
                                    }
                                    collect_elements(element.clone(), elements);
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
        collect_elements(root.clone(), &mut collected_elements);

        for (element, (id, use_count)) in &collected_elements {
            if *use_count == 0 {
                continue;
            }

            writer.write_line(&format!("{:?}", element.get_class()))?;
            writer.write_open_brace()?;
            writer.write_line(&format!("\"id\" \"elementid\" \"{}\"", id))?;
            writer.write_line(&format!("\"name\" \"string\" \"{}\"", element.get_name()))?;
            writer.write_attributes(element, &collected_elements)?;
            writer.write_close_brace()?;
            writer.write_line("")?;
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(Keyvalues2SerializationError::WrongEncoding);
        }

        if version < 1 || version > Self::version() {
            return Err(Keyvalues2SerializationError::InvalidEncodingVersion);
        }

        let mut reader = StringReader::new(buffer);
        let mut elements = IndexMap::new();
        let mut root = None;

        while let Some(token) = reader.next_token()? {
            match token {
                StringToken::String(class) => match reader.next_token()? {
                    Some(StringToken::OpenBrace) => {
                        if root.is_none() {
                            root = Some(read_element(&mut reader, class, &mut elements)?);
                            continue;
                        }

                        read_element(&mut reader, class, &mut elements)?;
                    }
                    Some(_) => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
                    None => return Err(Keyvalues2SerializationError::UnfinishedAttribute),
                },
                _ => return Err(Keyvalues2SerializationError::InvalidToken(reader.line_count)),
            }
        }

        Ok(root.unwrap_or_default())
    }
}

pub struct KeyValues2FlatSerializer;

impl Serializer for KeyValues2FlatSerializer {
    type Error = Keyvalues2SerializationError;

    fn name() -> &'static str {
        "keyvalues2_flat"
    }

    fn version() -> i32 {
        1
    }

    fn serialize(buffer: &mut impl Write, header: &Header, root: &Element) -> Result<(), Self::Error> {
        let mut writer = StringWriter::new(buffer);
        writer.write_raw(&header.create_header(Self::name(), Self::version()))?;

        fn collect_elements(root: Element, elements: &mut IndexMap<Element, (UUID, usize)>) {
            elements.insert(root.clone(), (UUID::new_v4(), 1));

            for attribute in root.get_attributes().values() {
                match attribute {
                    Attribute::Element(value) => match value {
                        Some(element) => {
                            if elements.get_mut(element).is_some() {
                                continue;
                            }
                            collect_elements(element.clone(), elements);
                        }
                        None => continue,
                    },
                    Attribute::ElementArray(values) => {
                        for value in values {
                            match value {
                                Some(element) => {
                                    if elements.get_mut(element).is_some() {
                                        continue;
                                    }
                                    collect_elements(element.clone(), elements);
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
        collect_elements(root.clone(), &mut collected_elements);

        for (element, (id, _)) in &collected_elements {
            writer.write_line(&format!("{:?}", element.get_class()))?;
            writer.write_open_brace()?;
            writer.write_line(&format!("\"id\" \"elementid\" \"{}\"", id))?;
            writer.write_line(&format!("\"name\" \"string\" \"{}\"", element.get_name()))?;
            writer.write_attributes(element, &collected_elements)?;
            writer.write_close_brace()?;
            writer.write_line("")?;
        }

        Ok(())
    }

    fn deserialize(buffer: &mut impl BufRead, encoding: String, version: i32) -> Result<Element, Self::Error> {
        if encoding != Self::name() {
            return Err(Keyvalues2SerializationError::WrongEncoding);
        }

        if version < 1 || version > Self::version() {
            return Err(Keyvalues2SerializationError::InvalidEncodingVersion);
        }

        KeyValues2Serializer::deserialize(buffer, String::from(KeyValues2Serializer::name()), KeyValues2Serializer::version())
    }
}
