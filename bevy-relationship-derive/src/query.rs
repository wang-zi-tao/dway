use convert_case::Casing;
use derive_syn_parse::Parse;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use syn::{punctuated::Punctuated, spanned::Spanned, Token};

use crate::{filter::Filter, PathQuery};

structstruck::strike! {
    pub struct ReturnStat{
        pub variables: Punctuated<Ident, Token![,]>,
    }
}

#[derive(Parse)]
pub struct GraphQuery {
    pub name: Ident,
    pub eq: Token![=],
    pub match_: Token![match],
    pub path: PathQuery,
    #[prefix(Option<Token![where]> as where_)]
    #[parse_if(where_.is_some())]
    pub filter: Option<Filter>,
}

impl GraphQuery {
    pub fn build_foreach(
        &self,
        query_set: Rc<RefCell<QuerySetBuilder>>,
        mutable: bool,
    ) -> QueryBuilder {
        let mut query_builder = QueryBuilder {
            query_set,
            ..Default::default()
        };
        let inner = &query_builder.code;
        let mutable_token = if mutable { quote!(mut) } else { quote!() };
        let func = if mutable {
            format_ident!("foreach_{}", self.name)
        } else {
            format_ident!("foreach_{}_mut", self.name)
        };
        let function = quote! {
            pub fn #func(&#mutable_token self, callback) {
                #inner
            }
        };
        query_builder.code = function;
        query_builder
    }
}

#[derive(Parse)]
pub struct GraphQuerySet {
    #[call(Punctuated::parse_terminated)]
    pub querys: Punctuated<GraphQuery, Token![;]>,
}

#[derive(Default)]
pub struct QuerySetBuilder {
    pub items: HashMap<String, TokenStream>,
    pub names: HashMap<String, usize>,
    pub querys: HashMap<String, (Ident, TokenStream)>,
}

#[derive(Default)]
pub struct QueryBuilder {
    pub query_set: Rc<RefCell<QuerySetBuilder>>,
    pub code: TokenStream,
    pub entity_vars: Vec<(Ident,TokenStream)>,
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
    pub fn alloc_name(&self, base_name: &str, span: Span) -> Ident {
        let mut query_set_builder = self.query_set.borrow_mut();
        let count = query_set_builder
            .names
            .entry(base_name.to_string())
            .or_default();
        let name = if *count != 0 {
            base_name.to_string()
        } else {
            let name = format!("{base_name}{count}");
            *count += 1;
            name
        };
        Ident::new(&name, span)
    }

    pub fn add_query(&self, ty: &TokenStream) -> Ident {
        let mut query_set_builder = self.query_set.borrow_mut();
        let key = ty.to_string();
        query_set_builder
            .querys
            .entry(key)
            .or_insert_with(|| {
                let name = format_ident!("query_{}", convert_type_name(ty));
                let query = quote_spanned!(ty.span()=> #name: Query<#ty>);
                (name, query)
            })
            .0
            .clone()
    }

    pub fn add_item(&self, name: &Ident, item: TokenStream) {
        let mut query_set_builder = self.query_set.borrow_mut();
        query_set_builder.items.insert(name.to_string(), item);
    }
}
