use std::rc::Rc;

use crate::element::DmElement;
use crate::{BinaraySerializer, DmHeader, SerializingError};

pub trait Serializer {
    fn serialize(&self, root: Rc<DmElement>, header: &DmHeader) -> Result<Vec<u8>, SerializingError>;
    fn unserialize(&self, data: Vec<u8>) -> Result<Rc<DmElement>, SerializingError>;
}

pub fn get_serializer(header: &DmHeader) -> Result<Box<dyn Serializer>, SerializingError> {
    match header.encoding_name.as_str() {
        "binary" => Ok(Box::new(BinaraySerializer {})),
        _ => Err(SerializingError::new("Not Supported encoding!")),
    }
}
