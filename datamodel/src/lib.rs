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
