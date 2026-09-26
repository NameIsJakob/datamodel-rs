use crate::attribute::{Attribute, AttributeInfo, AttributeType, AttributeValue};
use indexmap::IndexMap;
use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};
use thiserror::Error as ThisError;
use uuid::Uuid as UUID;

struct ElementInternal {
    class: String,
    id: UUID,
    attributes: IndexMap<String, Attribute>,
}

/// A reference-counted, structure that stores attributes.
///
/// A Element has a class label to specify what data is stored in the element.
///
/// # Panics
/// Borrowing rules from [RefCell] apply:
/// operations may panic if runtime borrow rules are violated
pub struct Element<Class: ElementClass = ()> {
    inner: Rc<RefCell<ElementInternal>>,
    class_marker: PhantomData<Class>,
}

impl<Class: ElementClass> Clone for Element<Class> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            class_marker: self.class_marker,
        }
    }
}

impl<Class: ElementClass> std::fmt::Debug for Element<Class> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let internal = self.inner.borrow();
        writeln!(f, "Element {} {} {{", internal.class, internal.id)?;

        for (attribute_name, attribute) in &internal.attributes {
            let attribute_value = match attribute.get_inner_value() {
                AttributeValue::Element(element) => {
                    if let Some(element_value) = element {
                        format!("Element(Some({:?}))", element_value.inner.borrow().id)
                    } else {
                        String::from("Element(None)")
                    }
                }
                AttributeValue::ElementArray(elements) => {
                    let mut element_values = Vec::with_capacity(elements.len());
                    for element in elements {
                        if let Some(element_value) = element {
                            element_values.push(format!("Some({:?})", element_value.inner.borrow().id));
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

impl<Class: ElementClass> Default for Element<Class> {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ElementInternal {
                class: String::from(Element::class_name()),
                id: UUID::new_v4(),
                attributes: IndexMap::new(),
            })),
            class_marker: Default::default(),
        }
    }
}

impl<Class: ElementClass> Eq for Element<Class> {}

impl<Class: ElementClass> std::hash::Hash for Element<Class> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.borrow().id.hash(state);
    }
}

impl<Class: ElementClass> PartialEq for Element<Class> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.borrow().id == other.inner.borrow().id
    }
}

impl<Class: ElementClass> Element<Class> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ElementInternal {
                class: Class::class_name().to_owned(),
                id: UUID::new_v4(),
                attributes: IndexMap::new(),
            })),
            class_marker: Default::default(),
        }
    }

    pub fn full(class: impl Into<String>, id: UUID) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ElementInternal {
                class: class.into(),
                id,
                attributes: IndexMap::new(),
            })),
            class_marker: Default::default(),
        }
    }

    pub fn convert<N: ElementClass>(self) -> Element<N> {
        Element::<N> {
            inner: self.inner,
            class_marker: Default::default(),
        }
    }

    pub fn cast<N: ElementClass>(&self) -> &Element<N> {
        // SAFETY: The size of the Element does not change.
        unsafe { std::mem::transmute(self) }
    }
}

#[derive(Debug, ThisError)]
pub enum ElementError {
    #[error("Borrow Error: {0}")]
    BorrowingError(#[from] std::cell::BorrowError),
    #[error("Borrow Mut Error: {0}")]
    BorrowingMutError(#[from] std::cell::BorrowMutError),
    #[error("Attribute {attribute_name} Does Not Exist")]
    NonExistingAttribute { attribute_name: String },
    #[error("Attribute {attribute_name} Is Not Type {attribute_type:?}")]
    WrongAttributeType { attribute_name: String, attribute_type: AttributeType },
}

impl<Class: ElementClass> Element<Class> {
    pub fn get_class(&self) -> Ref<'_, String> {
        Ref::map(self.inner.borrow(), |internals| &internals.class)
    }

    pub fn try_get_class(&self) -> Result<Ref<'_, String>, ElementError> {
        Ok(Ref::map(self.inner.try_borrow()?, |internals| &internals.class))
    }

