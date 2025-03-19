//! Structures for serializing and deserializing.

mod binary;
pub use binary::BinarySerializationError;
pub use binary::BinarySerializer;

mod keyvalues2;
pub use keyvalues2::KeyValues2FlatSerializer;
pub use keyvalues2::KeyValues2Serializer;
pub use keyvalues2::Keyvalues2SerializationError;
