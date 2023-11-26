use convert_case::Casing;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{Attribute, Ident};

pub struct ComponentBuilder {
    pub name: Ident,
    pub attributes: Vec<TokenStream>,
    pub fields: BTreeMap<String, TokenStream>,
    pub init: BTreeMap<String, TokenStream>,
    pub generate_init: bool,
}
impl ComponentBuilder {
    pub fn new(name: Ident) -> Self {
        Self {
            name,
            fields: Default::default(),
            attributes: Default::default(),
            init: Default::default(),
            generate_init: false,
        }
    }
    pub fn add_field(&mut self, name: &Ident, field: TokenStream) {
        self.fields.insert(name.to_string(), field);
    }
    pub fn add_field_with_initer(&mut self, name: &Ident, field: TokenStream, init: TokenStream) {
        self.fields.insert(name.to_string(), field);
        self.init.insert(name.to_string(), init.to_token_stream());
    }
}

impl ToTokens for ComponentBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            name,
            attributes,
            fields,
            init,
            generate_init,
        } = &self;
        let fields = fields.values();
        tokens.extend(quote! {
            #(#attributes)*
            pub struct #name {
                #(#fields),*
            }
        });
        if *generate_init {
            let field_init = self
                .fields
                .keys()
                .map(|field| {
                    let ident = format_ident!("{}", field);
                    if let Some(value) = init.get(field) {
                        quote! {#ident: #value}
                    } else {
                        quote! {#ident: Default::default()}
                    }
                })
                .chain(self.init.iter().filter_map(|(key, init)| {
                    let field = format_ident!("{}", key);
                    if !self.fields.contains_key(key) {
                        Some(quote! {#field: #init})
                    } else {
                        None
                    }
                }));
            tokens.extend(quote! {
                impl Default for #name {
                    fn default() -> Self {
                        Self {
                             #(#field_init),*
                        }
                    }
                }
            });
        }
    }
}

pub struct PluginBuilder {
    pub components: Vec<ComponentBuilder>,
    pub resources: Vec<ResourceBuilder>,
    pub systems: Vec<TokenStream>,
    pub stmts: Vec<TokenStream>,
    pub other_items: Vec<TokenStream>,
    pub name: Ident,
}

impl PluginBuilder {
    pub fn new(name: Ident) -> Self {
        Self {
            components: vec![],
            resources: vec![],
            systems: vec![],
            stmts: vec![],
            other_items: vec![],
            name,
        }
    }
}

impl ToTokens for PluginBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            other_items,
            components,
            resources,
            systems,
            stmts,
            name,
        } = self;
        let resource_stmts = resources.iter().map(|r| r.into_plugin());
        tokens.extend(quote! {
            #(#components)*
            #(#resources)*
            #(#systems)*
            #(#other_items)*
            pub struct #name;
            impl Plugin for #name {
                fn build(&self, app: &mut App) {
                    #(#stmts)*
                    #(#resource_stmts)*
                }
            }
        });
    }
}

pub struct ResourceBuilder {
    pub name: Ident,
    pub attributes: Vec<TokenStream>,
    pub fields: BTreeMap<String, TokenStream>,
    pub init: BTreeMap<String, TokenStream>,
    pub generate_init: bool,
}
impl ResourceBuilder {
    pub fn new(name: Ident) -> Self {
        Self {
            name,
            fields: Default::default(),
            attributes: Default::default(),
            init: Default::default(),
            generate_init: false,
        }
    }
    pub fn add_field(&mut self, name: &Ident, field: TokenStream) {
        self.fields.insert(name.to_string(), field);
    }
    pub fn add_field_with_initer(&mut self, name: &Ident, field: TokenStream, init: TokenStream) {
        self.fields.insert(name.to_string(), field);
        self.init.insert(name.to_string(), init.to_token_stream());
    }
    pub fn into_plugin(&self) -> TokenStream {
        let Self {
            name,
            init,
            generate_init,
            ..
        } = &self;
        let resource_var = format_ident!(
            "resource_{}",
            name.to_string().to_case(convert_case::Case::Snake),
            span = name.span()
        );
        if *generate_init {
            let field_init = self
                .fields
                .keys()
                .map(|field| {
                    let ident = format_ident!("{}", field);
                    if let Some(value) = init.get(field) {
                        quote! {#ident: #value}
                    } else {
                        quote! {#ident: Default::default()}
                    }
                })
                .chain(self.init.iter().filter_map(|(key, init)| {
                    let field = format_ident!("{}", key);
                    if !self.fields.contains_key(key) {
                        Some(quote! {#field: #init})
                    } else {
                        None
                    }
                }));
            quote! {
                let #resource_var = #name {
                    #(#field_init),*
                };
                app.insert_resource(#resource_var);
            }
        } else {
            quote!()
        }
    }
}
impl ToTokens for ResourceBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            name,
            attributes,
            fields,
            ..
        } = &self;
        let fields = fields.values();
        tokens.extend(quote! {
            #(#attributes)*
            pub struct #name {
                #(#fields),*
            }
        });
    }
}
