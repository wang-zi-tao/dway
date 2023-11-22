use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::{Block, Expr, ExprAssign, ExprField, ExprReference, Member, Stmt};

use crate::generate::generate_state_change_variable_from_raw;

#[derive(Default, Debug)]
pub struct ParseCodeResult {
    pub use_state: HashMap<String, Span>,
    pub set_state: HashMap<String, Span>,
}
impl ParseCodeResult {
    pub fn add(&mut self, ident: &Ident) {
        self.use_state.insert(ident.to_string(), ident.span());
        self.set_state.insert(ident.to_string(), ident.span());
    }
    pub fn use_state(&mut self, ident: &Ident) {
        self.use_state.insert(ident.to_string(), ident.span());
    }
    pub fn generate_condition(&self) -> TokenStream {
        if self.use_state.is_empty() {
            quote!(true)
        } else {
            let exprs = self
                .use_state
                .iter()
                .map(|(name, span)| generate_state_change_variable_from_raw(name, *span));
            quote!((#(#exprs)&*))
        }
    }
    pub fn is_changed(&self) -> Option<TokenStream> {
        if self.use_state.is_empty() {
            None
        } else {
            let exprs = self.use_state.iter().map(|(name, span)| {
                let changed_ident = format_ident!("is_{}_changed", name, span = *span);
                Some(quote!(state.#changed_ident()))
            });
            Some(quote!((#(#exprs)&&*)))
        }
    }

    pub fn from_expr(expr: &Expr) -> Self {
        let mut this = Self::default();
        parse_expr(expr, &mut this);
        this
    }
}

fn on_parse_field(f: &ExprField, output: &mut ParseCodeResult, is_mut: bool) -> bool {
    if let Expr::Path(p) = &*f.base {
        if p.path.is_ident("state") {
            match &f.member {
                Member::Named(n) => {
                    if is_mut {
                        output.add(n);
                    } else {
                        output.use_state(n);
                    }
                    return true;
                }
                Member::Unnamed(_) => {}
            };
        }
    }
    false
}

fn on_parse_assign(i: &ExprAssign, output: &mut ParseCodeResult, is_mut: bool) -> bool {
    if let Expr::Field(f) = &*i.left {
        return on_parse_field(f, output, is_mut);
    }
    false
}

fn on_parse_reference(i: &ExprReference, output: &mut ParseCodeResult) -> bool {
    if let Expr::Field(f) = &*i.expr {
        return on_parse_field(f, output, i.mutability.is_some());
    }
    false
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
                ) if base_ident.to_string() == "state" && dot_punct.as_char() == '.' => {
                    output.add(member_ident);
                }
                _ => {}
            };
        })
        .for_each(|_| {});
}

pub fn parse_expr(expr: &Expr, output: &mut ParseCodeResult) {
    match expr {
        Expr::Array(e) => e.elems.iter().for_each(|e| parse_expr(e, output)),
        Expr::Assign(i) => {
            parse_expr(&i.right, output);
            if !on_parse_assign(i, output, true) {
                parse_expr(&i.left, output)
            };
        }
        Expr::Async(i) => parse_block(&i.block, output),
        Expr::Await(i) => parse_expr(&i.base, output),
        Expr::Binary(i) => {
            parse_expr(&i.left, output);
            parse_expr(&i.right, output);
        }
        Expr::Block(b) => parse_block(&b.block, output),
        Expr::Break(b) => {
            b.expr.as_ref().map(|e| parse_expr(e, output));
        }
        Expr::Call(i) => {
            parse_expr(&i.func, output);
            i.args.iter().for_each(|e| parse_expr(e, output));
        }
        Expr::Cast(i) => parse_expr(&i.expr, output),
        Expr::Closure(c) => parse_expr(&c.body, output),
        Expr::Const(c) => parse_block(&c.block, output),
        Expr::Continue(_) => {}
        Expr::Field(f) => {
            parse_expr(&f.base, output);
            on_parse_field(f, output, true);
        }
        Expr::ForLoop(i) => {
            parse_expr(&i.expr, output);
            parse_block(&i.body, output);
        }
        Expr::Group(i) => parse_expr(&i.expr, output),
        Expr::If(i) => {
            parse_expr(&i.cond, output);
            parse_block(&i.then_branch, output);
            i.else_branch.as_ref().map(|b| parse_expr(&b.1, output));
        }
        Expr::Index(i) => {
            parse_expr(&i.expr, output);
            parse_expr(&i.index, output);
        }
        Expr::Infer(_) => {}
        Expr::Let(i) => {
            parse_expr(&i.expr, output);
        }
        Expr::Lit(_) => {}
        Expr::Loop(i) => {
            parse_block(&i.body, output);
        }
        Expr::Macro(i) => {
            if i.mac.path.is_ident("state") {
                // output.use_state(&syn::parse2::<Ident>(i.mac.tokens.clone()).unwrap());
            } else {
            }
            parse_expr_tokens(&i.mac.tokens, output);
        }
        Expr::Match(i) => {
            parse_expr(&i.expr, output);
            i.arms.iter().for_each(|arm| {
                arm.guard.as_ref().map(|e| parse_expr(&e.1, output));
                parse_expr(&arm.body, output);
            });
        }
        Expr::MethodCall(c) => {
            parse_expr(&c.receiver, output);
            c.args.iter().for_each(|e| parse_expr(e, output));
        }
        Expr::Paren(i) => parse_expr(&i.expr, output),
        Expr::Path(_) => {}
        Expr::Range(i) => {
            i.start.as_ref().map(|e| parse_expr(&e, output));
            i.end.as_ref().map(|e| parse_expr(&e, output));
        }
        Expr::Reference(i) => {
            if !on_parse_reference(i, output) {
                parse_expr(&i.expr, output);
            };
        }
        Expr::Repeat(i) => {
            parse_expr(&i.expr, output);
            parse_expr(&i.len, output);
        }
        Expr::Return(i) => {
            i.expr.as_ref().map(|e| parse_expr(&e, output));
        }
        Expr::Struct(e) => {
            e.rest.as_ref().map(|e| parse_expr(e, output));
            e.fields.iter().for_each(|f| parse_expr(&f.expr, output));
        }
        Expr::Try(i) => parse_expr(&i.expr, output),
        Expr::TryBlock(b) => parse_block(&b.block, output),
        Expr::Tuple(i) => {
            i.elems.iter().for_each(|e| parse_expr(e, output));
        }
        Expr::Unary(i) => parse_expr(&i.expr, output),
        Expr::Unsafe(i) => parse_block(&i.block, output),
        Expr::Verbatim(i) => parse_expr_tokens(i, output),
        Expr::While(i) => {
            parse_expr(&i.cond, output);
            parse_block(&i.body, output);
        }
        Expr::Yield(i) => {
            i.expr.as_ref().map(|e| parse_expr(&e, output));
        }
        _ => {}
    }
}

fn parse_stmt(stmt: &Stmt, output: &mut ParseCodeResult) {
    match stmt {
        Stmt::Local(l) => {
            if let Some(init) = &l.init {
                parse_expr(&init.expr, output);
            }
        }
        Stmt::Expr(expr, _) => {
            parse_expr(expr, output);
        }
        Stmt::Item(_) => {}
        Stmt::Macro(_) => {}
    }
}

pub fn parse_block(block: &Block, output: &mut ParseCodeResult) {
    for stmt in &block.stmts {
        parse_stmt(stmt, output);
    }
}
