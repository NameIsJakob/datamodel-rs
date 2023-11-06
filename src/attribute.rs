use std::rc::Rc;

use uuid::Uuid;

use crate::element::DmElement;

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
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug)]
pub enum Attribute {
    Unknown,
    Element(Rc<DmElement>),
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Void(Vec<u8>),
    ObjectId(Uuid),
    Color(Color),
    Vector2(Vector2),
    Vector3(Vector3),
    Vector4(Vector4),
    QAngle(Vector3),
    Quaternion(Vector4),
    Matrix([f32; 16]),
    ElementArray(Vec<Rc<DmElement>>),
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BoolArray(Vec<bool>),
    StringArray(Vec<String>),
    VoidArray(Vec<Vec<u8>>),
    ObjectIdArray(Vec<Uuid>),
    ColorArray(Vec<Color>),
    Vector2Array(Vec<Vector2>),
    Vector3Array(Vec<Vector3>),
    Vector4Array(Vec<Vector4>),
    QAngleArray(Vec<Vector3>),
    QuaternionArray(Vec<Vector4>),
    MatrixArray(Vec<[f32; 16]>),
}
