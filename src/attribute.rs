//! The supported types that data model uses.

pub use chrono::Duration;
pub use uuid::Uuid as UUID;

use crate::Element;

/// The enum represents a valid attribute supported by dmx.
#[derive(Clone, Debug)]
pub enum Attribute {
    Element(Option<Element>),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    String(String),
    Binary(BinaryBlock),
    #[deprecated = "Replaced By Time Value"]
    ObjectId(UUID),
    Time(Duration),
    Color(Color),
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4(Vector4),
    Angle(Angle),
    Quaternion(Quaternion),
    Matrix(Matrix),

    ElementArray(Vec<Option<Element>>),
    IntegerArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BooleanArray(Vec<bool>),
    StringArray(Vec<String>),
    BinaryArray(Vec<BinaryBlock>),
    #[deprecated = "Replaced By Time Array Value"]
    ObjectIdArray(Vec<UUID>),
    TimeArray(Vec<Duration>),
    ColorArray(Vec<Color>),
    Vector2Array(Vec<Vector2>),
    Vector3Array(Vec<Vector3>),
    Vector4Array(Vec<Vector4>),
    AngleArray(Vec<Angle>),
    QuaternionArray(Vec<Quaternion>),
    MatrixArray(Vec<Matrix>),
}

/// Binary data.
#[derive(Clone, Debug, Default)]
pub struct BinaryBlock(pub Vec<u8>);

/// RGBA color values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

pub type Vector2 = mint::Vector2<f32>;
pub type Vector3 = mint::Vector3<f32>;
pub type Vector4 = mint::Vector4<f32>;
pub type Angle = mint::EulerAngles<f32, mint::IntraXYZ>;
pub type Quaternion = mint::Quaternion<f32>;
pub type Matrix = mint::RowMatrix4<f32>;

/// A type to get an element array from a value.
pub type ElementArray = Vec<Option<Element>>;

/// Implement conversions between [`Attribute`] and it type.
macro_rules! declare_attribute {
    ($qualifier:ty, $attribute:path, $array:path) => {
        impl From<$qualifier> for Attribute {
            fn from(value: $qualifier) -> Self {
                $attribute(value)
            }
        }

        impl TryFrom<Attribute> for $qualifier {
            type Error = ();

            fn try_from(value: Attribute) -> Result<Self, Self::Error> {
                match value {
                    $attribute(value) => Ok(value),
                    _ => Err(()),
                }
            }
        }

        impl<'a> TryFrom<&'a Attribute> for &'a $qualifier {
            type Error = ();

            fn try_from(value: &'a Attribute) -> Result<Self, Self::Error> {
                match value {
                    $attribute(value) => Ok(value),
                    _ => Err(()),
                }
            }
        }

        impl From<Vec<$qualifier>> for Attribute {
            fn from(value: Vec<$qualifier>) -> Self {
                $array(value)
            }
        }

        impl TryFrom<Attribute> for Vec<$qualifier> {
            type Error = ();

            fn try_from(value: Attribute) -> Result<Self, Self::Error> {
                match value {
                    $array(value) => Ok(value),
                    _ => Err(()),
                }
            }
        }

        impl<'a> TryFrom<&'a Attribute> for &'a Vec<$qualifier> {
            type Error = ();

            fn try_from(value: &'a Attribute) -> Result<Self, Self::Error> {
                match value {
                    $array(value) => Ok(value),
                    _ => Err(()),
                }
            }
        }
    };
}

impl From<Element> for Attribute {
    fn from(value: Element) -> Self {
        Attribute::Element(Some(value))
    }
}

impl TryFrom<Attribute> for Element {
    type Error = ();

    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        match value {
            Attribute::Element(value) => Ok(value.ok_or(())?),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Attribute> for &'a Element {
    type Error = ();

    fn try_from(value: &'a Attribute) -> Result<Self, Self::Error> {
        match value {
            Attribute::Element(value) => Ok(value.as_ref().ok_or(())?),
            _ => Err(()),
        }
    }
}

declare_attribute!(Option<Element>, Attribute::Element, Attribute::ElementArray);
declare_attribute!(i32, Attribute::Integer, Attribute::IntegerArray);
declare_attribute!(f32, Attribute::Float, Attribute::FloatArray);
declare_attribute!(bool, Attribute::Boolean, Attribute::BooleanArray);
declare_attribute!(String, Attribute::String, Attribute::StringArray);
declare_attribute!(BinaryBlock, Attribute::Binary, Attribute::BinaryArray);

impl TryFrom<Attribute> for UUID {
    type Error = ();

    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        match value {
            #[allow(deprecated)]
            Attribute::ObjectId(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Attribute> for &'a UUID {
    type Error = ();

    fn try_from(value: &'a Attribute) -> Result<Self, Self::Error> {
        match value {
            #[allow(deprecated)]
            Attribute::ObjectId(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl TryFrom<Attribute> for Vec<UUID> {
    type Error = ();

    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        match value {
            #[allow(deprecated)]
            Attribute::ObjectIdArray(value) => Ok(value),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Attribute> for &'a Vec<UUID> {
    type Error = ();

    fn try_from(value: &'a Attribute) -> Result<Self, Self::Error> {
        match value {
            #[allow(deprecated)]
            Attribute::ObjectIdArray(value) => Ok(value),
            _ => Err(()),
        }
    }
}

declare_attribute!(Duration, Attribute::Time, Attribute::TimeArray);
declare_attribute!(Color, Attribute::Color, Attribute::ColorArray);
declare_attribute!(Vector2, Attribute::Vector2, Attribute::Vector2Array);
declare_attribute!(Vector3, Attribute::Vector3, Attribute::Vector3Array);
declare_attribute!(Vector4, Attribute::Vector4, Attribute::Vector4Array);
declare_attribute!(Angle, Attribute::Angle, Attribute::AngleArray);
declare_attribute!(Quaternion, Attribute::Quaternion, Attribute::QuaternionArray);
declare_attribute!(Matrix, Attribute::Matrix, Attribute::MatrixArray);
