use std::{cell::RefCell, rc::Rc};

use indexmap::IndexMap;
use uuid::Uuid;

use crate::attribute::{Attribute, Color, Vector2, Vector3, Vector4};

#[derive(Clone, Debug)]
pub struct DmElement {
    element_name: String,
    id: Uuid,
    name: RefCell<String>,
    attribute: RefCell<IndexMap<String, Attribute>>,
}

impl DmElement {
    pub fn new(element_name: String, name: String, id: Option<Uuid>) -> Self {
        Self {
            element_name,
            id: id.unwrap_or(Uuid::new_v4()),
            name: RefCell::new(name),
            attribute: RefCell::new(IndexMap::new()),
        }
    }

    pub fn get_name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_element_name(&self) -> &str {
        &self.element_name
    }

    pub fn get_attribute(&self, name: &str) -> Option<Attribute> {
        self.attribute.borrow().get(name).cloned()
    }

    pub fn get_attribute_element(&self, name: &str) -> Option<Rc<DmElement>> {
        match self.get_attribute(name) {
            Some(Attribute::Element(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_int(&self, name: &str) -> Option<i32> {
        match self.get_attribute(name) {
            Some(Attribute::Int(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_float(&self, name: &str) -> Option<f32> {
        match self.get_attribute(name) {
            Some(Attribute::Float(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_boolean(&self, name: &str) -> Option<bool> {
        match self.get_attribute(name) {
            Some(Attribute::Bool(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_string(&self, name: &str) -> Option<String> {
        match self.get_attribute(name) {
            Some(Attribute::String(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_bytes(&self, name: &str) -> Option<Vec<u8>> {
        match self.get_attribute(name) {
            Some(Attribute::Void(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_id(&self, name: &str) -> Option<Uuid> {
        match self.get_attribute(name) {
            Some(Attribute::ObjectId(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_color(&self, name: &str) -> Option<Color> {
        match self.get_attribute(name) {
            Some(Attribute::Color(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_vector2(&self, name: &str) -> Option<Vector2> {
        match self.get_attribute(name) {
            Some(Attribute::Vector2(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_vector3(&self, name: &str) -> Option<Vector3> {
        match self.get_attribute(name) {
            Some(Attribute::Vector3(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_vector4(&self, name: &str) -> Option<Vector4> {
        match self.get_attribute(name) {
            Some(Attribute::Vector4(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_qangle(&self, name: &str) -> Option<Vector3> {
        match self.get_attribute(name) {
            Some(Attribute::QAngle(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_quaternion(&self, name: &str) -> Option<Vector4> {
        match self.get_attribute(name) {
            Some(Attribute::Quaternion(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_matrix(&self, name: &str) -> Option<[f32; 16]> {
        match self.get_attribute(name) {
            Some(Attribute::Matrix(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_element_array(&self, name: &str) -> Option<Vec<Rc<DmElement>>> {
        match self.get_attribute(name) {
            Some(Attribute::ElementArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_int_array(&self, name: &str) -> Option<Vec<i32>> {
        match self.get_attribute(name) {
            Some(Attribute::IntArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_float_array(&self, name: &str) -> Option<Vec<f32>> {
        match self.get_attribute(name) {
            Some(Attribute::FloatArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_boolean_array(&self, name: &str) -> Option<Vec<bool>> {
        match self.get_attribute(name) {
            Some(Attribute::BoolArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_string_array(&self, name: &str) -> Option<Vec<String>> {
        match self.get_attribute(name) {
            Some(Attribute::StringArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_bytes_array(&self, name: &str) -> Option<Vec<Vec<u8>>> {
        match self.get_attribute(name) {
            Some(Attribute::VoidArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_id_array(&self, name: &str) -> Option<Vec<Uuid>> {
        match self.get_attribute(name) {
            Some(Attribute::ObjectIdArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_color_array(&self, name: &str) -> Option<Vec<Color>> {
        match self.get_attribute(name) {
            Some(Attribute::ColorArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_vector2_array(&self, name: &str) -> Option<Vec<Vector2>> {
        match self.get_attribute(name) {
            Some(Attribute::Vector2Array(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_vector3_array(&self, name: &str) -> Option<Vec<Vector3>> {
        match self.get_attribute(name) {
            Some(Attribute::Vector3Array(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_vector4_array(&self, name: &str) -> Option<Vec<Vector4>> {
        match self.get_attribute(name) {
            Some(Attribute::Vector4Array(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_qangle_array(&self, name: &str) -> Option<Vec<Vector3>> {
        match self.get_attribute(name) {
            Some(Attribute::QAngleArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_quaternion_array(&self, name: &str) -> Option<Vec<Vector4>> {
        match self.get_attribute(name) {
            Some(Attribute::QuaternionArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_attribute_matrix_array(&self, name: &str) -> Option<Vec<[f32; 16]>> {
        match self.get_attribute(name) {
            Some(Attribute::MatrixArray(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_all_attributes(&self) -> IndexMap<String, Attribute> {
        self.attribute.borrow().clone()
    }

    pub fn set_name(&self, name: String) {
        *self.name.borrow_mut() = name
    }

    pub fn add_attribute(&self, name: String, attribute: Attribute) {
        self.attribute.borrow_mut().insert(name, attribute);
    }
}