    pub fn set_class_name(&mut self, class: impl Into<String>) {
        let mut internals = self.inner.borrow_mut();
        internals.class = class.into();
    }

    pub fn set_class<E: ElementClass>(&mut self) {
        self.set_class_name(E::class_name());
    }

    pub fn try_set_class_name(&mut self, class: impl Into<String>) -> Result<(), ElementError> {
        let mut internals = self.inner.try_borrow_mut()?;
        internals.class = class.into();
        Ok(())
    }

    pub fn try_set_class<E: ElementClass>(&mut self) -> Result<(), ElementError> {
        self.try_set_class_name(E::class_name())
    }

    pub fn get_id(&self) -> Ref<'_, UUID> {
        Ref::map(self.inner.borrow(), |internals| &internals.id)
    }

    pub fn try_get_id(&self) -> Result<Ref<'_, UUID>, ElementError> {
        Ok(Ref::map(self.inner.borrow(), |internals| &internals.id))
    }

    pub fn set_id(&mut self, id: UUID) {
        let mut internals = self.inner.borrow_mut();
        internals.id = id;
    }

    pub fn try_set_id(&mut self, id: UUID) -> Result<(), ElementError> {
        let mut internals = self.inner.try_borrow_mut()?;
        internals.id = id;
        Ok(())
    }
}

impl<Class: ElementClass> Element<Class> {
    pub fn get_value<T: AttributeInfo>(&self, name: impl AsRef<str>) -> Result<Ref<'_, T>, ElementError> {
        let internals = self.inner.try_borrow()?;
        let attribute_name = name.as_ref();

        if !internals.attributes.contains_key(attribute_name) {
            return Err(ElementError::NonExistingAttribute {
                attribute_name: attribute_name.to_owned(),
            });
        }

