use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use indexmap::IndexMap;
use uuid::Uuid as UUID;

use crate::Attribute;

/// The element struct represents a single element in the data model.
///
/// It contains a name, a class, and a set of attributes.
///
/// A element can have multiple references to multiple attributes.
#[derive(Clone, Debug)]
pub struct Element(Rc<RefCell<ElementData>>);

impl Default for Element {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(ElementData {
            name: String::from(Self::DEFAULT_ELEMENT_NAME),
            class: String::from(Self::DEFAULT_ELEMENT_CLASS),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
        })))
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.0.borrow().id == other.0.borrow().id
    }
}

impl Eq for Element {}

impl std::hash::Hash for Element {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().id.hash(state);
    }
}

impl Element {
    pub const DEFAULT_ELEMENT_NAME: &str = "unnamed";
    pub const DEFAULT_ELEMENT_CLASS: &str = "DmElement";

    /// Creates a new element with the given name and class.
    pub fn create(name: impl Into<String>, class: impl Into<String>) -> Self {
        Self(Rc::new(RefCell::new(ElementData {
            name: name.into(),
            class: class.into(),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
        })))
    }

    /// Creates a new element with a default class with the given name.
    pub fn named(name: impl Into<String>) -> Self {
        Self(Rc::new(RefCell::new(ElementData {
            name: name.into(),
            class: String::from(Self::DEFAULT_ELEMENT_CLASS),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
        })))
    }

    /// Creates a new nameless element with the given class.
    pub fn class(class: impl Into<String>) -> Self {
        Self(Rc::new(RefCell::new(ElementData {
            name: String::from(Self::DEFAULT_ELEMENT_NAME),
            class: class.into(),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
        })))
    }

    /// Create a element with the name, class, and id specified.
    pub fn full(name: impl Into<String>, class: impl Into<String>, id: UUID) -> Self {
        Self(Rc::new(RefCell::new(ElementData {
            name: name.into(),
            class: class.into(),
            id,
            attributes: IndexMap::new(),
        })))
    }

    /// Returns the name of the element.
    pub fn get_name(&self) -> Ref<String> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element| &element.name)
    }

    /// Sets the name of the element.
    pub fn set_name(&self, name: impl Into<String>) {
        let mut element_data = self.0.borrow_mut();
        element_data.name = name.into();
    }

    /// Returns the class of the element.
    pub fn get_class(&self) -> Ref<String> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element| &element.class)
    }

    /// Sets the class of the element.
    pub fn set_class(&self, class: impl Into<String>) {
        let mut element_data = self.0.borrow_mut();
        element_data.class = class.into();
    }

    /// Returns the [UUID] of the element.
    pub fn get_id(&self) -> Ref<UUID> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element: &ElementData| &element.id)
    }

    /// Sets the id of the element.
    pub fn set_id(&self, id: UUID) {
        let mut element_data = self.0.borrow_mut();
        element_data.id = id;
    }

    /// Returns the attribute with the given name. If the attribute does not exist, returns None.
    pub fn get_attribute(&self, name: impl AsRef<str>) -> Option<Ref<Attribute>> {
        let element_data = self.0.borrow();
        let attribute_name = name.as_ref();
        Ref::filter_map(element_data, |element| element.attributes.get(attribute_name)).ok()
    }

    /// Sets the attribute with the given name.
    pub fn set_attribute(&mut self, name: impl Into<String>, attribute: Attribute) -> Option<Attribute> {
        let mut element_data = self.0.borrow_mut();
        let attribute_name = name.into();

        if attribute_name.eq("name") || attribute_name.eq("id") {
            return None;
        }

        element_data.attributes.insert(attribute_name, attribute)
    }

    /// Removes the attribute with the given name and returns it. If the attribute does not exist, returns None.
    pub fn remove_attribute(&mut self, name: impl AsRef<str>) -> Option<Attribute> {
        let mut element_data = self.0.borrow_mut();
        let attribute_name = name.as_ref();
        element_data.attributes.shift_remove(attribute_name)
    }

    /// Returns the value of the attribute with the given name. If the attribute does not exist or is not the same type, returns None.
    pub fn get_value<V>(&self, name: impl AsRef<str>) -> Option<Ref<V>>
    where
        for<'a> &'a V: TryFrom<&'a Attribute>,
    {
        let element_data = self.0.borrow();
        let attribute_name = name.as_ref();
        let element_attribute = Ref::filter_map(element_data, |element| element.attributes.get(attribute_name)).ok()?;
        Ref::filter_map(element_attribute, |attribute| attribute.try_into().ok()).ok()
    }

    /// Sets the value of the attribute with the given name. If there was a value with the same type then its returned.
    pub fn set_value<V>(&mut self, name: impl Into<String>, value: V) -> Option<V>
    where
        V: Into<Attribute> + TryFrom<Attribute>,
    {
        let mut element_data = self.0.borrow_mut();
        let attribute_name = name.into();
        let attribute_value = value.into();

        if attribute_name.eq("name") || attribute_name.eq("id") {
            return None;
        }

        element_data
            .attributes
            .insert(attribute_name, attribute_value)
            .and_then(|attribute| attribute.try_into().ok())
    }

    /// Removes the value of the attribute with the given name and returns it. If the attribute does not exist or is not the same type, returns None.
    pub fn remove_value<V: TryFrom<Attribute>>(&mut self, name: impl AsRef<str>) -> Option<V> {
        let attribute = self.remove_attribute(name)?;
        V::try_from(attribute).ok()
    }

    /// Returns the attributes of the element.
    pub fn get_attributes(&self) -> Ref<IndexMap<String, Attribute>> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element| &element.attributes)
    }

    /// Reserves capacity for at least additional more elements to be inserted in the given attributes.
    pub fn reserve_attributes(&mut self, additional: usize) {
        let mut element_data = self.0.borrow_mut();
        element_data.attributes.reserve(additional);
    }
}

#[derive(Debug)]
struct ElementData {
    name: String,
    class: String,
    id: UUID,
    attributes: IndexMap<String, Attribute>,
}
