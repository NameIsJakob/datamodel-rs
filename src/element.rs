use std::{cell::RefCell, collections::HashMap};

use uuid::Uuid;

use crate::attribute::Attribute;

#[derive(Clone, Debug)]
pub struct DmElement {
    element_name: String,
    id: Uuid,
    name: RefCell<String>,
    attribute: RefCell<HashMap<String, Attribute>>,
}

impl DmElement {
    pub fn new(element_name: String, name: String, id: Option<Uuid>) -> Self {
        Self {
            element_name,
            id: id.unwrap_or(Uuid::new_v4()),
            name: RefCell::new(name),
            attribute: RefCell::new(HashMap::new()),
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

    pub fn set_name(&mut self, name: String) {
        *self.name.borrow_mut() = name
    }

    pub fn set_id(&mut self, id: Uuid) {
        self.id = id
    }

    pub fn add_attribute(&self, name: String, attribute: Attribute) {
        self.attribute.borrow_mut().insert(name, attribute);
    }
}
