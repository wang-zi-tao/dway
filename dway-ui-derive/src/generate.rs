use convert_case::Casing;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Type;

#[derive(Clone)]
pub enum BoolExpr {
    False,
    True,
    RuntimeValue(TokenStream),
}
impl BoolExpr {
    pub fn optional(tokens: Option<TokenStream>, default: bool) -> Self {
        if let Some(tokens) = tokens {
            Self::RuntimeValue(tokens)
        } else if default {
            Self::True
        } else {
            Self::False
        }
    }
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
            BoolExpr::RuntimeValue(c) => Self::RuntimeValue(quote!(!(#c))),
        }
    }
    pub fn optional_token_stream(self) -> Option<TokenStream> {
        match self {
            BoolExpr::False => None,
            BoolExpr::True => Some(quote!(true)),
            BoolExpr::RuntimeValue(c) => Some(c),
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

pub fn convert_type_name(ty: &Type) -> String {
    let name = ty.to_token_stream().to_string();
    let name = name.replace('_', "__");
    let name = name.replace(
        |char| {
            !(char == '_'
                || ('0'..='9').contains(&char)
                || ('A'..='Z').contains(&char)
                || ('a'..='z').contains(&char))
        },
        "__",
    );
    
    name.to_case(convert_case::Case::Snake)
}
