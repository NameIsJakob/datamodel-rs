mod attribute;

pub use attribute::Angle;
pub use attribute::Attribute;
pub use attribute::Binary;
pub use attribute::Color;
pub use attribute::Matrix;
pub use attribute::Quaternion;
pub use attribute::Vector2;
pub use attribute::Vector3;
pub use attribute::Vector4;

mod element;

pub use element::Element;

mod serializing;

pub use serializing::deserialize;
pub use serializing::serialize;
pub use serializing::Header;
pub use serializing::SerializationError;
pub use serializing::SerializationFormat;
