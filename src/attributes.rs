use std::time::Duration;
use uuid::Uuid as UUID;

pub trait Attribute {
    fn to_attribute(self) -> DMAttribute;
    fn from_attribute(value: &DMAttribute) -> Option<&Self>
    where
        Self: Sized;
}

#[derive(Clone, Debug)]
pub struct Binary {
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
pub enum DMAttribute {
    Element(UUID),
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Binary(Binary),
    Id(UUID),
    Time(Duration),
    Color(Color),
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4(Vector4),
    QAngle(QAngle),
    Quaternion(Quaternion),
    Matrix(Matrix),

    ElementArray(Vec<UUID>),
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BoolArray(Vec<bool>),
    StringArray(Vec<String>),
    BinaryArray(Vec<Binary>),
    IdArray(Vec<UUID>),
    TimeArray(Vec<Duration>),
    ColorArray(Vec<Color>),
    Vector2Array(Vec<Vector2>),
    Vector3Array(Vec<Vector3>),
    Vector4Array(Vec<Vector4>),
    QAngleArray(Vec<QAngle>),
    QuaternionArray(Vec<Quaternion>),
    MatrixArray(Vec<Matrix>),
}

impl Attribute for i32 {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Int(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Int(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for f32 {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Float(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Float(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for bool {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Bool(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Bool(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for String {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::String(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::String(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Binary {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Binary(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Binary(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for UUID {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Id(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Id(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Duration {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Time(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Time(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Color {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Color(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Color(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vector2 {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Vector2(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Vector2(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vector3 {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Vector3(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Vector3(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vector4 {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Vector4(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Vector4(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for QAngle {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::QAngle(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::QAngle(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Quaternion {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Quaternion(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Quaternion(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Matrix {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Matrix(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Matrix(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<i32> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::IntArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::IntArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<f32> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::FloatArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::FloatArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<bool> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::BoolArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::BoolArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<String> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::StringArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::StringArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Binary> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::BinaryArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::BinaryArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<UUID> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::IdArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::IdArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Duration> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::TimeArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::TimeArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Color> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::ColorArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::ColorArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Vector2> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Vector2Array(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Vector2Array(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Vector3> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Vector3Array(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Vector3Array(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Vector4> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::Vector4Array(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::Vector4Array(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<QAngle> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::QAngleArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::QAngleArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Quaternion> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::QuaternionArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::QuaternionArray(value) => Some(value),
            _ => None,
        }
    }
}

impl Attribute for Vec<Matrix> {
    fn to_attribute(self) -> DMAttribute {
        DMAttribute::MatrixArray(self)
    }

    fn from_attribute(value: &DMAttribute) -> Option<&Self> {
        match value {
            DMAttribute::MatrixArray(value) => Some(value),
            _ => None,
        }
    }
}
