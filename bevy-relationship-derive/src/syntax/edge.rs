use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    bracketed, punctuated::Punctuated, spanned::Spanned, token::Bracket, Expr, Ident,
    Path, Token,
};

use crate::builder::QueryBuilder;

use super::path::EdgeDirection;

pub enum EdgeQuery {
    Relationship(Path),
    Children(Path),
    Expr(Expr),
    Lambda(Token![?]),
    Many {
        bracket: Bracket,
        edeges: Punctuated<EdgeQuery, Token![|]>,
    },
}

impl EdgeQuery {
    pub fn get_iterator(
        &self,
        builder: &mut QueryBuilder,
        direction: &EdgeDirection,
    ) -> TokenStream {
        match self {
            EdgeQuery::Relationship(edge) => {
                let span = edge.span();
                let query_name = match direction {
                    EdgeDirection::LeftToRight => builder.add_query(
                        &quote!(&'static <#edge as bevy_relationship::Relationship>::From),
                        None,
                    ),
                    EdgeDirection::RightToLeft => builder.add_query(
                        &quote!(&'static <#edge as bevy_relationship::Relationship>::To),
                        None,
                    ),
                };
                quote_spanned!(span=>
                    self.#query_name.get(entity).into_iter()
                        .map(bevy_relationship::Connectable::iter).flatten()
                )
            }
            EdgeQuery::Children(p) => {
                let span = p.span();
                let query_name = match direction {
                    EdgeDirection::LeftToRight => {
                        builder.add_query(&quote!(&'static Children), None)
                    }
                    EdgeDirection::RightToLeft => builder.add_query(&quote!(&'static Parent), None),
                };
                quote_spanned!(span=>
                    self.#query_name.get(entity).into_iter()
                        .map(bevy_relationship::Connectable::iter).flatten()
                )
            }
            EdgeQuery::Expr(expr) => quote! {
                (#expr)
            },
            EdgeQuery::Lambda(t) => {
                let node = builder.node_stack.last().unwrap();
                let name = &node.name;
                let ty = &node.callback_type;
                let arg = node.callback_arg.clone();
                let lambda_name = builder.alloc_name(
                    &format!(
                        "{}_edge",
                        name.to_string().to_case(convert_case::Case::Snake)
                    ),
                    t.span,
                );
                let iterator_name = builder.alloc_name(
                    &format!(
                        "{}EdgeIterator",
                        name.to_string().to_case(convert_case::Case::Pascal)
                    ),
                    t.span,
                );
                let lambda_var = quote_spanned! {t.span=>
                    mut #lambda_name: impl FnMut(#ty) -> #iterator_name
                };
                builder.add_generic_parameter(quote_spanned!(t.span=>
                    #iterator_name: IntoIterator<Item=Entity>
                ));
                builder.add_param(lambda_var);
                quote_spanned! {t.span=>
                    #lambda_name(#arg)
                }
            }
            EdgeQuery::Many { bracket, edeges } => {
                let iters = edeges.iter().map(|e| e.get_iterator(builder, direction));
                quote_spanned! {bracket.span=>
                    {
                        let mut entitys = bevy::ecs::entity::EntityHashSet::default();
                        #(entitys.extend(#iters);)*
                        entitys
                    }
                }
            }
        }
    }
    pub fn build(&self, builder: &mut QueryBuilder, direction: &EdgeDirection) {
        let inner = std::mem::replace(&mut builder.code, quote!());
        let iter = self.get_iterator(builder, direction);
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
        } else if input.peek(Token![?]) {
            Ok(Self::Lambda(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}
