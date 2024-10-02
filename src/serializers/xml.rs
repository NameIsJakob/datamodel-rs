use std::io::{BufRead, Write};

use thiserror::Error as ThisError;

use crate::{Element, Header, Serializer};

#[derive(Debug, ThisError)]
pub enum XMLSerializationError {}

pub struct XMLSerializer;

impl Serializer for XMLSerializer {
    type Error = XMLSerializationError;

    fn name() -> &'static str {
        "xml"
    }

    fn version() -> i32 {
        1
    }

    fn serialize(_buffer: &mut impl Write, _header: &Header, _root: &Element) -> Result<(), Self::Error> {
        todo!("Implement XMLSerializer::serialize")
    }

    fn deserialize(_buffer: &mut impl BufRead, _encoding: String, _version: i32) -> Result<Element, Self::Error> {
        todo!("Implement XMLSerializer::deserialize")
    }
}

pub struct XMLFlatSerializer;

impl Serializer for XMLFlatSerializer {
    type Error = XMLSerializationError;

    fn name() -> &'static str {
        "xml_flat"
    }

    fn version() -> i32 {
        1
    }

    fn serialize(_buffer: &mut impl Write, _header: &Header, _root: &Element) -> Result<(), Self::Error> {
        todo!("Implement XMLFlatSerializer::serialize")
    }

    fn deserialize(_buffer: &mut impl BufRead, _encoding: String, _version: i32) -> Result<Element, Self::Error> {
        todo!("Implement XMLFlatSerializer::deserialize")
    }
}
