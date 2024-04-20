use derive_syn_parse::Parse;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use std::{cell::RefCell, rc::Rc};
use syn::{punctuated::Punctuated, Token};
use crate::builder::{QueryBuilder, QuerySetBuilder};
use super::path::PathQuery;

#[derive(Parse)]
pub struct GraphQuery {
    pub mut_: Option<Token![mut]>,
    pub name: Ident,
    pub eq: Token![=],
    pub _match: Token![match],
    pub path: PathQuery,
}

impl GraphQuery {
    pub fn build_foreach(
        &self,
        query_set: Rc<RefCell<QuerySetBuilder>>,
        mutable: bool,
        has_begin_node: bool,
    ) -> QueryBuilder {
        let mut query_builder = QueryBuilder::new(query_set, has_begin_node, mutable);
        let span = self.eq.span;

        self.path.build_foreach(&mut query_builder);

        let inner = &query_builder.code;
        let mutable_token = if mutable { quote!(mut) } else { quote!() };
        let func = format_ident!(
            "foreach_{}{}{}",
            self.name,
            (if mutable { "_mut" } else { "" }),
            (if has_begin_node { "_from" } else { "" }),
            span = self.name.span()
        );
        let callback_arg = &query_builder.callback_arg;
        let lifetimes = query_builder.lifetimes.iter().rev();
        let params = query_builder.params.iter().rev();
        let generic_params = query_builder.generic_parameters.iter().rev();
        let begin_node = if has_begin_node {
            Some(quote! {entity: bevy::ecs::entity::Entity,})
        } else {
            None
        };
        let function = quote_spanned! {span=>
            pub fn #func<#(#lifetimes,)* #(#generic_params,)* ReturnType>(&#mutable_token self, #begin_node #(#params,)* #callback_arg) -> Option<ReturnType> {
                loop {
                    #inner
                    break
                }
                None
            }
        };
        query_builder.code = function;
        query_builder
    }
}

#[derive(Parse)]
pub struct GraphQuerySet {
    pub name: Ident,
    _split: Token![=>],
    #[call(Punctuated::parse_terminated)]
    pub querys: Punctuated<GraphQuery, Token![;]>,
}

impl GraphQuerySet {
    pub fn build(&self) -> TokenStream {
        let query_set = Rc::new(RefCell::new(QuerySetBuilder::new(self.name.clone())));
        for query in &self.querys {
            query
                .build_foreach(query_set.clone(), false, false)
                .finish();
            query.build_foreach(query_set.clone(), false, true).finish();
            if query.mut_.is_some() {
                query.build_foreach(query_set.clone(), true, false).finish();
                query.build_foreach(query_set.clone(), true, true).finish();
            }
        }
        let borrow = &query_set.borrow();
        borrow.build()
    }
}
