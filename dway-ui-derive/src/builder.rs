use std::collections::BTreeMap;

use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Generics, Ident};

pub struct ComponentBuilder {
    pub name: Ident,
    pub generics: Generics,
    pub attributes: Vec<TokenStream>,
    pub fields: BTreeMap<String, TokenStream>,
    pub init: BTreeMap<String, TokenStream>,
    pub generate_init: bool,
}
impl ComponentBuilder {
    pub fn new(name: Ident) -> Self {
        Self {
            name,
            generics: Default::default(),
            fields: Default::default(),
            attributes: Default::default(),
            init: Default::default(),
            generate_init: false,
        }
    }

    pub fn new_with_generics(name: Ident, generics: Generics) -> Self {
        Self {
            name,
            generics,
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

    pub fn get_type(&self) -> TokenStream {
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let name = &self.name;
        quote! {#name #ty_generics}
    }
}

impl ToTokens for ComponentBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            name,
            generics,
            attributes,
            fields,
            init,
            generate_init,
        } = &self;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let fields = fields.values();
        tokens.extend(quote! {
            #(#attributes)*
            pub struct #name #impl_generics #where_clause {
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
                impl #impl_generics Default for #name #ty_generics #where_clause {
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
    pub hot_reload_stmts: Vec<TokenStream>,
    pub other_items: Vec<TokenStream>,
    pub name: Ident,
    pub generics: Generics,
    pub enable_hot_reload: bool,
}

impl PluginBuilder {
    pub fn new(name: Ident, generics: Generics, enable_hot_reload: bool) -> Self {
        Self {
            components: vec![],
            resources: vec![],
            systems: vec![],
            stmts: vec![],
            other_items: vec![],
            hot_reload_stmts: vec![],
            name,
            generics,
            enable_hot_reload,
        }
    }
}

impl ToTokens for PluginBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            hot_reload_stmts,
            other_items,
            components,
            resources,
            systems,
            stmts,
            name,
            generics,
            enable_hot_reload,
        } = self;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let (hot_reload_loader, hot_reload_register, plugin_stats) = if *enable_hot_reload {
            let reloadable_mod_name = format_ident!(
                "{}_reload_mod",
                name.to_string().to_case(convert_case::Case::Snake),
                span = name.span()
            );
            let reloadable_name = format_ident!(
                "{}_reload",
                name.to_string().to_case(convert_case::Case::Snake),
                span = name.span()
            );
            (
                Some(quote! {
                    mod #reloadable_mod_name{
                        use super::*;
                        use dway_ui_framework::reexport::bevy_dexterous_developer::{self,*};
                        reloadable_scope!(#reloadable_name(app){
                            #(#hot_reload_stmts)*
                        });
                    }
                    pub use #reloadable_mod_name::*;
                }),
                Some(quote! {
                    use dway_ui_framework::reexport::bevy_dexterous_developer::ReloadableElementsSetup as _;
                    app.setup_reloadable_elements::<#reloadable_name>();
                }),
                quote! {
                    #(#stmts)*
                },
            )
        } else {
            (
                None::<TokenStream>,
                None::<TokenStream>,
                quote! {
                    #(#hot_reload_stmts)*
                    #(#stmts)*
                },
            )
        };

        let bundle_struct = if generics.type_params().next().is_some() {
            let type_params = generics.type_params().map(|g| &g.ident);
            Some(quote! {(std::marker::PhantomData<(#(#type_params),*)>)})
        } else {
            None
        };

        let resource_stmts = resources.iter().map(|r| r.into_plugin());
        tokens.extend(quote! {
            #hot_reload_loader

            #(#components)*
            #(#resources)*
            #(#systems)*
            #(#other_items)*
            pub struct #name #impl_generics #bundle_struct;
            impl #impl_generics Plugin for #name #ty_generics #where_clause {
                fn build(&self, app: &mut App) {
                    #plugin_stats
                    #(#resource_stmts)*
                    #hot_reload_register
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
