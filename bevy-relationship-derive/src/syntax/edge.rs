use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    bracketed, punctuated::Punctuated, spanned::Spanned, token::Bracket, Expr, Ident, Path, Token,
};

use crate::builder::QueryBuilder;

use super::path::EdgeDirection;

mod kw {
    use syn::custom_keyword;
    custom_keyword!(HasChildren);
}

pub enum EdgeQuery {
    Relationship(Path),
    Hierarchy(kw::HasChildren),
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
                        .flat_map(bevy_relationship::Connectable::iter)
                )
            }
            EdgeQuery::Hierarchy(p) => {
                let span = p.span();
                match direction {
                    EdgeDirection::LeftToRight => {
                        let query_name = builder.add_query(&quote!(&'static Children), None);
                        quote_spanned! {span=>
                            self.#query_name.get(entity).into_iter().flatten().cloned()
                        }
                    }
                    EdgeDirection::RightToLeft => {
                        let query_name = builder.add_query(&quote!(&'static Parent), None);
                        quote_spanned! {span=>
                            self.#query_name.get(entity).into_iter().map(Parent::get)
                        }
                    }
                }
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

    pub fn get_iterator_from_node(
        &self,
        builder: &mut QueryBuilder,
        direction: &EdgeDirection,
    ) -> TokenStream {
        match self {
            EdgeQuery::Relationship(edge) => {
                let span = edge.span();
                let query_name = match direction {
                    EdgeDirection::LeftToRight => builder.add_edge_query(
                        quote!(&'static <#edge as bevy_relationship::Relationship>::From),
                    ),
                    EdgeDirection::RightToLeft => builder.add_edge_query(
                        quote!(&'static <#edge as bevy_relationship::Relationship>::To),
                    ),
                };
                quote_spanned!(span=> bevy_relationship::Connectable::iter(#query_name))
            }
            EdgeQuery::Hierarchy(p) => {
                let span = p.span();
                match direction {
                    EdgeDirection::LeftToRight => {
                        let query_name = builder.add_edge_query(quote!(&'static Children));
                        quote_spanned! {span=> #query_name.into_iter() }
                    }
                    EdgeDirection::RightToLeft => {
                        let query_name = builder.add_edge_query(quote!(&'static Parent));
                        quote_spanned! {span=> #query_name.get() }
                    }
                }
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
                let iters = edeges
                    .iter()
                    .map(|e| e.get_iterator_optional(builder, direction));
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

    pub fn get_iterator_optional(
        &self,
        builder: &mut QueryBuilder,
        direction: &EdgeDirection,
    ) -> TokenStream {
        match self {
            EdgeQuery::Relationship(edge) => {
                let span = edge.span();
                let query_name = match direction {
                    EdgeDirection::LeftToRight => builder.add_edge_query(
                        quote!(Option<&'static <#edge as bevy_relationship::Relationship>::From >),
                    ),
                    EdgeDirection::RightToLeft => builder.add_edge_query(
                        quote!(Option<&'static <#edge as bevy_relationship::Relationship>::To >),
                    ),
                };
                quote_spanned!(span=>
                    #query_name.map(bevy_relationship::Connectable::iter).into_iter().flatten()
                )
            }
            EdgeQuery::Hierarchy(p) => {
                let span = p.span();
                match direction {
                    EdgeDirection::LeftToRight => {
                        let query_name = builder.add_edge_query(quote!(Option<&'static Children>));
                        quote_spanned! {span=> #query_name.into_iter().flatten() }
                    }
                    EdgeDirection::RightToLeft => {
                        let query_name = builder.add_edge_query(quote!(Option<&'static Parent>));
                        quote_spanned! {span=> #query_name.map(bevy::hierarchy::Parent::get).into_iter() }
                    }
                }
            }
            _ => self.get_iterator_from_node(builder, direction),
        }
    }

    pub fn build(&self, builder: &mut QueryBuilder, direction: &EdgeDirection) {
        let inner = std::mem::replace(&mut builder.code, quote!());
        let iter = self.get_iterator_from_node(builder, direction);
        builder.code = quote! {
            let __bevy_relationship_entitys = #iter;
            #inner
        }
    }
}

impl syn::parse::Parse for EdgeQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(kw::HasChildren) {
            Ok(Self::Hierarchy(input.parse()?))
        } else if input.peek(Ident) {
            Ok(Self::Relationship(input.parse()?))
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
