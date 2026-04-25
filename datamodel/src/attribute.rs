use super::element::{Element, ElementClass};
use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};
pub use uuid::Uuid as UUID;

macro_rules! attribute_list {
    ($($name:ident : $value:ty),* $(,)?) => {
        paste::paste! {
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum AttributeType {
                $($name,)*
                $([<$name Array>],)*
            }

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

            $(
                impl AttributeInfo for $value {
                    fn attribute_type() -> AttributeType {
                        AttributeType::$name
                    }

                    fn into_attribute_type(self) -> AttributeValue {
                        AttributeValue::$name(self)
                    }

                    fn get_inner(attribute: &AttributeValue) -> Option<&Self> {
                        match attribute {
                            AttributeValue::$name(inner_value) => Some(inner_value),
                            _ => None
                        }
                    }

                    fn get_inner_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
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

                    fn get_inner(attribute: &AttributeValue) -> Option<&Self> {
                        match attribute {
                            AttributeValue::[<$name Array>](inner_value) => Some(inner_value),
                            _ => None
                        }
                    }

                    fn get_inner_mut(attribute: &mut AttributeValue) -> Option<&mut Self> {
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

#[derive(Debug, Clone, Default)]
pub struct BinaryBlock(pub Vec<u8>);

#[derive(Debug, Clone, Copy, Default)]
pub struct Time(pub i32);

impl Time {
    pub fn as_seconds(&self) -> f32 {
        self.0 as f32 / 10000.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

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
            pitch: value.b,
            yaw: value.c,
            roll: value.a,
        }
    }
}

#[cfg(feature = "mint")]
impl From<Angle> for mint::EulerAngles<f32, mint::IntraXYZ> {
    fn from(value: Angle) -> Self {
        Self {
            a: value.roll,
            b: value.pitch,
            c: value.yaw,
            marker: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "mint")]
impl mint::IntoMint for Angle {
    type MintType = mint::EulerAngles<f32, mint::IntraXYZ>;
}

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

#[derive(Debug, Clone, Copy, Default)]
pub struct Matrix(pub [[f32; 4]; 4]);

attribute_list! {
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
}

#[derive(Clone, Debug)]
pub struct Attribute(Rc<RefCell<AttributeValue>>);

impl Attribute {
    pub fn new(value: AttributeValue) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn get_type(&self) -> AttributeType {
        self.0.borrow().attribute_type()
    }

    pub fn get_inner(&self) -> Ref<'_, AttributeValue> {
        self.0.borrow()
    }
}

pub trait AttributeInfo: Default {
    fn attribute_type() -> AttributeType;
    fn into_attribute_type(self) -> AttributeValue;
    fn into_attribute(self) -> Attribute {
        Attribute::new(self.into_attribute_type())
    }
    fn get_inner(attribute: &AttributeValue) -> Option<&Self>;
    fn get_inner_mut(attribute: &mut AttributeValue) -> Option<&mut Self>;
}

#[derive(Clone)]
pub struct AttributeVariable<A: AttributeInfo> {
    owner: Element,
    attribute: Attribute,
    phantom: PhantomData<A>,
}

impl<A: AttributeInfo> AttributeVariable<A> {
    pub fn initialize(owner: Element, attribute_name: &'static str) -> Self {
        Self::initialize_with(owner, attribute_name, A::default())
    }

    pub fn initialize_with(mut owner: Element, attribute_name: &'static str, value: A) -> Self {
        let attribute = if let Some(owned_attribute) = owner.get_attribute(attribute_name)
            && owned_attribute.get_type() == A::attribute_type()
        {
            owned_attribute
        } else {
            value.into_attribute()
        };
        owner.set_attribute(attribute_name, Attribute::clone(&attribute));
        Self {
            owner,
            attribute,
            phantom: PhantomData,
        }
    }

    pub fn get(&self) -> Ref<'_, A> {
        Ref::map(self.attribute.0.borrow(), |inner| A::get_inner(inner).unwrap())
    }

    pub fn get_mut(&self) -> RefMut<'_, A> {
        RefMut::map(self.attribute.0.borrow_mut(), |inner| A::get_inner_mut(inner).unwrap())
    }

    pub fn set(&mut self, value: A) {
        *A::get_inner_mut(&mut self.attribute.0.borrow_mut()).unwrap() = value;
    }

    pub fn owner(&self) -> Element {
        Element::clone(&self.owner)
    }

    pub fn attribute(&self) -> Attribute {
        Attribute::clone(&self.attribute)
    }
}

#[derive(Clone)]
pub struct AttributeElement<E: ElementClass> {
    owner: Element,
    attribute: Attribute,
    phantom: PhantomData<E>,
}

impl<E: ElementClass> AttributeElement<E> {
    pub fn initialize(owner: Element, attribute_name: &'static str) -> Self {
        Self::initialize_with(owner, attribute_name, None)
    }

    pub fn initialize_with(mut owner: Element, attribute_name: &'static str, value: Option<E>) -> Self {
        let attribute = if let Some(owned_attribute) = owner.get_attribute(attribute_name)
            && owned_attribute.get_type() == AttributeType::Element
        {
            owned_attribute
        } else {
            value.map(|e| e.into_element()).into_attribute()
        };
        owner.set_attribute(attribute_name, Attribute::clone(&attribute));
        Self {
            owner,
            attribute,
            phantom: PhantomData,
        }
    }

    pub fn get(&self) -> Option<E> {
        Ref::map(self.attribute.0.borrow(), |inner| Option::<Element>::get_inner(inner).unwrap())
            .as_ref()
            .map(|e| E::from_element(Element::clone(e)))
    }

    pub fn get_as<C: ElementClass>(&self) -> Option<C> {
        Ref::map(self.attribute.0.borrow(), |inner| Option::<Element>::get_inner(inner).unwrap())
            .as_ref()
            .map(|e| C::from_element(Element::clone(e)))
    }

    pub fn set<C: ElementClass>(&mut self, value: Option<C>) {
        *Option::<Element>::get_inner_mut(&mut self.attribute.0.borrow_mut()).unwrap() = value.map(|e| e.into_element());
    }

    pub fn owner(&self) -> Element {
        Element::clone(&self.owner)
    }

    pub fn attribute(&self) -> Attribute {
        Attribute::clone(&self.attribute)
    }
}

#[derive(Clone)]
pub struct AttributeElementArray<E: ElementClass> {
    owner: Element,
    attribute: Attribute,
    phantom: PhantomData<E>,
}

impl<E: ElementClass> AttributeElementArray<E> {
    pub fn initialize(owner: Element, attribute_name: &'static str) -> Self {
        Self::initialize_with(owner, attribute_name, Vec::new())
    }

    pub fn initialize_with(mut owner: Element, attribute_name: &'static str, value: Vec<Option<E>>) -> Self {
        let attribute = if let Some(owned_attribute) = owner.get_attribute(attribute_name)
            && owned_attribute.get_type() == AttributeType::ElementArray
        {
            owned_attribute
        } else {
            value
                .into_iter()
                .map(|a| a.map(|e| e.into_element()))
                .collect::<Vec<Option<Element>>>()
                .into_attribute()
        };
        owner.set_attribute(attribute_name, Attribute::clone(&attribute));
        Self {
            owner,
            attribute,
            phantom: PhantomData,
        }
    }

    pub fn get<C: ElementClass>(&self) -> Vec<Option<C>> {
        Ref::map(self.attribute.0.borrow(), |inner| Vec::<Option<Element>>::get_inner(inner).unwrap())
            .iter()
            .map(|a| a.as_ref().map(|e| C::from_element(Element::clone(e))))
            .collect()
    }

    pub fn set<C: ElementClass>(&self, value: Vec<Option<C>>) {
        *Vec::<Option<Element>>::get_inner_mut(&mut self.attribute.0.borrow_mut()).unwrap() = value.into_iter().map(|a| a.map(|e| e.into_element())).collect()
    }

    pub fn get_index<C: ElementClass>(&self, index: usize) -> Option<C> {
        Ref::map(self.attribute.0.borrow(), |inner| Vec::<Option<Element>>::get_inner(inner).unwrap())
            .get(index)
            .and_then(|a| a.as_ref().map(|e| C::from_element(Element::clone(e))))
    }

    pub fn push<C: ElementClass>(&mut self, value: Option<C>) {
        Vec::<Option<Element>>::get_inner_mut(&mut self.attribute.0.borrow_mut())
            .unwrap()
            .push(value.map(|e| e.into_element()));
    }

    pub fn insert<C: ElementClass>(&mut self, index: usize, value: Option<C>) {
        Vec::<Option<Element>>::get_inner_mut(&mut self.attribute.0.borrow_mut())
            .unwrap()
            .insert(index, value.map(|e| e.into_element()));
    }

    pub fn owner(&self) -> Element {
        Element::clone(&self.owner)
    }

    pub fn attribute(&self) -> Attribute {
        Attribute::clone(&self.attribute)
    }
}
