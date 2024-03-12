use std::collections::HashMap;

use indexmap::IndexMap;
use uuid::Uuid as UUID;

use crate::attributes::Attribute;

#[derive(Clone, Debug)]
pub struct Element {
    pub name: String,
    pub class: String,
    id: UUID,
    attributes: IndexMap<String, Attribute>,
    elements: HashMap<UUID, Element>,
}

impl Element {
    pub fn new<N, C>(name: N, class: C) -> Self
    where
        N: Into<String>,
        C: Into<String>,
    {
        Self {
            name: name.into(),
            class: class.into(),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
            elements: HashMap::new(),
        }
    }

    pub fn create<N, C>(name: N, class: C, id: UUID) -> Self
    where
        N: Into<String>,
        C: Into<String>,
    {
        Self {
            name: name.into(),
            class: class.into(),
            id,
            attributes: IndexMap::new(),
            elements: HashMap::new(),
        }
    }

    pub fn get_id(&self) -> UUID {
        self.id
    }

    pub fn add_attribute<S, A>(&mut self, name: S, value: A)
    where
        S: Into<String>,
        A: Into<Attribute>,
    {
        let attribute = value.into();

        match attribute {
            Attribute::ElementData(element) => {
                let element_id = element.get_id();
                self.elements.insert(element.get_id(), element);
                self.add_attribute(name, Attribute::Element(element_id));
            }

            Attribute::ElementDataArray(elements) => {
                let mut element_ids: Vec<UUID> = Vec::with_capacity(elements.len());
                for element in elements {
                    element_ids.push(element.get_id());
                    self.elements.insert(element.get_id(), element);
                }
                self.add_attribute(name, Attribute::ElementArray(element_ids));
            }

            _ => {
                self.attributes.insert(name.into(), attribute);
            }
        }
    }

    pub fn get_attribute<'a, A>(&'a self, name: &'a str) -> Option<&'a A>
    where
        &'a A: TryFrom<&'a Attribute>,
    {
        let attribute = self.attributes.get(name)?;
        let convertion = attribute.try_into().ok()?;
        Some(convertion)
    }

    pub fn get_attributes(&self) -> &IndexMap<String, Attribute> {
        &self.attributes
    }

    pub fn add_element<E>(&mut self, element: E)
    where
        E: Into<Element>,
    {
        let element = element.into();
        self.elements.insert(element.get_id(), element);
    }

    pub fn get_element<'a, E>(&'a self, id: &UUID) -> Option<E>
    where
        E: TryFrom<&'a Element>,
    {
        let element = self.elements.get(id)?;
        let convertion = element.try_into().ok()?;
        Some(convertion)
    }

    pub fn get_elements(&self) -> &HashMap<UUID, Element> {
        &self.elements
    }

    pub fn has_element_attribute(&self, id: UUID) -> bool {
        self.attributes.values().any(|attr| match attr {
            Attribute::Element(value) => value == &id,
            Attribute::ElementArray(values) => values.contains(&id),
            _ => false,
        })
    }
}

impl Default for Element {
    fn default() -> Self {
        Self {
            name: String::from("root"),
            class: String::from("DmElement"),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
            elements: HashMap::new(),
        }
    }
}
