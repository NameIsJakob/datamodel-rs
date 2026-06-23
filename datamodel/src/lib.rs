//! # Data Model
//!
//! A way to interface with Valve's datamodel formats with elements and run-time type information attributes.
//!
//! # Quick Start
//! Code to load a dmx file and print the header.
//! ```
//! let file = std::fs::File::open("file.dmx").unwrap();
//! let mut file_buffer = std::io::BufReader::new(file);
//! let (header, _) = datamodel::deserialize(&mut file_buffer).unwrap();
//! println!("Dmx file format is {} with version {}.", header.format, header.format_version);
//! ```
//! Code to create an Element and serialize it to a buffer.
//! ```
//! use datamodel::Serializer;
//!
//! let mut root = datamodel::Element::default();
//! root.set_attribute("name", String::from("The Angle").into_attribute());
//! root.set_attribute("rotation", 43.46f32.into_attribute());
//! let file = std::fs::File::create("file.dmx").unwrap();
//! let mut buffer = std::io::BufWriter::new(file);
//! let header = datamodel::Header {
//!     format: String::from("Rotation"),
//!     format_version: 7,
//! };
//! BinarySerializer::serialize(&mut buffer, &header, &root).unwrap();
//! ```
//!
//! # Features
//! - [mint](https://crates.io/crates/mint) Allow for math library interoperability for math attributes.
//! - [datamodel-derive](https://crates.io/crates/datamodel-derive) A derive marco to implement ElementClass.

pub mod attribute;

mod element;
pub use element::Element;
pub use element::ElementClass;

pub mod serializers;

mod serializing;
pub use serializing::FileHeaderError;
pub use serializing::Header;
pub use serializing::SerializationError;
pub use serializing::Serializer;
pub use serializing::deserialize;
