use std::time::Duration;
use uuid::Uuid as UUID;

use crate::elements::Element;

#[derive(Clone, Debug)]
pub struct ObjectId {
    pub id: UUID,
}

#[derive(Clone, Debug)]
pub struct BinaryData {
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Debug)]
pub struct QAngle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Debug)]
pub struct Matrix {
    pub entries: [f32; 16],
}

#[derive(Clone, Debug)]
pub enum Attribute {
    ElementData(Element),
    Element(UUID),
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Binary(BinaryData),
    Id(ObjectId),
    Time(Duration),
    Color(Color),
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4(Vector4),
    QAngle(QAngle),
    Quaternion(Quaternion),
    Matrix(Matrix),

    ElementDataArray(Vec<Element>),
    ElementArray(Vec<UUID>),
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BoolArray(Vec<bool>),
    StringArray(Vec<String>),
    BinaryArray(Vec<BinaryData>),
    IdArray(Vec<ObjectId>),
    TimeArray(Vec<Duration>),
    ColorArray(Vec<Color>),
    Vector2Array(Vec<Vector2>),
    Vector3Array(Vec<Vector3>),
    Vector4Array(Vec<Vector4>),
    QAngleArray(Vec<QAngle>),
    QuaternionArray(Vec<Quaternion>),
    MatrixArray(Vec<Matrix>),
}

macro_rules! attribute_type {
    ($qualifier:ty, $attribute:path) => {
        impl From<$qualifier> for Attribute {
            fn from(value: $qualifier) -> Self {
                $attribute(value)
            }
        }

        impl<'a> TryFrom<&'a Attribute> for &'a $qualifier {
            type Error = InvalidAttribute;

            fn try_from(attr: &'a Attribute) -> Result<Self, Self::Error> {
                match attr {
                    $attribute(value) => Ok(value),
                    _ => Err(InvalidAttribute {}),
                }
            }
        }
    };
}

pub struct InvalidAttribute {}

impl From<Element> for Attribute {
    fn from(element: Element) -> Self {
        Attribute::ElementData(element)
    }
}

impl From<Vec<Element>> for Attribute {
    fn from(elements: Vec<Element>) -> Self {
        Attribute::ElementDataArray(elements)
    }
}

attribute_type!(UUID, Attribute::Element);
attribute_type!(i32, Attribute::Int);
attribute_type!(f32, Attribute::Float);
attribute_type!(bool, Attribute::Bool);
attribute_type!(String, Attribute::String);
attribute_type!(BinaryData, Attribute::Binary);
attribute_type!(ObjectId, Attribute::Id);
attribute_type!(Duration, Attribute::Time);
attribute_type!(Color, Attribute::Color);
attribute_type!(Vector2, Attribute::Vector2);
attribute_type!(Vector3, Attribute::Vector3);
attribute_type!(Vector4, Attribute::Vector4);
attribute_type!(QAngle, Attribute::QAngle);
attribute_type!(Quaternion, Attribute::Quaternion);
attribute_type!(Matrix, Attribute::Matrix);

// TODO: Make this automatically generated in the macro.
attribute_type!(Vec<UUID>, Attribute::ElementArray);
attribute_type!(Vec<i32>, Attribute::IntArray);
attribute_type!(Vec<f32>, Attribute::FloatArray);
attribute_type!(Vec<bool>, Attribute::BoolArray);
attribute_type!(Vec<String>, Attribute::StringArray);
attribute_type!(Vec<BinaryData>, Attribute::BinaryArray);
attribute_type!(Vec<ObjectId>, Attribute::IdArray);
attribute_type!(Vec<Duration>, Attribute::TimeArray);
attribute_type!(Vec<Color>, Attribute::ColorArray);
attribute_type!(Vec<Vector2>, Attribute::Vector2Array);
attribute_type!(Vec<Vector3>, Attribute::Vector3Array);
attribute_type!(Vec<Vector4>, Attribute::Vector4Array);
attribute_type!(Vec<QAngle>, Attribute::QAngleArray);
attribute_type!(Vec<Quaternion>, Attribute::QuaternionArray);
attribute_type!(Vec<Matrix>, Attribute::MatrixArray);
