use std::fmt::{self, Display, Formatter};

pub type Binary = Vec<u8>;

#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{} {} {} {}", self.r, self.g, self.b, self.a))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Display for Vector2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{} {}", self.x, self.y))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Display for Vector3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{} {} {}", self.x, self.y, self.z))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Display for Vector4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{} {} {} {}", self.x, self.y, self.z, self.w))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Angle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Display for Angle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{} {} {}", self.x, self.y, self.z))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Display for Quaternion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{} {} {} {}", self.x, self.y, self.z, self.w))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Matrix {
    pub elements: [[f32; 4]; 4],
}

impl Display for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            self.elements[0][0],
            self.elements[0][1],
            self.elements[0][2],
            self.elements[0][3],
            self.elements[1][0],
            self.elements[1][1],
            self.elements[1][2],
            self.elements[1][3],
            self.elements[2][0],
            self.elements[2][1],
            self.elements[2][2],
            self.elements[2][3],
            self.elements[3][0],
            self.elements[3][1],
            self.elements[3][2],
            self.elements[3][3]
        ))
    }
}
