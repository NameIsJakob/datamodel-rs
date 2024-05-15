use indexmap::IndexMap;
use uuid::Uuid as UUID;

use crate::Attribute;

#[derive(Clone, Debug)]
pub struct Element {
    pub name: String,
    pub class: String,
    pub id: UUID,
    pub external: bool,
    attributes: IndexMap<String, Attribute>,
}

impl Element {
    pub fn new(name: String, class: String) -> Self {
        Self {
            name,
            class,
            id: UUID::new_v4(),
            external: false,
            attributes: IndexMap::new(),
        }
    }

    pub fn create(name: String, class: String, id: UUID) -> Self {
        Self {
            name,
            class,
            id,
            external: false,
            attributes: IndexMap::new(),
        }
    }

    pub fn reserve_attributes(&mut self, amount: usize) {
        self.attributes.reserve(amount)
    }

    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }

    pub fn set_attribute<N: Into<String>>(&mut self, name: N, value: Attribute) {
        self.attributes.insert(name.into(), value);
    }

    pub fn get_attributes(&self) -> &IndexMap<String, Attribute> {
        &self.attributes
    }
}

impl Default for Element {
    fn default() -> Self {
        Self {
            name: String::from("unnamed"),
            class: String::from("DmElement"),
            id: UUID::new_v4(),
            external: Default::default(),
            attributes: Default::default(),
        }
    }
}
