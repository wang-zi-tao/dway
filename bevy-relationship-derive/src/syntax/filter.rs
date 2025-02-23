use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{parse::Parse, spanned::Spanned, Expr, Token};
use crate::builder::QueryBuilder;

pub enum Filter {
    Expr(Expr),
    Lambda(Token![?]),
}

impl Filter {
    pub fn span(&self) -> Span {
        match self {
            Filter::Expr(e) => e.span(),
            Filter::Lambda(t) => t.span,
        }
    }
    pub fn get_filter_expr(
        &self,
        builder: &mut QueryBuilder,
        name: &syn::Ident,
        arg: TokenStream,
        ty: TokenStream,
    ) -> TokenStream {
        
        match self {
            Filter::Expr(e) => quote!(#e),
            Filter::Lambda(t) => {
                let lambda_name =
                    builder.alloc_name(&format!("{}_filter", name), t.span);
                builder.add_param(quote_spanned! {t.span=>
                    mut #lambda_name: impl FnMut(#ty) -> bool
                });
                quote_spanned! {t.span=>
                    #lambda_name(#arg)
                }
            }
        }
    }
    pub fn build_modify_iter(
        &self,
        builder: &mut QueryBuilder,
        name: &syn::Ident,
        arg: TokenStream,
        ty: TokenStream,
    ) {
        let code = std::mem::replace(&mut builder.code, quote!());
        let filter_result = self.get_filter_expr(builder, name, arg, ty);
        builder.code = quote_spanned! {self.span()=>
            let __bevy_relationship_entitys = __bevy_relationship_entitys.into_iter().filter(|&entity|#filter_result);
            #code
        };
    }
    pub fn build(
        &self,
        builder: &mut QueryBuilder,
        name: &syn::Ident,
        arg: TokenStream,
        ty: TokenStream,
    ) {
        let code = std::mem::replace(&mut builder.code, quote!());
        let filter_result = self.get_filter_expr(builder, name, arg, ty);
        let code = quote_spanned! {self.span()=>
            if #filter_result {
                #code
            }
        };
        builder.code = code;
    }
}

impl Parse for Filter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![?]) {
            Ok(Filter::Lambda(input.parse()?))
        } else {
            Ok(Filter::Expr(input.parse()?))
        }
    }
}
