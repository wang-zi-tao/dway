use derive_syn_parse::Parse;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{At, Brace, If, Paren, RArrow},
    *,
};

use crate::{parse_expr, style, ParseCodeResult};

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DomArgKey {
    Key,
    Component(String),
    Id,
    Style,
    State(String),
    Query(String),
    For,
    If,
}

pub enum DomArg {
    Component {
        component: Type,
        _eq: Token![=],
        _wrap: Paren,
        expr: Expr,
    },
    If {
        _at: Token![@],
        _if: Token![if],
        _wrap: Paren,
        expr: Expr,
    },
    Id {
        _at: Token![@],
        _opcode: Ident,
        _eq: Token![=],
        id: LitStr,
    },
    Style {
        _at: Token![@],
        _opcode: Ident,
        _eq: Token![=],
        style: LitStr,
    },
    For {
        _at: Token![@],
        _opcode: Token![for],
        _paren: Paren,
        pat: syn::Pat,
        _in: Token![in],
        expr: Expr,
    },
    Key {
        _at: Token![@],
        _opcode: Ident,
        _paren: Paren,
        expr: Expr,
        _as: Token![:],
        ty: Type,
    },
    State {
        _at: Token![@],
        _opcode: Ident,
        _paren: Paren,
        name: Ident,
        _as: Token![:],
        ty: Type,
        _eq: Token![=],
        expr: Expr,
    },
    Query {
        _at: Token![@],
        _opcode: Ident,
        _paren: Paren,
        name: Ident,
        _as: Token![:],
        ty: Type,
    },
}

impl DomArg {
    pub fn key(&self) -> DomArgKey {
        match &self {
            DomArg::Component { component, .. } => {
                DomArgKey::Component(quote!(#component).to_string())
            }
            DomArg::If { .. } => DomArgKey::If,
            DomArg::Id { .. } => DomArgKey::Id,
            DomArg::Style { .. } => DomArgKey::Style,
            DomArg::For { .. } => DomArgKey::For,
            DomArg::Key { .. } => DomArgKey::Key,
            DomArg::State { name, .. } => DomArgKey::State(name.to_string()),
            DomArg::Query { name, .. } => DomArgKey::State(name.to_string()),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            DomArg::Component { component, .. } => component.span(),
            DomArg::If { _if, .. } => _if.span(),
            DomArg::Id { id, .. } => id.span(),
            DomArg::Style { style, .. } => style.span(),
            DomArg::For { _opcode, .. } => _opcode.span(),
            DomArg::Key { _opcode, .. } => _opcode.span(),
            DomArg::State { _opcode, .. } => _opcode.span(),
            DomArg::Query { name, .. } => name.span(),
        }
    }

    pub fn simplify(self) -> syn::Result<Self> {
        match self {
            DomArg::Style { style, .. } => {
                let value_tokens = crate::style::generate(&style);
                let expr_tokens = quote_spanned!(style.span()=>Style=(#value_tokens));
                Ok(syn::parse2(expr_tokens)?)
            }
            o => Ok(o),
        }
    }

    pub fn parse_map(input: ParseStream) -> syn::Result<BTreeMap<DomArgKey, Self>> {
        let mut map = BTreeMap::default();
        if input.peek(Token![@]) || input.peek(Ident) {
            let arg: Self = input.parse()?;
            let arg = arg.simplify()?;
            let key = arg.key();
            map.insert(key, arg);
        }
        Ok(map)
    }

    pub fn get_component_expr(&self) -> Option<TokenStream> {
        match self {
            DomArg::Component {
                component, expr, ..
            } => Some(quote!(#expr as #component)),
            _ => None,
        }
    }

    pub fn wrap_for_spawn(&self, inner: TokenStream) -> TokenStream {
        match self {
            Self::If { expr, .. } => {
                quote! {
                    if #expr {
                         #inner
                    }
                }
            }
            Self::For { pat, expr, .. } => {
                quote! {
                    for #pat in #expr{
                         #inner
                    }
                }
            }
            _ => inner,
        }
    }

    pub fn need_node_entity(&self) -> bool {
        match self {
            Self::If { .. } => true,
            Self::For { .. } => true,
            Self::Id { .. } => true,
            Self::Component { expr, .. } => {
                let component_state = ParseCodeResult::from_expr(expr);
                component_state.use_state.is_empty() && component_state.set_state.is_empty()
            }
            _ => false,
        }
    }

    pub fn generate_update(&self, entity: &Ident) -> Option<TokenStream> {
        match self {
            Self::Component { expr, .. } => {
                let dependencies = ParseCodeResult::from_expr(expr);
                dependencies.is_changed().map(|check_changed| {
                    quote! {
                        if #check_changed {
                             commands.entity(#entity).insert(#expr);
                        }
                    }
                })
            }
            _ => None,
        }
    }
}
impl syn::parse::Parse for DomArg {
    fn parse(input: ParseStream) -> Result<Self> {
        if !input.peek(Token![@]) {
            let content;
            Ok(Self::Component {
                component: input.parse()?,
                _eq: input.parse()?,
                _wrap: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else {
            let at = input.parse()?;
            if input.peek(Token![if]) {
                let content;
                Ok(Self::If {
                    _at: at,
                    _if: input.parse()?,
                    _wrap: parenthesized!(content in input),
                    expr: content.parse()?,
                })
            } else if input.peek(Token![for]) {
                let content;
                Ok(Self::For {
                    _at: at,
                    _opcode: input.parse()?,
                    _paren: parenthesized!(content in input),
                    pat: Pat::parse_multi(&content)?,
                    _in: content.parse()?,
                    expr: content.parse()?,
                })
            } else {
                let instruction: Ident = input.parse()?;
                match &*instruction.to_string() {
                    "key" => {
                        let content;
                        Ok(Self::Key {
                            _at: at,
                            _opcode: instruction,
                            _paren: parenthesized!(content in input),
                            expr: content.parse()?,
                            _as: content.parse()?,
                            ty: content.parse()?,
                        })
                    }
                    "state" => {
                        let content;
                        Ok(Self::State {
                            _at: at,
                            _opcode: input.parse()?,
                            _paren: parenthesized!(content in input),
                            name: content.parse()?,
                            _as: content.parse()?,
                            ty: content.parse()?,
                            _eq: content.parse()?,
                            expr: content.parse()?,
                        })
                    }
                    "query" => {
                        let content;
                        Ok(Self::Query {
                            _at: at,
                            _opcode: input.parse()?,
                            _paren: parenthesized!(content in input),
                            name: content.parse()?,
                            _as: content.parse()?,
                            ty: content.parse()?,
                        })
                    }
                    "id" => Ok(Self::Id {
                        _at: at,
                        _opcode: instruction,
                        _eq: input.parse()?,
                        id: input.parse()?,
                    }),
                    "style" => Ok(Self::Style {
                        _at: at,
                        _opcode: instruction,
                        _eq: input.parse()?,
                        style: input.parse()?,
                    }),
                    other => {
                        panic!(
                            "unsupported instruction: {other}, known instructions: {:?}",
                            ["if", "for", "key", "style", "id"]
                        );
                    }
                }
            }
        }
    }
}
