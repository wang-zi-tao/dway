use derive_syn_parse::Parse;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use std::collections::HashMap;
use syn::{
    parse_quote, visit::Visit, Block, Expr, ExprField, Macro, Member,
    Stmt,
};

use crate::generate::BoolExpr;

#[derive(Default, Debug)]
pub struct ParseCodeResult {
    pub use_state: HashMap<String, Span>,
    pub use_prop: HashMap<String, Span>,
    pub set_state: HashMap<String, Span>,
}
impl Visit<'_> for ParseCodeResult {
    fn visit_macro(&mut self, i: &'_ Macro) {
        if i.path.is_ident("state") {
            self.use_state(&syn::parse2::<Ident>(i.tokens.clone()).unwrap());
        } else if i.path.is_ident("prop") {
            self.use_prop(&syn::parse2::<Ident>(i.tokens.clone()).unwrap());
        } else {
            parse_expr_tokens(&i.tokens, self);
        }
        syn::visit::visit_macro(self, i);
    }
    fn visit_expr_field(&mut self, i: &'_ ExprField) {
        let Expr::Path(var) = &*i.base else {
            syn::visit::visit_expr_field(self, i);
            return;
        };
        let Member::Named(member) = &i.member else {
            syn::visit::visit_expr_field(self, i);
            return;
        };
        match &*var
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default()
        {
            "prop" => {
                self.use_prop(member);
            }
            "state" => {
                self.use_state(member);
            }
            _ => {}
        }
        syn::visit::visit_expr_field(self, i);
    }
    fn visit_expr_method_call(&mut self, i: &'_ syn::ExprMethodCall) {
        if let Expr::Path(var) = &*i.receiver {
            match &*var
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default()
            {
                "prop" => {
                    self.use_prop(&i.method);
                }
                "state" => {
                    self.use_state(&i.method);
                    let method_name = i.method.to_string();
                    if method_name.ends_with("_mut") {
                        if let Some((field, _)) = method_name.rsplit_once("_mut") {
                            self.use_state(&format_ident!("{}", field));
                        };
                    }
                }
                _ => {}
            }
        };
        syn::visit::visit_expr_method_call(self, i);
    }
}

impl ParseCodeResult {
    pub fn add_state(&mut self, ident: &Ident) {
        self.use_state.insert(ident.to_string(), ident.span());
        self.set_state.insert(ident.to_string(), ident.span());
    }
    pub fn use_prop(&mut self, ident: &Ident) {
        self.use_prop.insert(ident.to_string(), ident.span());
    }
    pub fn use_state(&mut self, ident: &Ident) {
        self.use_state.insert(ident.to_string(), ident.span());
    }
    pub fn changed_bool(&self) -> BoolExpr {
        if self.use_state.is_empty() && self.use_prop.is_empty() {
            BoolExpr::False
        } else {
            let exprs = self
                .use_state
                .iter()
                .map(|(name, span)| {
                    let method_name = format_ident!("{}_is_changed", name, span = *span);
                    quote_spanned!(*span=>state.#method_name())
                })
                .chain(
                    (!self.use_prop.is_empty())
                        .then_some(quote!(__dway_prop_changed)),
                );
            BoolExpr::RuntimeValue(quote!((#(#exprs)&&*)))
        }
    }
    pub fn is_changed(&self) -> Option<TokenStream> {
        self.changed_bool().optional_token_stream()
    }

    pub fn from_expr(expr: &Expr) -> Self {
        let mut this = Self::default();
        this.visit_expr(expr);
        this
    }
}

fn parse_expr_tokens(tokens: &TokenStream, output: &mut ParseCodeResult) {
    tokens.clone().into_iter().for_each(|token| {
        match &token {
            TokenTree::Group(g) => {
                parse_expr_tokens(&g.stream(), output);
            }
            _ => {}
        };
    });
    tokens
        .clone()
        .into_iter()
        .map_windows(|[base, dot, member]| {
            match (base, dot, member) {
                (
                    TokenTree::Ident(base_ident),
                    TokenTree::Punct(dot_punct),
                    TokenTree::Ident(member_ident),
                ) if *base_ident == "state" && dot_punct.as_char() == '.' => {
                    output.add_state(member_ident);
                }
                (
                    TokenTree::Ident(base_ident),
                    TokenTree::Punct(dot_punct),
                    TokenTree::Ident(member_ident),
                ) if *base_ident == "prop" && dot_punct.as_char() == '.' => {
                    output.use_prop(member_ident);
                }
                _ => {}
            };
        })
        .for_each(|_| {});
}

#[derive(Parse)]
struct Stmts {
    #[call(Block::parse_within)]
    pub _stmts: Vec<Stmt>,
}
pub fn check_stmts(input: TokenStream) -> TokenStream {
    let _: Stmts = parse_quote!(input);
    input
}
