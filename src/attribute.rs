//! The supported types that data model uses.

use std::time::Duration;

use uuid::Uuid as UUID;

use crate::Element;

/// The enum represents a valid attribute supported by dmx.
#[derive(Clone, Debug)]
pub enum Attribute {
    Element(Option<Element>),
    UInt64(u64),
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
    UInt64Array(Vec<u64>),
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

/// A 2 dimensional vector.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

/// A 3 dimensional vector.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A 4 dimensional vector.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// Tait–Bryan angles in degrees.
#[derive(Clone, Copy, Debug, Default)]
pub struct Angle {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

/// A rotation quaternion.
#[derive(Clone, Copy, Debug, Default)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// A 4 by 4 transformation matrix.
#[derive(Clone, Copy, Debug, Default)]
pub struct Matrix(pub [[f32; 4]; 4]);

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
declare_attribute!(u64, Attribute::UInt64, Attribute::UInt64Array);
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
