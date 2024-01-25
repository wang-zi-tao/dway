use crate::{filter::Filter, path::EdgeDirection, query::QueryBuilder};
use quote::{quote, quote_spanned};
use syn::{
    bracketed,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Bracket, Paren},
    Expr, ExprClosure, ExprRange, Ident, Path, Token,
};

pub enum EdgeQuery {
    Relationship(Path),
    Children(Path),
    Expr(Expr),
    Lambda(ExprClosure),
    Many {
        bracket: Bracket,
        edeges: Punctuated<EdgeQuery, Token![|]>,
    },
}

impl EdgeQuery {
    pub fn build(&self, builder: &mut QueryBuilder, direction: &EdgeDirection) {
        let inner = std::mem::replace(&mut builder.code, quote!());
        let iter = match self {
            EdgeQuery::Relationship(edge) => {
                let span = edge.span();
                match direction {
                    EdgeDirection::LeftToRight => {
                        let query_name = builder
                            .add_query(&quote!(<#edge as bevy_relationship::Relationship>::From));
                        quote_spanned!(span=>
                            bevy_relationship::Connectable::iter(
                                self.#query_name.get::<<#edge as bevy_relationship::Relationship>::From>(entity) 
                            )
                        )
                    }
                    EdgeDirection::RightToLeft => {
                        let query_name = builder
                            .add_query(&quote!(<#edge as bevy_relationship::Relationship>::To));
                        quote_spanned!(span=>
                            bevy_relationship::Connectable::iter(
                                self.#query_name.get::<<#edge as bevy_relationship::Relationship>::To>(entity) 
                            )
                        )
                    }
                }
            }
            EdgeQuery::Children(p) => {
                let span = p.span();
                match direction {
                    EdgeDirection::LeftToRight => {
                        let query_name = builder.add_query(&quote!(Children));
                        quote_spanned!(span=> &*self.#query_name.get::<Children>(entity))
                    }
                    EdgeDirection::RightToLeft => {
                        let query_name = builder.add_query(&quote!(Parent));
                        quote_spanned!(span=> &*self.#query_name.get::<Parent>(entity))
                    }
                }
            }
            EdgeQuery::Expr(expr) => quote! {
                #expr
            },
            EdgeQuery::Lambda(closure) => quote! {
                #closure(entity)
            },
            EdgeQuery::Many { bracket, edeges } => todo!(),
        };
        builder.code = quote! {
            for entity in #iter {
                #inner
            }
        }
    }
}

impl syn::parse::Parse for EdgeQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            let path: Path = input.parse()?;
            if path.is_ident("HasChildren") {
                Ok(Self::Children(path))
            } else {
                Ok(Self::Relationship(path))
            }
        } else if input.peek(Bracket) {
            let content;
            Ok(Self::Many {
                bracket: bracketed!(content in input),
                edeges: content.parse_terminated(EdgeQuery::parse, Token![|])?,
            })
        } else if input.peek(Token![|]) {
            Ok(Self::Lambda(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}
