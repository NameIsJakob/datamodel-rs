use crate::attribute::{Attribute, AttributeValue};
use indexmap::IndexMap;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};
use uuid::Uuid as UUID;

struct ElementInternal {
    class: String,
    id: UUID,
    attributes: IndexMap<String, Attribute>,
}

#[derive(Clone)]
pub struct Element(Rc<RefCell<ElementInternal>>);

impl Default for Element {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(ElementInternal {
            class: String::from(Element::class_name()),
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

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let internal = self.0.borrow();
        writeln!(f, "Element {} {} {{", internal.class, internal.id)?;

        for (attribute_name, attribute) in &internal.attributes {
            let attribute_value = match &*attribute.get_inner() {
                AttributeValue::Element(element) => {
                    if let Some(element_value) = element {
                        format!("Element(Some({:?}))", element_value.0.borrow().id)
                    } else {
                        String::from("Element(None)")
                    }
                }
                AttributeValue::ElementArray(elements) => {
                    let mut element_values = Vec::with_capacity(elements.len());
                    for element in elements {
                        if let Some(element_value) = element {
                            element_values.push(format!("Some({:?})", element_value.0.borrow().id));
                        } else {
                            element_values.push(String::from("None"));
                        }
                    }
                    format!("ElementArray([{}])", element_values.join(", "))
                }
                value => format!("{value:?}"),
            };
            writeln!(f, "\t\"{attribute_name}\" {attribute_value}")?;
        }

        write!(f, "}}")
    }
}

impl ElementClass for Element {
    fn class_name() -> &'static str {
        "DmElement"
    }

    fn from_element(element: Element) -> Self {
        element
    }

    fn into_element(self) -> Element {
        self
    }
}

impl Element {
    pub fn new(class: impl Into<String>) -> Self {
        Self(Rc::new(RefCell::new(ElementInternal {
            class: class.into(),
            id: UUID::new_v4(),
            attributes: IndexMap::new(),
        })))
    }

    pub fn full(class: impl Into<String>, id: UUID) -> Self {
        Self(Rc::new(RefCell::new(ElementInternal {
            class: class.into(),
            id,
            attributes: IndexMap::new(),
        })))
    }

    pub fn get_class(&'_ self) -> Ref<'_, String> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element| &element.class)
    }

    pub fn set_class<E: ElementClass>(&mut self) {
        self.set_class_name(E::class_name());
    }

    pub fn set_class_name(&mut self, class: impl Into<String>) {
        let mut element_data = self.0.borrow_mut();
        element_data.class = class.into();
    }

    pub fn get_id(&'_ self) -> Ref<'_, UUID> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element| &element.id)
    }

    pub fn set_id(&mut self, id: UUID) {
        let mut element_data = self.0.borrow_mut();
        element_data.id = id;
    }

    pub fn get_attribute(&self, name: impl AsRef<str>) -> Option<Attribute> {
        let attribute_name = name.as_ref();
        self.0.borrow().attributes.get(attribute_name).cloned()
    }

    pub fn remove_attribute(&mut self, name: impl AsRef<str>) -> Option<Attribute> {
        let mut element_data = self.0.borrow_mut();
        let attribute_name = name.as_ref();
        element_data.attributes.shift_remove(attribute_name)
    }

    pub fn set_attribute(&mut self, name: impl Into<String>, attribute: Attribute) -> Option<Attribute> {
        let attribute_name = name.into();
        self.0.borrow_mut().attributes.insert(attribute_name, attribute)
    }

    pub fn get_attributes(&self) -> Ref<'_, IndexMap<String, Attribute>> {
        let element_data = self.0.borrow();
        Ref::map(element_data, |element| &element.attributes)
    }

    pub fn reserve_attributes(&mut self, additional: usize) {
        let mut element_data = self.0.borrow_mut();
        element_data.attributes.reserve(additional);
    }
}

#[cfg(feature = "derive")]
pub use datamodel_derive::ElementClass;
pub trait ElementClass {
    fn class_name() -> &'static str;

    fn from_element(element: Element) -> Self;
    fn into_element(self) -> Element;
}
