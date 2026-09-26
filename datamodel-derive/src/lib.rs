use quote::quote;

#[proc_macro_derive(ElementClass, attributes(class_name, attribute))]
pub fn element_class_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tree = syn::parse::<syn::DeriveInput>(item).unwrap();
    let class_identifier = tree.ident;
    let class_name = tree
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("class_name"))
        .and_then(|attribute| attribute.parse_args::<syn::LitStr>().ok())
        .map(|class_name| class_name.value())
        .unwrap_or(class_identifier.to_string());

    let syn::Data::Struct(data_struct) = tree.data else {
        panic!("ElementClass Is Only Derivable From A Struct!");
    };

    let datamodel_name_space = import_datamodel();

    let mut class_interface_attributes = Vec::new();
    let mut class_interface_implementation = Vec::new();
    for field in data_struct.fields {
        let field_identifier = field.ident.expect("ElementClass Should Not Be Derived From Tuple Struct!");

        let Some(attribute_config) = field
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("attribute"))
            .map(parse_attribute_arguments)
        else {
            continue;
        };

        let attribute_name = attribute_config.name.unwrap_or(field_identifier.to_string());
        let attribute_getter = syn::Ident::new(&format!("get_{field_identifier}"), proc_macro2::Span::call_site());
        let attribute_getter_or = syn::Ident::new(&format!("get_{field_identifier}_or"), proc_macro2::Span::call_site());
        let attribute_getter_or_default = syn::Ident::new(&format!("get_{field_identifier}_or_default"), proc_macro2::Span::call_site());
        let attribute_getter_mut = syn::Ident::new(&format!("get_{field_identifier}_mut"), proc_macro2::Span::call_site());
        let attribute_getter_or_mut = syn::Ident::new(&format!("get_{field_identifier}_or_mut"), proc_macro2::Span::call_site());
        let attribute_getter_or_default_mut = syn::Ident::new(&format!("get_{field_identifier}_or_default_mut"), proc_macro2::Span::call_site());
        let attribute_type = field.ty;
        class_interface_attributes.push(quote! {
            fn #attribute_getter(&self) -> ::core::option::Option<::std::cell::Ref<'_, #attribute_type>>;
            fn #attribute_getter_or(&mut self, default: #attribute_type) -> ::std::cell::Ref<'_, #attribute_type>;
            fn #attribute_getter_or_default(&mut self) -> ::std::cell::Ref<'_, #attribute_type>;
            fn #attribute_getter_mut(&mut self) -> ::core::option::Option<::std::cell::RefMut<'_, #attribute_type>>;
            fn #attribute_getter_or_mut(&mut self, default: #attribute_type) -> ::std::cell::RefMut<'_, #attribute_type>;
            fn #attribute_getter_or_default_mut(&mut self) -> ::std::cell::RefMut<'_, #attribute_type>;
        });

        class_interface_implementation.push(quote! {
            fn #attribute_getter(&self) -> ::core::option::Option<::std::cell::Ref<'_, #attribute_type>> {
                self.get_value(#attribute_name).ok()
            }
            fn #attribute_getter_or(&mut self, default: #attribute_type) -> ::std::cell::Ref<'_, #attribute_type> {
                self.get_value_or(#attribute_name, default)
            }
            fn #attribute_getter_or_default(&mut self) -> ::std::cell::Ref<'_, #attribute_type> {
                self.get_value_or_default(#attribute_name)
            }
            fn #attribute_getter_mut(&mut self) -> ::core::option::Option<::std::cell::RefMut<'_, #attribute_type>> {
                self.get_value_mut(#attribute_name).ok()
            }
            fn #attribute_getter_or_mut(&mut self, default: #attribute_type) -> ::std::cell::RefMut<'_, #attribute_type> {
                self.get_value_or_mut(#attribute_name, default)
            }
            fn #attribute_getter_or_default_mut(&mut self) -> ::std::cell::RefMut<'_, #attribute_type> {
                self.get_value_or_default_mut(#attribute_name)
            }
        });
    }

    let class_interface = syn::Ident::new(&format!("{class_identifier}Interface"), class_identifier.span());

    quote! {
        impl #datamodel_name_space::ElementClass for #class_identifier {
            fn class_name() -> &'static str {
                #class_name
            }
        }

        trait #class_interface {
            #(#class_interface_attributes)*
        }

        impl #class_interface for #datamodel_name_space::Element<#class_identifier> {
            #(#class_interface_implementation)*
        }
    }
    .into()
}

fn import_datamodel() -> proc_macro2::TokenStream {
    let datamodel_crate = proc_macro_crate::crate_name("datamodel").expect("Dependency `datamodel` Should Be Present In `Cargo.toml`");
    match datamodel_crate {
        proc_macro_crate::FoundCrate::Itself => quote!(crate),
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!( ::#ident)
        }
    }
}

#[derive(Default)]
struct AttributeConfiguration {
    name: Option<String>,
}

fn parse_attribute_arguments(arguments: &syn::Attribute) -> AttributeConfiguration {
    let mut name = None;
    let _ = arguments.parse_nested_meta(|meta| {
        if meta.path.is_ident("name") {
            if name.is_some() {
                panic!("Duplicate Attribute Name Declaration!");
            }

            let value = meta.value()?;
            let attribute_name = value.parse::<syn::LitStr>()?;
            name = Some(attribute_name.value());
            return Ok(());
        }

        Ok(())
    });
    AttributeConfiguration { name }
}
