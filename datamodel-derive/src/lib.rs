use proc_macro::TokenStream;

#[proc_macro_derive(ElementClass, attributes(class_name, attribute_name, owner))]
pub fn element_class_derive(item: TokenStream) -> TokenStream {
    let tree = syn::parse::<syn::DeriveInput>(item).unwrap();

    let class_identifier = tree.ident;
    let class_name = tree
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("class_name"))
        .and_then(|attribute| attribute.parse_args::<syn::LitStr>().ok())
        .map(|class_name| class_name.value())
        .unwrap_or(class_identifier.to_string());

    let data_struct = match tree.data {
        syn::Data::Struct(data_struct) => data_struct,
        syn::Data::Enum(_) => panic!("Element Class Can't Be Derived From Enum!"),
        syn::Data::Union(_) => panic!("Element Class Can't Be Derived From Union!"),
    };

    let mut owner_attribute = None;
    let mut constructor_fields = Vec::new();
    let mut attribute_field_identifiers = Vec::new();
    for field in data_struct.fields {
        let field_identifier = field.ident.expect("Element Class Can't Be Derived From Tuple Structs!");
        let attribute_name = field
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("attribute_name"))
            .and_then(|attribute| attribute.parse_args::<syn::LitStr>().ok())
            .map(|attribute_name| attribute_name.value())
            .unwrap_or(field_identifier.to_string());

        if let syn::Type::Path(type_path) = field.ty
            && let Some(last_segment) = type_path.path.segments.last()
        {
            let field_type = last_segment.ident.to_string();
            if field_type == "AttributeVariable" {
                constructor_fields.push(quote::quote! {
                    #field_identifier: datamodel::attribute::AttributeVariable::initialize(datamodel::Element::clone(&element), #attribute_name)
                });

                if field.attrs.iter().any(|attribute| attribute.path().is_ident("owner")) {
                    if owner_attribute.is_some() {
                        panic!("Existing Attribute Marked As Owner")
                    }

                    owner_attribute = Some(field_identifier);
                } else {
                    attribute_field_identifiers.push(field_identifier);
                }
                continue;
            }

            if field_type == "AttributeElement" {
                constructor_fields.push(quote::quote! {
                    #field_identifier: datamodel::attribute::AttributeElement::initialize(datamodel::Element::clone(&element), #attribute_name)
                });

                if field.attrs.iter().any(|attribute| attribute.path().is_ident("owner")) {
                    if owner_attribute.is_some() {
                        panic!("Existing Attribute Marked As Owner")
                    }

                    owner_attribute = Some(field_identifier);
                } else {
                    attribute_field_identifiers.push(field_identifier);
                }
                continue;
            }

            if field_type == "AttributeElementArray" {
                constructor_fields.push(quote::quote! {
                    #field_identifier: datamodel::attribute::AttributeElementArray::initialize(datamodel::Element::clone(&element), #attribute_name)
                });

                if field.attrs.iter().any(|attribute| attribute.path().is_ident("owner")) {
                    if owner_attribute.is_some() {
                        panic!("Existing Attribute Marked As Owner")
                    }

                    owner_attribute = Some(field_identifier);
                } else {
                    attribute_field_identifiers.push(field_identifier);
                }
                continue;
            }
        }

        constructor_fields.push(quote::quote! {
            #field_identifier: Default::default()
        });

        if field.attrs.iter().any(|attribute| attribute.path().is_ident("owner")) && owner_attribute.is_some() {
            panic!("Non Attribute Marked As Owner")
        }
    }

    let into_element = if let Some(owner_identifier) = owner_attribute {
        owner_identifier
    } else {
        attribute_field_identifiers.first().expect("No Attributes In struct").clone()
    };

    quote::quote! {
        impl datamodel::ElementClass for #class_identifier {
            fn class_name() -> &'static str {
                #class_name
            }

            fn from_element(element: datamodel::Element) -> Self {
                Self {
                    #(#constructor_fields),*,
                }

            }

            fn into_element(self) -> datamodel::Element {
                let mut conversion = self.#into_element.owner();
                conversion.set_class::<Self>();
                conversion
            }
        }
    }
    .into()
}
