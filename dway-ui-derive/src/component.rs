use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{Attribute, Ident};

pub struct ComponentBuilder {
    pub name: Ident,

    pub attributes: Vec<Attribute>,
    pub fields: BTreeMap<String, TokenStream>,
}
impl ComponentBuilder {
    pub fn new(name: Ident) -> Self {
        Self {
            name,
            fields: Default::default(),
            attributes: Default::default(),
        }
    }
}

impl ToTokens for ComponentBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            name,
            attributes,
            fields,
        } = &self;
        let fields = fields.values();
        tokens.extend(quote! {
            #(#attributes)*
            #[derive(Component)]
            pub struct #name {
                #(#fields),*
            }
        });
    }
}
