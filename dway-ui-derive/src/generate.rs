use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};

pub fn generate_despawn(entity: TokenStream) -> TokenStream {
    quote! {
        if commands.get_entity(#entity).is_some() {
            commands
                .entity(#entity)
                .despawn_recursive();
        }
    }
}
pub fn generate_state_change_variable_from_raw(name: &str, span: Span) -> Ident {
    format_ident!("state_changed_{}", name, span = span)
}

pub fn generate_state_change_variable(ident: &Ident) -> Ident {
    generate_state_change_variable_from_raw(&ident.to_string(), ident.span())
}

#[derive(Clone)]
pub enum BoolExpr {
    False,
    True,
    RuntimeValue(TokenStream),
}
impl BoolExpr {
    pub fn map(&self, t: impl ToTokens, f: impl ToTokens) -> TokenStream {
        match self {
            BoolExpr::False => quote!(#f),
            BoolExpr::True => quote!(#t),
            BoolExpr::RuntimeValue(c) => quote! {
                if #c { #t } else { #f }
            },
        }
    }
    pub fn to_if_else(&self, t: impl ToTokens, f: Option<&impl ToTokens>) -> Option<TokenStream> {
        match self {
            BoolExpr::False => None,
            BoolExpr::True => Some(quote!(#t)),
            BoolExpr::RuntimeValue(c) => Some(
                f.map(|f| {
                    quote! {
                        if #c { #t } else { #f }
                    }
                })
                .unwrap_or_else(|| {
                    quote! {
                        if #c { #t }
                    }
                }),
            ),
        }
    }
    pub fn not(&self) -> Self {
        match self {
            BoolExpr::False => Self::True,
            BoolExpr::True => Self::False,
            BoolExpr::RuntimeValue(c) => Self::RuntimeValue(quote!(!(c))),
        }
    }
}
impl ToTokens for BoolExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let bool = match self {
            BoolExpr::False => quote!(false),
            BoolExpr::True => quote!(true),
            BoolExpr::RuntimeValue(c) => c.clone(),
        };
        tokens.extend(bool);
    }
}
