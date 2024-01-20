use crate::attributes::{Attribute, DMAttribute};
use indexmap::IndexMap;
use uuid::Uuid as UUID;

pub trait Element {
    fn get_element(self) -> DmElement;
    fn from_element(value: &DmElement) -> &Self;
}

pub struct DmElement {
    class: String,
    name: String,
    id: UUID,
    elements: IndexMap<UUID, DmElement>,
    attributes: IndexMap<String, DMAttribute>,
}

impl Element for DmElement {
    fn get_element(self) -> DmElement {
        self
    }

    fn from_element(value: &DmElement) -> &Self {
        value
    }
}

impl DmElement {
    pub fn new(class: String, name: String) -> Self {
        Self {
            class,
            name,
            id: UUID::new_v4(),
            elements: IndexMap::new(),
            attributes: IndexMap::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            class: "DmElement".to_string(),
            name: "unnamed".to_string(),
            id: UUID::new_v4(),
            elements: IndexMap::new(),
            attributes: IndexMap::new(),
        }
    }

    pub fn get_element<T: Element>(&self, name: &str) -> Option<&T> {
        self.attributes
            .get(name)
            .and_then(|attr| match attr {
                DMAttribute::Element(id) => self.elements.get(id),
                _ => None,
            })
            .map(|element| T::from_element(element))
    }

    pub fn get_element_array<T: Element>(&self, name: &str) -> Option<Vec<&T>> {
        self.attributes
            .get(name)
            .and_then(|attr| match attr {
                DMAttribute::ElementArray(ids) => Some(ids),
                _ => None,
            })
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.elements.get(id))
                    .map(|element| T::from_element(element))
                    .collect()
            })
    }

    pub fn get_elements(&self) -> &IndexMap<UUID, DmElement> {
        &self.elements
    }

    pub fn set_element<T: Element>(&mut self, name: String, value: T) {
        let element = value.get_element();
        self.attributes.insert(name, DMAttribute::Element(element.id));
        self.elements.insert(element.id, element);
    }

    pub fn set_element_array<T: Element>(&mut self, name: String, value: Vec<T>) {
        let elements = value.into_iter().map(|element| {
            let element = element.get_element();
            let element_id = element.id;
            self.elements.insert(element_id, element);
            element_id
        });

        self.attributes.insert(name, DMAttribute::ElementArray(elements.collect()));
    }

    pub fn set_element_by_id(&mut self, name: String, id: UUID) {
        self.attributes.insert(name, DMAttribute::Element(id));
    }

    pub fn set_element_array_by_id(&mut self, name: String, value: Vec<UUID>) {
        self.attributes.insert(name, DMAttribute::ElementArray(value));
    }

    pub fn add_element<T: Element>(&mut self, value: T) {
        let element = value.get_element();
        self.elements.insert(element.id, element);
    }

    pub fn has_element(&self, id: UUID) -> bool {
        self.elements.contains_key(&id)
    }

    pub fn has_element_attribute(&self, id: UUID) -> bool {
        self.attributes.values().any(|attr| match attr {
            DMAttribute::Element(value) => value == &id,
            DMAttribute::ElementArray(values) => values.contains(&id),
            _ => false,
        })
    }

    pub fn remove_element(&mut self, name: &str) {
        self.remove_attribute(name)
    }

    pub fn get_attribute<T: Attribute>(&self, name: &str) -> Option<&T> {
        self.attributes.get(name).and_then(|attr| T::from_attribute(attr))
    }

    pub fn get_attributes(&self) -> &IndexMap<String, DMAttribute> {
        &self.attributes
    }

    pub fn set_attribute<T: Attribute>(&mut self, name: String, value: T) {
        self.attributes.insert(name, value.to_attribute());
    }

    pub fn remove_attribute(&mut self, name: &str) {
        let removed = self.attributes.shift_remove(name);
        if let Some(DMAttribute::Element(id)) = removed {
            self.elements.shift_remove(&id);
        }
    }

    pub fn get_class(&self) -> &str {
        &self.class
    }

    pub fn set_class(&mut self, class: String) {
        self.class = class;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_id(&self) -> &UUID {
        &self.id
    }

    pub fn set_id(&mut self, id: UUID) {
        self.id = id;
    }
}
