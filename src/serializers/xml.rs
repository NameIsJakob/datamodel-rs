use std::{fs::File, io::BufReader};

use thiserror::Error as ThisError;

use crate::{Element, Header, Serializer};

#[derive(Debug, ThisError)]
pub enum XMLSerializationError {}

pub struct XMLSerializer;

impl Serializer for XMLSerializer {
    type Error = XMLSerializationError;

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error> {
        todo!()
    }

    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error> {
        todo!()
    }

    fn name() -> &'static str {
        "xml"
    }

    fn version() -> i32 {
        1
    }
}

pub struct XMLFlatSerializer;

impl Serializer for XMLFlatSerializer {
    type Error = XMLSerializationError;

    fn serialize(root: Element, header: &Header) -> Result<Vec<u8>, Self::Error> {
        todo!()
    }

    fn deserialize(data: BufReader<File>) -> Result<(Header, Element), Self::Error> {
        todo!()
    }

    fn name() -> &'static str {
        "xml_flat"
    }

    fn version() -> i32 {
        1
    }
}
