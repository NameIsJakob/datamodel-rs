use std::{
    cell::{Ref, RefCell},
    hash::{Hash, Hasher},
    rc::Rc,
};

use indexmap::IndexMap;

use crate::Attribute;

/// The element struct represents a single element in the data model.
///
/// It contains a name, a class, and a set of attributes.
///
/// A element can have multiple references to multiple attributes.
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
            attributes: Rc::new(RefCell::new(IndexMap::new())),
        }
    }
}

impl Eq for Element {}

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

impl Element {
    /// Creates a new element with a default class with the given name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Rc::new(RefCell::new(name.into())),
            class: Rc::new(RefCell::new(String::from("DmElement"))),
            attributes: Default::default(),
        }
    }

    /// Creates a new element with the given name and class.
    pub fn create(name: impl Into<String>, class: impl Into<String>) -> Self {
        Self {
            name: Rc::new(RefCell::new(name.into())),
            class: Rc::new(RefCell::new(class.into())),
            attributes: Default::default(),
        }
    }

    /// Creates a new nameless element with the given class.
    pub fn class(class: impl Into<String>) -> Self {
        Self {
            name: Rc::new(RefCell::new(String::from("unnamed"))),
            class: Rc::new(RefCell::new(class.into())),
            attributes: Default::default(),
        }
    }

    /// Returns the name of the element.
    pub fn get_name(&self) -> Ref<'_, String> {
        self.name.borrow()
    }

    /// Sets the name of the element.
    pub fn set_name(&mut self, name: impl Into<String>) {
        *self.name.borrow_mut() = name.into();
    }

    /// Returns the class of the element.
    pub fn get_class(&self) -> Ref<'_, String> {
        self.class.borrow()
    }

    /// Sets the class of the element.
    pub fn set_class(&mut self, class: impl Into<String>) {
        *self.class.borrow_mut() = class.into();
    }

    /// Returns the attribute with the given name. If the attribute does not exist, returns None.
    pub fn get_attribute(&self, name: impl AsRef<str>) -> Option<Ref<'_, Attribute>> {
        let attribute_name = name.as_ref();

        match attribute_name {
            "name" => None,
            "id" => None,
            _ => Ref::filter_map(self.attributes.borrow(), |attributes| attributes.get(attribute_name)).ok(),
        }
    }

    /// Returns the value of the attribute with the given name. If the attribute does not exist or is not the same type, returns None.
    pub fn get_value<V>(&self, name: impl AsRef<str>) -> Option<Ref<'_, V>>
    where
        for<'a> &'a V: TryFrom<&'a Attribute>,
    {
        let attribute_name = name.as_ref();

        match attribute_name {
            "name" => None,
            "id" => None,
            _ => Ref::filter_map(self.attributes.borrow(), |attributes| attributes.get(attribute_name)?.try_into().ok()).ok(),
        }
    }

    /// Sets the attribute with the given name.
    pub fn set_attribute(&mut self, name: impl Into<String>, attribute: Attribute) {
        let attribute_name = name.into();

        match attribute_name.as_str() {
            "name" => return,
            "id" => return,
            _ => {}
        }

        self.attributes.borrow_mut().insert(attribute_name, attribute);
    }

    /// Sets the value of the attribute with the given name.
    pub fn set_value(&mut self, name: impl Into<String>, value: impl Into<Attribute>) {
        self.set_attribute(name, value.into());
    }

    /// Removes the attribute with the given name and returns it. If the attribute does not exist, returns None.
    pub fn remove_attribute(&mut self, name: impl AsRef<str>) -> Option<Attribute> {
        let attribute_name = name.as_ref();

        match attribute_name {
            "name" => None,
            "id" => None,
            _ => self.attributes.borrow_mut().shift_remove(attribute_name),
        }
    }

    /// Removes the value of the attribute with the given name and returns it. If the attribute does not exist or is not the same type, returns None.
    pub fn remove_value<V: TryFrom<Attribute>>(&mut self, name: impl AsRef<str>) -> Option<V> {
        let attribute = self.remove_attribute(name)?;

        V::try_from(attribute).ok()
    }

    /// Returns the attributes of the element.
    pub fn get_attributes(&self) -> Ref<'_, IndexMap<String, Attribute>> {
        self.attributes.borrow()
    }

    /// Reserves capacity for at least additional more elements to be inserted in the given attributes.
    pub fn reserve_attributes(&mut self, additional: usize) {
        self.attributes.borrow_mut().reserve(additional);
    }
}
