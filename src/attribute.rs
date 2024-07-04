use std::time::Duration;

use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

use crate::{
    attributes::{Angle, Binary, Color, Matrix, Quaternion, Vector2, Vector3, Vector4},
    Element,
};

/// Errors the can be returned when working with attributes.
#[derive(Debug, ThisError)]
pub enum AttributeError {
    /// The attribute name provided was either "name" or "id". They can't be used as they are reserved for keyvalues 2.
    /// ```
    /// use datamodel::{Element, AttributeError};
    ///
    /// let mut root = Element::default();
    /// assert!(matches!(root.set_attribute("name", 0.0), Err(AttributeError::ReservedAttributeName)));
    /// assert!(matches!(root.set_attribute("id", 0.0), Err(AttributeError::ReservedAttributeName)));
    /// ```
    #[error("Attribute Name Is A Reserved Name")]
    ReservedAttributeName,
    /// The attribute name provided was empty.
    /// ```
    /// use datamodel::{Element, AttributeError};
    ///
    /// let mut root = Element::default();
    /// assert!(matches!(root.set_attribute("", 0.0), Err(AttributeError::EmptyAttributeName)));
    /// ```
    #[error("Attribute Name Can't Be Empty")]
    EmptyAttributeName,
    /// The attribute didn't exist in the element.
    /// ```
    /// use datamodel::{Element, AttributeError};
    ///
    /// let mut root = Element::default();
    /// assert!(matches!(root.get_attribute::<f32>("test"), Err(AttributeError::MissingAttribute)));
    /// ```
    #[error("Attribute Does Not Exist")]
    MissingAttribute,
    /// The attribute type was not the type provided.
    /// ```
    /// use datamodel::{Element, AttributeError};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", 0i32).unwrap();
    /// assert!(matches!(root.get_attribute::<f32>("test"), Err(AttributeError::WrongExpectedAttributeType)));
    /// ```
    #[error("Attribute Type Was Different Than Given Type")]
    WrongExpectedAttributeType,
    /// The attribute element was null when expecting it be not.
    /// ```
    /// use datamodel::{Element, AttributeError};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", None).unwrap();
    /// assert!(matches!(root.get_attribute::<Element>("test"), Err(AttributeError::AttributeElementWasNull)));
    /// ```
    #[error("Expected Element Type Was Null")]
    AttributeElementWasNull,
}

