pub mod spawn_context;
pub mod widget_context;

use crate::{
    dom::Dom,
    domarg::{DomArg, DomArgKey},
};
use derive_syn_parse::Parse;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{At, Brace, Paren, RArrow},
    *,
};

#[derive(Default)]
pub struct Context {
    pub system_querys: BTreeMap<String, TokenStream>,
    pub components: BTreeMap<String, TokenStream>,
    pub systems: BTreeMap<String, TokenStream>,
}

pub struct DomContext<'l> {
    pub context: &'l Context,
    pub root: &'l Dom,
    pub world_query: BTreeMap<String, TokenStream>,
    pub dom_list: Vec<&'l Dom>,
}

impl<'l> DomContext<'l> {
    pub fn new(context: &'l Context, root: &'l Dom) -> Self {
        Self {
            context,
            root,
            world_query: Default::default(),
            dom_list: Default::default(),
        }
    }

    pub fn get_dom_id(&mut self, dom: &'l Dom, upper_case: bool) -> Ident {
        self.dom_list.push(dom);
        if let Some(DomArg::Id { id: lit, .. }) = dom.args.get(&DomArgKey::Id) {
            format_ident!("{}", lit.value(), span = lit.span())
        } else {
            if upper_case {
                format_ident!("N{}", self.dom_list.len(), span = dom.span())
            } else {
                format_ident!("n{}", self.dom_list.len(), span = dom.span())
            }
        }
    }
    pub fn wrap_dom_id(prefix: &str, ident: &Ident, suffix: &str) -> Ident {
        format_ident!("{}{}{}", prefix, ident, suffix, span = ident.span())
    }
}
