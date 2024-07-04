use std::{
    cell::{Ref, RefCell},
    hash::{Hash, Hasher},
    rc::Rc,
};

use indexmap::IndexMap;

use crate::{attribute::AttributeError, Attribute};

/// The main structure to hold attributes.
#[derive(Clone, Debug)]
pub struct Element {
    name: Rc<RefCell<String>>,
    class: Rc<RefCell<String>>,
    attributes: Rc<RefCell<IndexMap<String, Attribute>>>,
}

impl Default for Element {
    fn default() -> Self {
        Self {
            name: Rc::new(RefCell::new(String::from("unnamed"))),
            class: Rc::new(RefCell::new(String::from("DmElement"))),
            attributes: Default::default(),
        }
    }
}

impl Hash for Element {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.name).hash(state);
        Rc::as_ptr(&self.class).hash(state);
        Rc::as_ptr(&self.attributes).hash(state);
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.name, &other.name) && Rc::ptr_eq(&self.class, &other.class) && Rc::ptr_eq(&self.attributes, &other.attributes)
    }
}

impl Eq for Element {}

impl Element {
    /// Creates a new element with a name and class specified.
    pub fn create<N: Into<String>, C: Into<String>>(name: N, class: C) -> Self {
        Self {
            name: Rc::new(RefCell::new(name.into())),
            class: Rc::new(RefCell::new(class.into())),
            attributes: Default::default(),
        }
    }

    /// Creates a new element with a name with the default class "DmElement".
    pub fn new<N: Into<String>>(name: N) -> Self {
        Self {
            name: Rc::new(RefCell::new(name.into())),
            class: Rc::new(RefCell::new(String::from("DmElement"))),
            attributes: Default::default(),
        }
    }

    /// Creates a nameless new element with a class.
    pub fn class<C: Into<String>>(class: C) -> Self {
        Self {
            name: Rc::new(RefCell::new(String::from("unnamed"))),
            class: Rc::new(RefCell::new(class.into())),
            attributes: Default::default(),
        }
    }

    /// Returns a reference to the element name.
    pub fn get_name(&self) -> Ref<String> {
        self.name.borrow()
    }

    /// Sets the name of the element.
    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        let element_name = name.into();
        *self.name.borrow_mut() = element_name;
    }

    /// Returns a reference to the class.
    pub fn get_class(&self) -> Ref<String> {
        self.class.borrow()
    }

    /// Sets the class of the element.
    pub fn set_class<S: Into<String>>(&mut self, class: S) {
        let element_class = class.into();
        *self.class.borrow_mut() = element_class;
    }

    /// Add or set an attribute to the element.
    ///
    /// Attributes names can't be empty, be "name", or "id" as they are reserved.
    ///
    /// The elements can't have references to them self.
    pub fn set_attribute<S: Into<String>, A: Into<Attribute>>(&mut self, name: S, value: A) -> Result<(), AttributeError> {
        let attribute_name = name.into();

        if attribute_name.is_empty() {
            return Err(AttributeError::EmptyAttributeName);
        }

        if &attribute_name == "name" || &attribute_name == "id" {
            return Err(AttributeError::ReservedAttributeName);
        }

        let attribute_value = value.into();

        // TODO: Check for recursion!

        self.attributes.borrow_mut().insert(attribute_name, attribute_value);
        Ok(())
    }

    /// Remove an attribute from element.
    ///
    /// returns bool if an attribute was removed.
    pub fn remove_attribute(&mut self, name: &str) -> bool {
        self.attributes.borrow_mut().shift_remove(name).is_some()
    }

    /// Get an attribute value from element.
    pub fn get_attribute<A: TryFrom<Attribute, Error = AttributeError>>(&self, name: &str) -> Result<A, AttributeError> {
        self.attributes.borrow().get(name).ok_or(AttributeError::MissingAttribute)?.clone().try_into()
    }

    /// Check if an attribute is in element.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.borrow().contains_key(name)
    }

    /// Returns a reference to all attributes to element.
    pub fn get_attributes(&self) -> Ref<IndexMap<String, Attribute>> {
        self.attributes.borrow()
    }
}