        Ref::filter_map(internals, |internals| {
            internals
                .attributes
                .get(attribute_name)
                .and_then(|attribute| T::get_attribute_value(attribute.get_inner_value()))
        })
        .map_err(|_| ElementError::WrongAttributeType {
            attribute_name: attribute_name.to_owned(),
            attribute_type: T::attribute_type(),
        })
    }

    pub fn get_value_or<T: AttributeInfo>(&mut self, name: impl AsRef<str>, default: T) -> Ref<'_, T> {
        let attribute_name = name.as_ref();
        let exists = self.inner.borrow_mut().attributes.contains_key(attribute_name);
        if !exists {
            self.set_value(attribute_name.to_owned(), default);
        }
        self.get_value(attribute_name).expect("value Was Inserted And Will Exist")
    }

    pub fn try_get_value_or<T: AttributeInfo>(&mut self, name: impl AsRef<str>, default: T) -> Result<Ref<'_, T>, ElementError> {
        let attribute_name = name.as_ref();
        let exists = self.inner.try_borrow_mut()?.attributes.contains_key(attribute_name);
        if !exists {
            self.set_value(attribute_name.to_owned(), default);
        }
        self.get_value(attribute_name)
    }

    pub fn get_value_or_default<T: AttributeInfo>(&mut self, name: impl AsRef<str>) -> Ref<'_, T> {
        self.get_value_or(name, T::default())
    }

    pub fn try_get_value_or_default<T: AttributeInfo>(&mut self, name: impl AsRef<str>) -> Result<Ref<'_, T>, ElementError> {
        self.try_get_value_or(name, T::default())
    }

    pub fn get_value_mut<T: AttributeInfo>(&mut self, name: impl AsRef<str>) -> Result<RefMut<'_, T>, ElementError> {
        let internals = self.inner.try_borrow_mut()?;
        let attribute_name = name.as_ref();

        if !internals.attributes.contains_key(attribute_name) {
            return Err(ElementError::NonExistingAttribute {
                attribute_name: attribute_name.to_owned(),
            });
        }

        RefMut::filter_map(internals, |internals| {
            internals
                .attributes
                .get_mut(attribute_name)
                .and_then(|attribute| T::get_attribute_value_mut(attribute.get_inner_value_mut()))
        })
        .map_err(|_| ElementError::WrongAttributeType {
            attribute_name: attribute_name.to_owned(),
            attribute_type: T::attribute_type(),
        })
    }

    pub fn get_value_or_mut<T: AttributeInfo>(&mut self, name: impl AsRef<str>, default: T) -> RefMut<'_, T> {
        let attribute_name = name.as_ref();
        let exists = self.inner.borrow_mut().attributes.contains_key(attribute_name);
        if !exists {
            self.set_value(attribute_name.to_owned(), default);
        }
        self.get_value_mut(attribute_name).expect("value Was Inserted And Will Exist")
    }

    pub fn try_get_value_or_mut<T: AttributeInfo>(&mut self, name: impl AsRef<str>, default: T) -> Result<RefMut<'_, T>, ElementError> {
        let attribute_name = name.as_ref();
        let exists = self.inner.try_borrow_mut()?.attributes.contains_key(attribute_name);
        if !exists {
            self.set_value(attribute_name.to_owned(), default);
        }
        self.get_value_mut(attribute_name)
    }

    pub fn get_value_or_default_mut<T: AttributeInfo>(&mut self, name: impl AsRef<str>) -> RefMut<'_, T> {
        self.get_value_or_mut(name, T::default())
    }

    pub fn try_get_value_or_default_mut<T: AttributeInfo>(&mut self, name: impl AsRef<str>) -> Result<RefMut<'_, T>, ElementError> {
        self.try_get_value_or_mut(name, T::default())
    }

    pub fn set_value(&mut self, name: impl Into<String>, value: impl AttributeInfo) {
        let attribute_name = name.into();
        self.inner.borrow_mut().attributes.insert(attribute_name, value.into_attribute());
    }

    pub fn try_set_value(&mut self, name: impl Into<String>, value: impl AttributeInfo) -> Result<(), ElementError> {
        let attribute_name = name.into();
        self.inner.try_borrow_mut()?.attributes.insert(attribute_name, value.into_attribute());
        Ok(())
    }
}

impl<Class: ElementClass> Element<Class> {
    pub fn get_attribute(&self, name: impl AsRef<str>) -> Option<Ref<'_, Attribute>> {
        Ref::filter_map(self.inner.borrow(), |internals| internals.attributes.get(name.as_ref())).ok()
    }

    pub fn set_attribute(&mut self, name: impl Into<String>, attribute: Attribute) -> Option<Attribute> {
        let attribute_name = name.into();
        self.inner.borrow_mut().attributes.insert(attribute_name, attribute)
    }

    pub fn remove_attribute(&mut self, name: impl AsRef<str>) -> Option<Attribute> {
        let mut element_data = self.inner.borrow_mut();
        element_data.attributes.shift_remove(name.as_ref())
    }

    pub fn get_attributes(&self) -> Ref<'_, IndexMap<String, Attribute>> {
        let element_data = self.inner.borrow();
        Ref::map(element_data, |element| &element.attributes)
    }

    pub fn reserve_attributes(&mut self, additional: usize) {
        let mut element_data = self.inner.borrow_mut();
        element_data.attributes.reserve(additional);
    }

    pub fn set_attributes(&mut self, attributes: IndexMap<String, Attribute>) {
        self.inner.borrow_mut().attributes = attributes;
    }
}

#[cfg(feature = "derive")]
pub use datamodel_derive::ElementClass;
pub trait ElementClass {
    fn class_name() -> &'static str;
}

impl ElementClass for () {
    fn class_name() -> &'static str {
        "DmElement"
    }
}

impl ElementClass for Element {
    fn class_name() -> &'static str {
        <() as ElementClass>::class_name()
    }
}
