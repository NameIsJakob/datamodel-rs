use std::{cell::RefCell, rc::Rc, time::Duration};

use uuid::Uuid as UUID;

use crate::Element;

pub type Binary = Vec<u8>;

#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Angle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Matrix {
    pub elements: [[f32; 4]; 4],
}

#[derive(Clone, Debug)]
pub enum Attribute {
    Element(Option<Rc<RefCell<Element>>>),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    String(String),
    Binary(Binary),
    ObjectId(UUID),
    Time(Duration),
    Color(Color),
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4(Vector4),
    Angle(Angle),
    Quaternion(Quaternion),
    Matrix(Matrix),
    ElementArray(Vec<Option<Rc<RefCell<Element>>>>),
    IntegerArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BooleanArray(Vec<bool>),
    StringArray(Vec<String>),
    BinaryArray(Vec<Binary>),
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

macro_rules! declare_attribute {
    ($qualifier:ty, $attribute:path, $array:path) => {
        impl From<$qualifier> for Attribute {
            fn from(value: $qualifier) -> Self {
                $attribute(value)
            }
        }

        impl From<Vec<$qualifier>> for Attribute {
            fn from(value: Vec<$qualifier>) -> Self {
                $array(value)
            }
        }
    };
}

declare_attribute!(Option<Rc<RefCell<Element>>>, Attribute::Element, Attribute::ElementArray);
declare_attribute!(i32, Attribute::Integer, Attribute::IntegerArray);
declare_attribute!(f32, Attribute::Float, Attribute::FloatArray);
declare_attribute!(bool, Attribute::Boolean, Attribute::BooleanArray);
declare_attribute!(String, Attribute::String, Attribute::StringArray);
declare_attribute!(Binary, Attribute::Binary, Attribute::BinaryArray);
declare_attribute!(UUID, Attribute::ObjectId, Attribute::ObjectIdArray);
declare_attribute!(Duration, Attribute::Time, Attribute::TimeArray);
declare_attribute!(Color, Attribute::Color, Attribute::ColorArray);
declare_attribute!(Vector2, Attribute::Vector2, Attribute::Vector2Array);
declare_attribute!(Vector3, Attribute::Vector3, Attribute::Vector3Array);
declare_attribute!(Vector4, Attribute::Vector4, Attribute::Vector4Array);
declare_attribute!(Angle, Attribute::Angle, Attribute::AngleArray);
declare_attribute!(Quaternion, Attribute::Quaternion, Attribute::QuaternionArray);
declare_attribute!(Matrix, Attribute::Matrix, Attribute::MatrixArray);