/// The types that datamodel supports.
#[derive(Clone, Debug)]
pub enum Attribute {
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test None", None).unwrap();
    /// assert!(root.get_attribute::<Option<Element>>("test None").is_ok());
    ///
    /// let mut root = Element::default();
    /// let test = Element::default();
    /// root.set_attribute("test Some", test).unwrap();
    /// assert!(root.get_attribute::<Option<Element>>("test Some").is_ok());
    /// ```
    Element(Option<Element>),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", 0i32).unwrap();
    /// assert!(root.get_attribute::<i32>("test").is_ok());
    /// ```
    Integer(i32),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", 0f32).unwrap();
    /// assert!(root.get_attribute::<f32>("test").is_ok());
    /// ```
    Float(f32),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test True", true).unwrap();
    /// assert!(root.get_attribute::<bool>("test True").is_ok());
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test False", false).unwrap();
    /// assert!(root.get_attribute::<bool>("test False").is_ok());
    /// ```
    Boolean(bool),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", String::from("test")).unwrap();
    /// assert!(root.get_attribute::<String>("test").is_ok());
    /// ```
    String(String),
    /// ```
    /// use datamodel::{Element, attributes::Binary};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![0u8; 3]).unwrap();
    /// assert!(root.get_attribute::<Binary>("test").is_ok());
    /// ```
    Binary(Binary),
    /// ```
    /// use datamodel::Element;
    /// use uuid::Uuid as UUID;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", UUID::nil()).unwrap();
    /// assert!(root.get_attribute::<UUID>("test").is_ok());
    /// ```
    ObjectId(UUID),
    /// ```
    /// use datamodel::Element;
    /// use std::time::Duration;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Duration::from_secs(5)).unwrap();
    /// assert!(root.get_attribute::<Duration>("test").is_ok());
    /// ```
    Time(Duration),
    /// ```
    /// use datamodel::{Element, attributes::Color};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Color::default()).unwrap();
    /// assert!(root.get_attribute::<Color>("test").is_ok());
    /// ```
    Color(Color),
    /// ```
    /// use datamodel::{Element, attributes::Vector2};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Vector2::default()).unwrap();
    /// assert!(root.get_attribute::<Vector2>("test").is_ok());
    /// ```
    Vector2(Vector2),
    /// ```
    /// use datamodel::{Element, attributes::Vector3};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Vector3::default()).unwrap();
    /// assert!(root.get_attribute::<Vector3>("test").is_ok());
    /// ```
    Vector3(Vector3),
    /// ```
    /// use datamodel::{Element, attributes::Vector4};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Vector4::default()).unwrap();
    /// assert!(root.get_attribute::<Vector4>("test").is_ok());
    /// ```
    Vector4(Vector4),
    /// ```
    /// use datamodel::{Element, attributes::Angle};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Angle::default()).unwrap();
    /// assert!(root.get_attribute::<Angle>("test").is_ok());
    /// ```
    Angle(Angle),
    /// ```
    /// use datamodel::{Element, attributes::Quaternion};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Quaternion::default()).unwrap();
    /// assert!(root.get_attribute::<Quaternion>("test").is_ok());
    /// ```
    Quaternion(Quaternion),
    /// ```
    /// use datamodel::{Element, attributes::Matrix};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", Matrix::default()).unwrap();
    /// assert!(root.get_attribute::<Matrix>("test").is_ok());
    /// ```
    Matrix(Matrix),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test None", vec![None; 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Option<Element>>>("test None").is_ok());
    ///
    /// let mut root = Element::default();
    /// let test1 = Element::default();
    /// let test2 = Element::default();
    /// let test3 = Element::default();
    /// root.set_attribute("test Some", vec![test1, test2, test3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Option<Element>>>("test Some").is_ok());
    /// ```
    ElementArray(Vec<Option<Element>>),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![0i32; 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<i32>>("test").is_ok());
    /// ```
    IntegerArray(Vec<i32>),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![0f32; 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<f32>>("test").is_ok());
    /// ```
    FloatArray(Vec<f32>),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test True", vec![true; 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<bool>>("test True").is_ok());
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test False", vec![false; 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<bool>>("test False").is_ok());
    /// ```
    BooleanArray(Vec<bool>),
    /// ```
    /// use datamodel::Element;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![String::from("test"); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<String>>("test").is_ok());
    /// ```
    StringArray(Vec<String>),
    /// ```
    /// use datamodel::{Element, attributes::Binary};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![vec![0u8; 3]; 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Binary>>("test").is_ok());
    /// ```
    BinaryArray(Vec<Binary>),
    /// ```
    /// use datamodel::Element;
    /// use uuid::Uuid as UUID;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![UUID::nil(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<UUID>>("test").is_ok());
    /// ```
    ObjectIdArray(Vec<UUID>),
    /// ```
    /// use datamodel::Element;
    /// use std::time::Duration;
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Duration::from_secs(5); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Duration>>("test").is_ok());
    /// ```
    TimeArray(Vec<Duration>),
    /// ```
    /// use datamodel::{Element, attributes::Color};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Color::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Color>>("test").is_ok());
    /// ```
    ColorArray(Vec<Color>),
    /// ```
    /// use datamodel::{Element, attributes::Vector2};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Vector2::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Vector2>>("test").is_ok());
    /// ```
    Vector2Array(Vec<Vector2>),
    /// ```
    /// use datamodel::{Element, attributes::Vector3};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Vector3::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Vector3>>("test").is_ok());
    /// ```
    Vector3Array(Vec<Vector3>),
    /// ```
    /// use datamodel::{Element, attributes::Vector4};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Vector4::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Vector4>>("test").is_ok());
    /// ```
    Vector4Array(Vec<Vector4>),
    /// ```
    /// use datamodel::{Element, attributes::Angle};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Angle::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Angle>>("test").is_ok());
    /// ```
    AngleArray(Vec<Angle>),
    /// ```
    /// use datamodel::{Element, attributes::Quaternion};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Quaternion::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Quaternion>>("test").is_ok());
    /// ```
    QuaternionArray(Vec<Quaternion>),
    /// ```
    /// use datamodel::{Element, attributes::Matrix};
    ///
    /// let mut root = Element::default();
    /// root.set_attribute("test", vec![Matrix::default(); 3]).unwrap();
    /// assert!(root.get_attribute::<Vec<Matrix>>("test").is_ok());
    /// ```
    MatrixArray(Vec<Matrix>),
}

macro_rules! declare_attribute {
    ($qualifier:ty, $attribute:path, $array:path) => {
        impl TryFrom<Attribute> for $qualifier {
            type Error = AttributeError;

            fn try_from(value: Attribute) -> Result<Self, Self::Error> {
                match value {
                    $attribute(attribute) => Ok(attribute),
                    _ => Err(AttributeError::WrongExpectedAttributeType),
                }
            }
        }

        impl From<$qualifier> for Attribute {
            fn from(value: $qualifier) -> Self {
                $attribute(value)
            }
        }

        impl TryFrom<Attribute> for Vec<$qualifier> {
            type Error = AttributeError;

            fn try_from(value: Attribute) -> Result<Self, Self::Error> {
                match value {
                    $array(attribute) => Ok(attribute),
                    _ => Err(AttributeError::WrongExpectedAttributeType),
                }
            }
        }

        impl From<Vec<$qualifier>> for Attribute {
            fn from(value: Vec<$qualifier>) -> Self {
                $array(value)
            }
        }
    };
}

impl TryFrom<Attribute> for Element {
    type Error = AttributeError;

    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        match value {
            Attribute::Element(attribute) => attribute.ok_or(AttributeError::AttributeElementWasNull),
            _ => Err(AttributeError::WrongExpectedAttributeType),
        }
    }
}

impl From<Element> for Attribute {
    fn from(value: Element) -> Self {
        Self::Element(Some(value))
    }
}

impl TryFrom<Attribute> for Vec<Element> {
    type Error = AttributeError;

    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        match value {
            Attribute::ElementArray(attribute) => attribute
                .into_iter()
                .map(|element| element.ok_or(AttributeError::AttributeElementWasNull))
                .collect(),
            _ => Err(AttributeError::WrongExpectedAttributeType),
        }
    }
}

impl From<Vec<Element>> for Attribute {
    fn from(value: Vec<Element>) -> Self {
        Self::ElementArray(value.into_iter().map(Some).collect())
    }
}

declare_attribute!(Option<Element>, Attribute::Element, Attribute::ElementArray);
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
