//! # Data Model
//! Data Model is a structured container for key-value data, organized into elements and attributes.
//!
//! This is a implementation of data structure designed by Valve Corporation used by source and source 2.
//! Derived from XML, it stores data generically while allowing strong typing value accessing.
//! Elements store attribute data and can reference other elements as a attribute for reuse.
//!
//! ## Usage
//! All models must have a root element. Here is how to create a root element.
//! ```
//! use datamodel::Element;
//!
//! let mut root = Element::named("root");
//! ```
//! To add attributes to a element.
//! ```
//! root.set_value("Length", 42);
//! root.set_value("Size", 10.45);
//! ```
//! To read values from elements.
//! ```
//! let length = root.get_value::<i32>("Length").unwrap();
//! let size = root.get_value::<f32>("Size").unwrap();
//! ```
//! To serialize the model to a file.
//! ```
//! use datamodel::{serializers::KeyValues2Serializer, Header, Serializer};
//!
//! let header = Header::new("example", 4);
//! let file = std::fs::File::create("data.dmx").unwrap();
//! let mut file_buffer = std::io::BufWriter::new(file);
//!
//! KeyValues2Serializer::serialize(&mut file_buffer, &header, &root).unwrap();
//! ```
//! To deserialize the model from a file.
//! ```
//! use datamodel::deserialize;
//!
//! let file = std::fs::File::open("data.dmx").unwrap();
//! let mut file_buffer = std::io::BufReader::new(file);
//!
//! let (header, root) = deserialize(&mut file_buffer, &header, &root).unwrap();
//! ```

pub mod attribute;

mod element;
pub use element::Element;

pub mod serializers;

mod serializing;
pub use serializing::deserialize;
pub use serializing::FileHeaderError;
pub use serializing::Header;
pub use serializing::SerializationError;
pub use serializing::Serializer;
