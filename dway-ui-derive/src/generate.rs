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
        |c: char| {
            !(c == '_'
                || c.is_ascii_digit()
                || c.is_ascii_uppercase()
                || c.is_ascii_lowercase())
        },
        "__",
    );

    name.to_case(convert_case::Case::Snake)
}
