use crate::attributes::{Attribute, DMAttribute};
use indexmap::IndexMap;
use uuid::Uuid as UUID;

/// A trait to allow structs to be converted into a DmElement.
pub trait Element {
    /// Return an element representation of the struct.
    fn to_element(self) -> DmElement;

    /// Return a representation of a struct from an element.
    fn from_element(value: &DmElement) -> Option<Self>
    where
        Self: Sized;
}

/// The main struct for storing data in a DMX file.
#[derive(Clone, Debug)]
pub struct DmElement {
    class: String,
    name: String,
    id: UUID,
    elements: IndexMap<UUID, DmElement>,
    attributes: IndexMap<String, DMAttribute>,
}

impl Element for DmElement {
    fn to_element(self) -> DmElement {
        self
    }

    fn from_element(value: &DmElement) -> Option<Self> {
        // TODO: Can this be done without cloning?
        Some(value.clone())
    }
}

impl DmElement {
    /// Create a new DmElement with the given class and name.
    pub fn new<S: Into<String>>(class: S, name: S) -> Self {
        Self {
            class: class.into(),
            name: name.into(),
            id: UUID::new_v4(),
            elements: IndexMap::new(),
            attributes: IndexMap::new(),
        }
    }

    /// Create an empty DmElement.
    pub fn empty() -> Self {
        Self {
            class: "DmElement".to_string(),
            name: "unnamed".to_string(),
            id: UUID::new_v4(),
            elements: IndexMap::new(),
            attributes: IndexMap::new(),
        }
    }

    /// Get the element with the given name and type.
    pub fn get_element<T: Element, S: AsRef<str>>(&self, name: S) -> Option<T> {
        self.attributes
            .get(name.as_ref())
            .and_then(|attribute| match attribute {
                DMAttribute::Element(id) => Some(id),
                _ => None,
            })
            .and_then(|id| self.elements.get(id))
            .and_then(|element| T::from_element(element))
    }

    /// Get the element arry with the given name and type.
    pub fn get_element_array<T: Element, S: AsRef<str>>(&self, name: S) -> Option<Vec<T>> {
        self.attributes
            .get(name.as_ref())
            .and_then(|attribute| match attribute {
                DMAttribute::ElementArray(ids) => Some(ids),
                _ => None,
            })
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.elements.get(id))
                    .filter_map(|element| T::from_element(element))
                    .collect()
            })
    }

    /// Return a reference to all elements stored.
    pub fn get_elements(&self) -> &IndexMap<UUID, DmElement> {
        &self.elements
    }

    /// Set an element attribute with the given name and element.
    pub fn set_element<T: Element, S: Into<String>>(&mut self, name: S, value: T) {
        let element = value.to_element();
        self.attributes.insert(name.into(), DMAttribute::Element(element.id));
        self.elements.insert(element.id, element);
    }

    /// Set an element array attribute with the given name and elements.
    pub fn set_element_array<T: Element, S: Into<String>>(&mut self, name: S, value: Vec<T>) {
        let elements = value.into_iter().map(|element| {
            let element = element.to_element();
            let element_id = element.id;
            self.elements.insert(element_id, element);
            element_id
        });

        self.attributes.insert(name.into(), DMAttribute::ElementArray(elements.collect()));
    }

    /// Set an element attribute with the given name without any element data.
    pub fn set_null_element<S: Into<String>>(&mut self, name: S) {
        self.attributes.insert(name.into(), DMAttribute::Element(UUID::nil()));
    }

    #[doc(hidden)]
    pub fn set_element_by_id(&mut self, name: String, id: UUID) {
        self.attributes.insert(name, DMAttribute::Element(id));
    }

    #[doc(hidden)]
    pub fn set_element_array_by_id(&mut self, name: String, value: Vec<UUID>) {
        self.attributes.insert(name, DMAttribute::ElementArray(value));
    }

    #[doc(hidden)]
    pub fn add_element<T: Element>(&mut self, value: T) {
        let element = value.to_element();
        self.elements.insert(element.id, element);
    }

    /// Check if the element conctains an element with the given id.
    pub fn has_element(&self, id: UUID) -> bool {
        self.elements.contains_key(&id)
    }

    /// Check if the element conctains an attribute element with the given name.
    pub fn has_element_attribute(&self, id: UUID) -> bool {
        self.attributes.values().any(|attr| match attr {
            DMAttribute::Element(value) => value == &id,
            DMAttribute::ElementArray(values) => values.contains(&id),
            _ => false,
        })
    }

    /// Remove an element attribute with the given name.
    pub fn remove_element<S: AsRef<str>>(&mut self, name: S) {
        self.remove_attribute(name)
    }

    /// Return a reference to an attribute with the given name.
    pub fn get_attribute<T: Attribute>(&self, name: &str) -> Option<&T> {
        self.attributes.get(name).and_then(|attr| T::from_attribute(attr))
    }

    /// Return a reference to all attributes in the element.
    pub fn get_attributes(&self) -> &IndexMap<String, DMAttribute> {
        &self.attributes
    }

    /// Set an attribute with the given name and value.
    pub fn set_attribute<T: Attribute, S: Into<String>>(&mut self, name: S, value: T) {
        self.attributes.insert(name.into(), value.to_attribute());
    }

    /// Remove an attribute with the given name.
    pub fn remove_attribute<S: AsRef<str>>(&mut self, name: S) {
        let removed = self.attributes.shift_remove(name.as_ref());
        if let Some(DMAttribute::Element(id)) = removed {
            self.elements.shift_remove(&id);
        }
    }

    pub fn get_class(&self) -> &str {
        &self.class
    }

    pub fn set_class<S: Into<String>>(&mut self, class: S) {
        self.class = class.into();
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        self.name = name.into();
    }

    pub fn get_id(&self) -> &UUID {
        &self.id
    }

    pub fn set_id(&mut self, id: UUID) {
        self.id = id;
    }
}
