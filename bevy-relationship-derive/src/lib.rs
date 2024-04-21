mod base;
mod builder;
mod create;
mod insert;
mod model;
mod syntax;
mod update;
mod v1;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident};
use syntax::query::GraphQuerySet;

#[proc_macro]
pub fn graph_query(input: TokenStream) -> TokenStream {
    v1::graph_query(input)
}

#[proc_macro]
pub fn graph_query2(input: TokenStream) -> TokenStream {
    let graph_query = parse_macro_input!(input as GraphQuerySet);
    graph_query.build().into()
}

#[proc_macro]
pub fn right_of(input: TokenStream) -> TokenStream { 
    let name = parse_macro_input!(input as Ident);
    let ident = format_ident!("edge_{}", name, span=name.span());
    quote!(#ident).into()
}
