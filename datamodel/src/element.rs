use crate::attribute::{Attribute, AttributeInfo, AttributeType, AttributeValue};
use indexmap::{IndexMap, map::RawEntryApiV1};
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

pub enum AttributeEntry<'a, T: AttributeInfo> {
    Occupied(OccupiedAttributeEntry<'a, T>),
    Vacant(VacantAttributeEntry<'a, T>),
}

pub struct OccupiedAttributeEntry<'a, T: AttributeInfo> {
    owner: &'a Element,
    key: &'a str,
    index: usize,
    type_marker: PhantomData<T>,
}

pub struct VacantAttributeEntry<'a, T: AttributeInfo> {
    owner: &'a Element,
    key: &'a str,
    type_marker: PhantomData<T>,
}

impl<'a, T: AttributeInfo> AttributeEntry<'a, T> {
    fn new(owner: &'a Element, key: &'a str) -> Self {
        match owner.inner.borrow_mut().attributes.raw_entry_mut_v1().from_key(key) {
            indexmap::map::raw_entry_v1::RawEntryMut::Occupied(occupied) => AttributeEntry::Occupied(OccupiedAttributeEntry {
                owner,
                key,
                index: occupied.index(),
                type_marker: PhantomData,
            }),
            indexmap::map::raw_entry_v1::RawEntryMut::Vacant(_) => AttributeEntry::Vacant(VacantAttributeEntry {
                owner,
                key,
                type_marker: PhantomData,
            }),
        }
    }

    fn try_new(owner: &'a Element, key: &'a str) -> Result<Self, ElementError> {
        Ok(match owner.inner.try_borrow_mut()?.attributes.raw_entry_mut_v1().from_key(key) {
            indexmap::map::raw_entry_v1::RawEntryMut::Occupied(occupied) => AttributeEntry::Occupied(OccupiedAttributeEntry {
                owner,
                key,
                index: occupied.index(),
                type_marker: PhantomData,
            }),
            indexmap::map::raw_entry_v1::RawEntryMut::Vacant(_) => AttributeEntry::Vacant(VacantAttributeEntry {
                owner,
                key,
                type_marker: PhantomData,
            }),
        })
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Occupied(entry) => entry.index(),
            Self::Vacant(entry) => entry.index(),
        }
    }

    pub fn try_index(&self) -> Result<usize, ElementError> {
        match self {
            Self::Occupied(entry) => Ok(entry.index()),
            Self::Vacant(entry) => entry.try_index(),
        }
    }

    pub fn key(&self) -> &str {
        match *self {
            Self::Occupied(ref entry) => entry.key(),
            Self::Vacant(ref entry) => entry.key(),
        }
    }

    pub fn insert_entry(self, value: T) -> OccupiedAttributeEntry<'a, T> {
        match self {
            Self::Occupied(mut entry) => {
                entry.insert(value);
                entry
            }
            Self::Vacant(entry) => entry.insert_entry(value),
        }
    }

    pub fn try_insert_entry(self, value: T) -> Result<OccupiedAttributeEntry<'a, T>, ElementError> {
        match self {
            Self::Occupied(mut entry) => {
                entry.try_insert(value)?;
                Ok(entry)
            }
            Self::Vacant(entry) => entry.try_insert_entry(value),
        }
    }

    pub fn or_insert(self, default: T) -> RefMut<'a, T> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    pub fn try_or_insert(self, default: T) -> Result<RefMut<'a, T>, ElementError> {
        match self {
            Self::Occupied(entry) => entry.try_into_mut(),
            Self::Vacant(entry) => entry.try_insert(default),
        }
    }

    pub fn or_insert_with(self, call: impl FnOnce() -> T) -> RefMut<'a, T> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(call()),
        }
    }

    pub fn try_or_insert_with(self, call: impl FnOnce() -> T) -> Result<RefMut<'a, T>, ElementError> {
        match self {
            Self::Occupied(entry) => entry.try_into_mut(),
            Self::Vacant(entry) => entry.try_insert(call()),
        }
    }

    pub fn or_insert_with_key(self, call: impl FnOnce(&str) -> T) -> RefMut<'a, T> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => {
                let value = call(entry.key());
                entry.insert(value)
            }
        }
    }

    pub fn try_or_insert_with_key(self, call: impl FnOnce(&str) -> T) -> Result<RefMut<'a, T>, ElementError> {
        match self {
            Self::Occupied(entry) => entry.try_into_mut(),
            Self::Vacant(entry) => {
                let value = call(entry.key());
                entry.try_insert(value)
            }
        }
    }

    pub fn or_default(self) -> RefMut<'a, T> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(T::default()),
        }
    }

    pub fn try_or_default(self) -> Result<RefMut<'a, T>, ElementError> {
        match self {
            Self::Occupied(entry) => entry.try_into_mut(),
            Self::Vacant(entry) => entry.try_insert(T::default()),
        }
    }
}

