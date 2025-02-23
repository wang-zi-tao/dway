use convert_case::Casing;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote_spanned, ToTokens};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use syn::{spanned::Spanned, Type};

pub struct QuerySetBuilder {
    pub name: Ident,
    pub items: HashMap<String, TokenStream>,
    pub names: HashMap<String, usize>,
    pub querys: HashMap<String, (Ident, TokenStream)>,
    pub methods: Vec<TokenStream>,
}

impl QuerySetBuilder {
    pub fn new(name: Ident) -> Self {
        Self {
            name,
            items: Default::default(),
            names: Default::default(),
            querys: Default::default(),
            methods: Default::default(),
        }
    }

    pub fn build(&self) -> TokenStream {
        let Self {
            name,
            items,
            querys,
            methods,
            ..
        } = self;
        let items_values = items.values();
        let querys_values = querys.values().map(|x| &x.1);
        quote_spanned! {name.span()=>
            #(
                #items_values
            )*

            #[allow(non_snake_case)]
            #[derive(bevy::ecs::system::SystemParam)]
            pub struct #name<'w, 's> {
                #(#querys_values),*
            }
            #[allow(dead_code)]
            #[allow(non_snake_case)]
            impl <'w, 's> #name<'w, 's> {
                #(#methods)*
            }
        }
    }
}

pub struct NodeInfo {
    pub name: Ident,
    pub extract_querys:Vec<(Ident, TokenStream)>,
    pub callback_arg: TokenStream,
    pub callback_type: TokenStream,
}

pub struct QueryBuilder {
    pub query_set: Rc<RefCell<QuerySetBuilder>>,
    pub params: Vec<TokenStream>,
    pub lifetimes: Vec<TokenStream>,
    pub generic_parameters: Vec<TokenStream>,
    pub callback_arg: TokenStream,
    pub code: TokenStream,
    pub node_stack: Vec<NodeInfo>,
    pub mutable: bool,
    pub has_begin_node: bool,
}

pub fn convert_type_name(ty: &TokenStream) -> String {
    let name = ty.to_token_stream().to_string();
    let name = name.replace('_', "__");
    let name = name.replace(
        |char: char| {
            !(char == '_'
                || char.is_ascii_digit()
                || char.is_ascii_uppercase()
                || char.is_ascii_lowercase())
        },
        "__",
    );

    name.to_case(convert_case::Case::Snake)
}

impl QueryBuilder {
    pub fn new(
        query_set: Rc<RefCell<QuerySetBuilder>>,
        has_begin_node: bool,
        mutable: bool,
    ) -> Self {
        Self {
            query_set,
            params: Default::default(),
            lifetimes: Default::default(),
            generic_parameters: Default::default(),
            code: Default::default(),
            callback_arg: Default::default(),
            node_stack: vec![],
            has_begin_node,
            mutable,
        }
    }

    pub fn alloc_name(&self, base_name: &str, span: Span) -> Ident {
        let mut query_set_builder = self.query_set.borrow_mut();
        let count = query_set_builder
            .names
            .entry(base_name.to_string())
            .or_default();
        let name = if *count == 0 {
            base_name.to_string()
        } else {
            format!("{base_name}{count}")
        };
        *count += 1;
        Ident::new(&name, span)
    }

    pub fn add_param(&mut self, tokens: TokenStream) {
        self.params.push(tokens);
    }

    pub fn add_generic_parameter(&mut self, tokens: TokenStream) {
        self.generic_parameters.push(tokens);
    }

    pub fn add_lifetime(&mut self, tokens: TokenStream) {
        self.lifetimes.push(tokens);
    }

    pub fn add_edge_query(&mut self, ty: TokenStream) -> Ident {
        let node = self.node_stack.last_mut().unwrap();
        let base_name = format!("edge_{}", node.name);
        let span = node.name.span();
        let query_var = self.alloc_name(&base_name, span);
        let node = self.node_stack.last_mut().unwrap();
        node.extract_querys.push((query_var.clone(), ty));
        query_var
    }

    pub fn add_query(&self, ty: &TokenStream, filter: Option<&Type>) -> Ident {
        let mut query_set_builder = self.query_set.borrow_mut();
        let key = ty.to_string();
        query_set_builder
            .querys
            .entry(key)
            .or_insert_with(|| {
                let name = format_ident!("query_{}", convert_type_name(ty));
                let query = quote_spanned!(ty.span()=> #name: Query<'w, 's, #ty, #filter>);
                (name, query)
            })
            .0
            .clone()
    }

    pub fn add_item(&self, name: &Ident, item: TokenStream) {
        let mut query_set_builder = self.query_set.borrow_mut();
        query_set_builder.items.insert(name.to_string(), item);
    }

    pub fn finish(self) {
        let Self {
            query_set, code, ..
        } = self;
        let mut query_set = query_set.borrow_mut();
        query_set.methods.push(code);
    }
}
