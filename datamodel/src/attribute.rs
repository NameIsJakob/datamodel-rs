use std::mem::transmute;

use crate::ElementClass;

use super::element::Element;
pub use uuid::Uuid as UUID;

/// A structure that holds raw binary data.
#[derive(Debug, Clone, Default)]
pub struct BinaryBlock(pub Vec<u8>);

/// A representation of time in tenths of a millisecond.
#[derive(Debug, Clone, Copy, Default)]
pub struct Time(pub i32);

impl Time {
    pub fn from_seconds(seconds: f32) -> Option<Self> {
        let tenths_of_milliseconds = (seconds * 10000.0 + 0.5).floor();
        if tenths_of_milliseconds > i32::MAX as f32 || tenths_of_milliseconds < i32::MIN as f32 {
            return None;
        }
        Some(Time(tenths_of_milliseconds as i32))
    }

    pub fn as_seconds(&self) -> f32 {
        self.0 as f32 / 10000.0
    }
}

/// A structure that 8 bit RGBA color.
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// A mathematical 2 dimensional vector.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[cfg(feature = "mint")]
impl From<mint::Point2<f32>> for Vector2 {
    fn from(v: mint::Point2<f32>) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[cfg(feature = "mint")]
impl From<Vector2> for mint::Point2<f32> {
    fn from(v: Vector2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[cfg(feature = "mint")]
impl From<mint::Vector2<f32>> for Vector2 {
    fn from(v: mint::Vector2<f32>) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[cfg(feature = "mint")]
impl From<Vector2> for mint::Vector2<f32> {
    fn from(v: Vector2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Vector2 {
    type MintType = mint::Vector2<f32>;
}

/// A mathematical 3 dimensional vector.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(feature = "mint")]
impl From<mint::Point3<f32>> for Vector3 {
    fn from(v: mint::Point3<f32>) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

#[cfg(feature = "mint")]
impl From<Vector3> for mint::Point3<f32> {
    fn from(v: Vector3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

#[cfg(feature = "mint")]
impl From<mint::Vector3<f32>> for Vector3 {
    fn from(v: mint::Vector3<f32>) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

#[cfg(feature = "mint")]
impl From<Vector3> for mint::Vector3<f32> {
    fn from(v: Vector3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Vector3 {
    type MintType = mint::Vector3<f32>;
}

/// A mathematical 4 dimensional vector.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[cfg(feature = "mint")]
impl From<mint::Vector4<f32>> for Vector4 {
    fn from(v: mint::Vector4<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }
}

#[cfg(feature = "mint")]
impl From<Vector4> for mint::Vector4<f32> {
    fn from(v: Vector4) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Vector4 {
    type MintType = mint::Vector4<f32>;
}

/// A Tait-Bryan 3 dimensional angle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Angle {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[cfg(feature = "mint")]
impl From<mint::EulerAngles<f32, mint::IntraXYZ>> for Angle {
    fn from(value: mint::EulerAngles<f32, mint::IntraXYZ>) -> Self {
        Self {
            pitch: value.b.to_degrees(),
            yaw: value.c.to_degrees(),
            roll: value.a.to_degrees(),
        }
    }
}

#[cfg(feature = "mint")]
impl From<Angle> for mint::EulerAngles<f32, mint::IntraXYZ> {
    fn from(value: Angle) -> Self {
        Self {
            a: value.roll.to_radians(),
            b: value.pitch.to_radians(),
            c: value.yaw.to_radians(),
            marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Angle {
    type MintType = mint::EulerAngles<f32, mint::IntraXYZ>;
}

/// A mathematical Quaternion.
#[derive(Debug, Clone, Copy)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quaternion {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

#[cfg(feature = "mint")]
impl From<mint::Quaternion<f32>> for Quaternion {
    fn from(v: mint::Quaternion<f32>) -> Self {
        Self {
            x: v.v.x,
            y: v.v.y,
            z: v.v.z,
            w: v.s,
        }
    }
}

#[cfg(feature = "mint")]
impl From<Quaternion> for mint::Quaternion<f32> {
    fn from(v: Quaternion) -> Self {
        Self {
            v: mint::Vector3 { x: v.x, y: v.y, z: v.z },
            s: v.w,
        }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Quaternion {
    type MintType = mint::Quaternion<f32>;
}

/// A mathematical 4 by 4 matrix.
#[derive(Debug, Clone, Copy)]
pub struct Matrix(pub [[f32; 4]; 4]);

impl Default for Matrix {
    fn default() -> Self {
        Self([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])
    }
}

#[cfg(feature = "mint")]
impl From<mint::RowMatrix4<f32>> for Matrix {
    fn from(value: mint::RowMatrix4<f32>) -> Self {
        Self([
            [value.x.x, value.x.y, value.x.z, value.x.w],
            [value.y.x, value.y.y, value.y.z, value.y.w],
            [value.z.x, value.z.y, value.z.z, value.z.w],
            [value.w.x, value.w.y, value.w.z, value.w.w],
        ])
    }
}

#[cfg(feature = "mint")]
impl From<Matrix> for mint::RowMatrix4<f32> {
    fn from(value: Matrix) -> Self {
        Self {
            x: mint::Vector4 {
                x: value.0[0][0],
                y: value.0[0][1],
                z: value.0[0][2],
                w: value.0[0][3],
            },
            y: mint::Vector4 {
                x: value.0[1][0],
                y: value.0[1][1],
                z: value.0[1][2],
                w: value.0[1][3],
            },
            z: mint::Vector4 {
                x: value.0[2][0],
                y: value.0[2][1],
                z: value.0[2][2],
                w: value.0[2][3],
            },
            w: mint::Vector4 {
                x: value.0[3][0],
                y: value.0[3][1],
                z: value.0[3][2],
                w: value.0[3][3],
            },
        }
    }
}

#[cfg(feature = "mint")]
impl From<mint::ColumnMatrix4<f32>> for Matrix {
    fn from(value: mint::ColumnMatrix4<f32>) -> Self {
        Self([
            [value.x.x, value.y.x, value.z.x, value.w.x],
            [value.x.y, value.y.y, value.z.y, value.w.y],
            [value.x.z, value.y.z, value.z.z, value.w.z],
            [value.x.w, value.y.w, value.z.w, value.w.w],
        ])
    }
}

#[cfg(feature = "mint")]
impl From<Matrix> for mint::ColumnMatrix4<f32> {
    fn from(value: Matrix) -> Self {
        Self {
            x: mint::Vector4 {
                x: value.0[0][0],
                y: value.0[1][0],
                z: value.0[2][0],
                w: value.0[3][0],
            },
            y: mint::Vector4 {
                x: value.0[0][1],
                y: value.0[1][1],
                z: value.0[2][1],
                w: value.0[3][1],
            },
            z: mint::Vector4 {
                x: value.0[0][2],
                y: value.0[1][2],
                z: value.0[2][2],
                w: value.0[3][2],
            },
            w: mint::Vector4 {
                x: value.0[0][3],
                y: value.0[1][3],
                z: value.0[2][3],
                w: value.0[3][3],
            },
        }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Matrix {
    type MintType = mint::RowMatrix4<f32>;
}

macro_rules! declare_attributes {
    ($($name:ident : $value:ty),* $(,)?) => {
        paste::paste! {
            /// A value to specify what type the attribute is.
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum AttributeType {
                $($name,)*
                $([<$name Array>],)*
            }

            /// Possible values which the attribute will store.
            #[derive(Clone, Debug)]
            pub enum AttributeValue {
                $($name($value),)*
                $([<$name Array>](Vec<$value>),)*
            }

            impl AttributeValue {
                pub fn attribute_type(&self) -> AttributeType {
                    match self {
                        $(AttributeValue::$name(_) => AttributeType::$name,)*
                        $(AttributeValue::[<$name Array>](_) => AttributeType::[<$name Array>],)*
                    }
                }
            }
        }
    };
}

declare_attributes! {
    Element: Option<Element>,
    Integer: i32,
    Float: f32,
    Boolean: bool,
    String: String,
    Binary: BinaryBlock,
    ObjectId: UUID,
    Time: Time,
    Color: Color,
    Vector2: Vector2,
    Vector3: Vector3,
    Vector4: Vector4,
    Angle: Angle,
    Quaternion: Quaternion,
    Matrix: Matrix,
    ULong: u64,
    UByte: u8,
}

#[derive(Clone, Debug)]
pub struct Attribute(AttributeValue);

impl Attribute {
    pub fn new(value: AttributeValue) -> Self {
        Self(value)
    }

    pub fn get_type(&self) -> AttributeType {
        self.0.attribute_type()
    }

    pub fn get_inner_value(&self) -> &AttributeValue {
        &self.0
    }

    pub fn get_inner_value_mut(&mut self) -> &mut AttributeValue {
        &mut self.0
    }
}

/// A trait to implement a type that stores as a attribute value.
pub trait AttributeInfo: Clone + Default {
    /// Returns the attribute type the value stores.
    fn attribute_type() -> AttributeType;
    /// Converts the value into a attribute value.
    fn into_attribute_type(self) -> AttributeValue;
    /// Converts the value into an attribute.
    fn into_attribute(self) -> Attribute {
        Attribute::new(self.into_attribute_type())
    }
    /// Gets the inner values of a attribute if its the correct attribute type.
    fn get_attribute_value(attribute: &AttributeValue) -> Option<&Self>;
    /// Gets the inner values as mutably of a attribute if its the correct attribute type.
    fn get_attribute_value_mut(attribute: &mut AttributeValue) -> Option<&mut Self>;
}

impl<Class: ElementClass> AttributeInfo for Element<Class> {
    fn attribute_type() -> AttributeType {
        AttributeType::Element
    }

    fn into_attribute_type(self) -> AttributeValue {
        AttributeValue::Element(Some(self.convert()))
    }

    fn get_attribute_value(attribute: &AttributeValue) -> Option<&Self> {
        match attribute {
            AttributeValue::Element(element) => {
                if let Some(element) = element {
                    Some(element.cast())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn get_attribute_value_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
        match attribute {
            AttributeValue::Element(element) => {
                if let Some(element) = element {
                    // SAFETY: The size of the Element does not change.
                    Some(unsafe { std::mem::transmute::<&mut Element, &mut Element<Class>>(element) })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl<Class: ElementClass> AttributeInfo for Option<Element<Class>> {
    fn attribute_type() -> AttributeType {
        AttributeType::Element
    }

    fn into_attribute_type(self) -> AttributeValue {
        AttributeValue::Element(self.map(|element| element.convert()))
    }

    fn get_attribute_value(attribute: &AttributeValue) -> Option<&Self> {
        match attribute {
            // SAFETY: The size of the Element does not change.
            AttributeValue::Element(element) => Some(unsafe { transmute::<&Option<Element>, &Option<Element<Class>>>(element) }),
            _ => None,
        }
    }

    fn get_attribute_value_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
        match attribute {
            // SAFETY: The size of the Element does not change.
            AttributeValue::Element(element) => Some(unsafe { transmute::<&mut Option<Element>, &mut Option<Element<Class>>>(element) }),
            _ => None,
        }
    }
}

impl<Class: ElementClass> AttributeInfo for Vec<Option<Element<Class>>> {
    fn attribute_type() -> AttributeType {
        AttributeType::ElementArray
    }

    fn into_attribute_type(self) -> AttributeValue {
        AttributeValue::ElementArray(unsafe { transmute::<Vec<Option<Element<Class>>>, Vec<Option<Element>>>(self) })
    }

    fn get_attribute_value(attribute: &AttributeValue) -> Option<&Self> {
        match attribute {
            // SAFETY: The size of the Element does not change.
            AttributeValue::ElementArray(element) => Some(unsafe { transmute::<&Vec<Option<Element>>, &Vec<Option<Element<Class>>>>(element) }),
            _ => None,
        }
    }

    fn get_attribute_value_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
        match attribute {
            // SAFETY: The size of the Element does not change.
            AttributeValue::ElementArray(element) => Some(unsafe { transmute::<&mut Vec<Option<Element>>, &mut Vec<Option<Element<Class>>>>(element) }),
            _ => None,
        }
    }
}

macro_rules! define_attributes_info {
    ($($name:ident : $value:ty),* $(,)?) => {
        paste::paste! {
            $(
                impl AttributeInfo for $value {
                    fn attribute_type() -> AttributeType {
                        AttributeType::$name
                    }
                    fn into_attribute_type(self) -> AttributeValue {
                        AttributeValue::$name(self)
                    }
                    fn get_attribute_value(attribute: &AttributeValue) -> Option<&Self> {
                        match attribute {
                            AttributeValue::$name(inner_value) => Some(inner_value),
                            _ => None
                        }
                    }
                    fn get_attribute_value_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
                        match attribute {
                            AttributeValue::$name(inner_value) => Some(inner_value),
                            _ => None
                        }
                    }
                }
                impl AttributeInfo for Vec<$value> {
                    fn attribute_type() -> AttributeType {
                        AttributeType::[<$name Array>]
                    }
                    fn into_attribute_type(self) -> AttributeValue {
                        AttributeValue::[<$name Array>](self)
                    }
                    fn get_attribute_value(attribute: &AttributeValue) -> Option<&Self> {
                        match attribute {
                            AttributeValue::[<$name Array>](inner_value) => Some(inner_value),
                            _ => None
                        }
                    }
                    fn get_attribute_value_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
                        match attribute {
                            AttributeValue::[<$name Array>](inner_value) => Some(inner_value),
                            _ => None
                        }
                    }
                }
            )*
        }
    };
}

define_attributes_info! {
    Integer: i32,
    Float: f32,
    Boolean: bool,
    String: String,
    Binary: BinaryBlock,
    ObjectId: UUID,
    Time: Time,
    Color: Color,
    Vector2: Vector2,
    Vector3: Vector3,
    Vector4: Vector4,
    Angle: Angle,
    Quaternion: Quaternion,
    Matrix: Matrix,
    ULong: u64,
    UByte: u8,
}