impl<'a, T: AttributeInfo> OccupiedAttributeEntry<'a, T> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn key(&self) -> &str {
        self.key
    }

    pub fn get_mut(&mut self) -> RefMut<'a, T> {
        if self.owner.inner.borrow().attributes[self.index].get_type() != T::attribute_type() {
            self.insert(T::default());
        }
        RefMut::map(self.owner.inner.borrow_mut(), |internals| {
            internals
                .attributes
                .get_index_mut(self.index)
                .map(|(_, attribute)| T::get_attribute_value_mut(attribute.get_inner_value_mut()).expect("attribute should be correct type"))
                .expect("entry should be exist")
        })
    }

    pub fn into_mut(mut self) -> RefMut<'a, T> {
        if self.owner.inner.borrow().attributes[self.index].get_type() != T::attribute_type() {
            OccupiedAttributeEntry::insert(&mut self, T::default());
        }
        RefMut::map(self.owner.inner.borrow_mut(), |internals| {
            internals
                .attributes
                .get_index_mut(self.index)
                .map(|(_, attribute)| T::get_attribute_value_mut(attribute.get_inner_value_mut()).expect("attribute should be correct type"))
                .expect("entry should be exist")
        })
    }

    pub fn try_into_mut(self) -> Result<RefMut<'a, T>, ElementError> {
        if self.owner.inner.borrow().attributes[self.index].get_type() != T::attribute_type() {
            return Err(ElementError::WrongAttributeType {
                attribute_name: self.key.to_owned(),
                attribute_type: T::attribute_type(),
            });
        }
        Ok(RefMut::map(self.owner.inner.try_borrow_mut()?, |internals| {
            internals
                .attributes
                .get_index_mut(self.index)
                .map(|(_, attribute)| T::get_attribute_value_mut(attribute.get_inner_value_mut()).expect("attribute should be correct type"))
                .expect("entry should be exist")
        }))
    }

    pub fn insert(&mut self, value: T) -> Attribute {
        self.owner
            .inner
            .borrow_mut()
            .attributes
            .insert(self.key.to_owned(), value.into_attribute())
            .expect("entry should be exist")
    }

    pub fn try_insert(&mut self, value: T) -> Result<Attribute, ElementError> {
        Ok(self
            .owner
            .inner
            .try_borrow_mut()?
            .attributes
            .insert(self.key.to_owned(), value.into_attribute())
            .expect("entry should be exist"))
    }
}

impl<'a, T: AttributeInfo> VacantAttributeEntry<'a, T> {
    pub fn index(&self) -> usize {
        self.owner.inner.borrow().attributes.len()
    }

    pub fn try_index(&self) -> Result<usize, ElementError> {
        Ok(self.owner.inner.try_borrow()?.attributes.len())
    }

    pub fn key(&self) -> &str {
        self.key
    }

    pub fn insert(self, value: T) -> RefMut<'a, T> {
        self.owner.inner.borrow_mut().attributes.insert(self.key.to_owned(), value.into_attribute());
        RefMut::map(self.owner.inner.borrow_mut(), |internals| {
            internals
                .attributes
                .last_mut()
                .map(|(_, attribute)| T::get_attribute_value_mut(attribute.get_inner_value_mut()).expect("attribute should be correct type"))
                .expect("entry should be exist")
        })
    }

    pub fn try_insert(self, value: T) -> Result<RefMut<'a, T>, ElementError> {
        self.owner.inner.borrow_mut().attributes.insert(self.key.to_owned(), value.into_attribute());
        Ok(RefMut::map(self.owner.inner.try_borrow_mut()?, |internals| {
            internals
                .attributes
                .last_mut()
                .map(|(_, attribute)| T::get_attribute_value_mut(attribute.get_inner_value_mut()).expect("attribute should be correct type"))
                .expect("entry should be exist")
        }))
    }

    pub fn insert_entry(self, value: T) -> OccupiedAttributeEntry<'a, T> {
        self.owner.inner.borrow_mut().attributes.insert(self.key.to_owned(), value.into_attribute());
        OccupiedAttributeEntry {
            owner: self.owner,
            key: self.key,
            index: self.owner.inner.borrow().attributes.len(),
            type_marker: PhantomData,
        }
    }

    pub fn try_insert_entry(self, value: T) -> Result<OccupiedAttributeEntry<'a, T>, ElementError> {
        self.owner
            .inner
            .try_borrow_mut()?
            .attributes
            .insert(self.key.to_owned(), value.into_attribute());
        Ok(OccupiedAttributeEntry {
            owner: self.owner,
            key: self.key,
            index: self.owner.inner.borrow().attributes.len(),
            type_marker: PhantomData,
        })
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

    pub fn entry<'a, T: AttributeInfo>(&'a mut self, name: &'a str) -> AttributeEntry<'a, T> {
        AttributeEntry::new(self.cast(), name)
    }

    pub fn try_entry<'a, T: AttributeInfo>(&'a mut self, name: &'a str) -> Result<AttributeEntry<'a, T>, ElementError> {
        AttributeEntry::try_new(self.cast(), name)
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
