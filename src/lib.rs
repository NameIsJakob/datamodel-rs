//! Data Model
//!
//! # Example
//! ```no_run
//! use datamodel::{
//!     serializers::KeyValues2Serializer,
//!     Element,
//!     Header,
//!     Serializer
//! };
//!
//! let mut root = Element::named("root");
//! root.set_value("The Value", 84);
//! root.set_value("The Size", 0.4);
//!
//! let file = std::fs::File::create("example.dmx").unwrap();
//! let mut writer = std::io::BufWriter::new(file);
//! let _ = KeyValues2Serializer::serialize(&mut writer, &Header::default(), &root);
//!```
pub mod attribute;
pub use attribute::Attribute;

mod element;
pub use element::Element;

pub mod serializers;

mod serializing;
pub use serializing::deserialize;
pub use serializing::Header;
pub use serializing::Serializer;
