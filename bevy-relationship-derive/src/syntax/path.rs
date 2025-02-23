use derive_syn_parse::Parse;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    spanned::Spanned,
    token::{Bracket, Paren},
    ExprRange, Token, Type,
};

use crate::builder::{NodeInfo, QueryBuilder};

use super::{edge::EdgeQuery, filter::Filter, node::NodeQuery};

mod kw {
    use syn::custom_keyword;
    custom_keyword!(filter);
}

#[derive(Clone, Copy)]
pub enum EdgeDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Parse)]
pub struct PathEdgeQuery {
    #[bracket]
    pub _wrap: Bracket,
    #[inside(_wrap)]
    pub edge: EdgeQuery,
    #[inside(_wrap)]
    pub where_: Option<Token![where]>,
    #[inside(_wrap)]
    #[parse_if(where_.is_some())]
    pub filter: Option<Filter>,
    #[inside(_wrap)]
    pub multi: Option<Token![*]>,
    #[inside(_wrap)]
    #[parse_if(multi.is_some())]
    pub range: Option<ExprRange>,
}

impl PathEdgeQuery {
    pub fn build(&self, builder: &mut QueryBuilder, direction: &EdgeDirection) {
        if let Some(range) = &self.range {
            let span = self.multi.span();
            let inner = std::mem::replace(&mut builder.code, quote!());
            let iterator = self.edge.get_iterator(builder, direction);
            let code = quote_spanned!{span=>
                let __bevy_relationship_range: std::ops::Range<usize> = #range; 
                let mut __bevy_relationship_entity_set: bevy::ecs::entity::EntityHashMap<usize> = Default::default(); 
                let mut __bevy_relationship_entity_stack: Vec<(bevy::ecs::entity::Entity, usize)> = vec![(entity, 0)]; 
                __bevy_relationship_entity_set.insert(entity, 0);

                while let Some((entity, level)) = __bevy_relationship_entity_stack.pop() {
                    if level < __bevy_relationship_range.end {
                        for peer_entity in #iterator {
                            let peer_level = __bevy_relationship_entity_set.entry(peer_entity).or_insert(usize::MAX);
                            if level + 1 < __bevy_relationship_range.end && level + 1 < *peer_level {
                                __bevy_relationship_entity_stack.push((peer_entity, level + 1));
                            }
                            *peer_level = (*peer_level).min(level + 1);
                        }
                    }
                }

                let __bevy_relationship_entitys = __bevy_relationship_entity_set
                    .into_iter()
                    .filter(|e|e.1 >= __bevy_relationship_range.start)
                    .map(|e|e.0);
                #inner
            };
            builder.code = code;
        } else {
            if let Some(filter) = &self.filter {
                let name = format_ident!(
                    "edge{}",
                    builder.node_stack.len(),
                    span = self.where_.span()
                );
                filter.build_modify_iter(
                    builder,
                    &name,
                    quote!(entity),
                    quote!(bevy::ecs::entity::Entity),
                );
            }
            self.edge.build(builder, direction);
        }
    }
}

#[derive(Parse)]
pub struct PathNodeQuery {
    #[paren]
    pub _wrap: Paren,
    #[inside(_wrap)]
    pub name: Ident,
    #[inside(_wrap)]
    pub _col: Token![:],
    #[inside(_wrap)]
    pub node: NodeQuery,
    #[inside(_wrap)]
    pub _filter: Option<kw::filter>,
    #[inside(_wrap)]
    #[parse_if(_filter.is_some())]
    pub query_filter: Option<Type>,
    #[inside(_wrap)]
    pub _where: Option<Token![where]>,
    #[inside(_wrap)]
    #[parse_if(_where.is_some())]
    pub filter: Option<Filter>,
}

impl PathNodeQuery {
    pub fn build(&self, builder: &mut QueryBuilder) {
        let name = &self.name;
        if let Some(filter) = &self.filter {
            let ty = self.node.to_item_type(builder, name);
            let arg = if builder.mutable {
                quote!(&mut #name)
            } else {
                quote!(&#name)
            };
            filter.build(builder, name, arg, ty);
        }
        self.node.build(builder, name, self.query_filter.as_ref());
    }
}

pub struct PathQuery {
    pub nodes: Vec<PathNodeQuery>,
    pub edges: Vec<(PathEdgeQuery, EdgeDirection)>,
}

impl PathQuery {
    fn push_node(node_query: &PathNodeQuery, builder: &mut QueryBuilder) {
        let name = &node_query.name;
        let ty = node_query.node.to_item_type(builder, name);
        builder.node_stack.push(NodeInfo {
            name: name.clone(),
            callback_arg: if builder.mutable {
                quote!(&mut #name)
            } else {
                quote!(&#name)
            },
            callback_type: ty,
            extract_querys: vec![],
        });
    }
    pub fn build_foreach_changed(&self, builder: &mut QueryBuilder) {
        let entitys: Vec<TokenStream> = vec![];
        let querys: Vec<TokenStream> = vec![];
        let mut_flags: Vec<TokenStream> = vec![];
        let get_methods: Vec<TokenStream> = vec![];
        let args: Vec<TokenStream> = vec![];
        let code = quote!{
            let __bevy_relationship__lambda = |this: &Self,#(#entitys: bevy::ecs::entity::Entity),*| {
                #(let Ok(#args) = #querys.get_methods(#entitys);)*
                callback(#(&args));
            }
        };
    }

    pub fn build_foreach(&self, builder: &mut QueryBuilder) {
        let node0 = &self.nodes[0];
        Self::push_node(node0, builder);

        self.build_foreach_inner(builder, &self.nodes[1..], &self.edges);

        let node0 = &self.nodes[0];
        node0.build(builder);
        builder.node_stack.pop();
    }
    fn build_foreach_inner(
        &self,
        builder: &mut QueryBuilder,
        nodes: &[PathNodeQuery],
        edges: &[(PathEdgeQuery, EdgeDirection)],
    ) {
        if !nodes.is_empty() && !edges.is_empty() {
            Self::push_node(&nodes[0], builder);

            self.build_foreach_inner(builder, &nodes[1..], &edges[1..]);

            let node = &nodes[0];
            let (edge, edge_direction) = edges.last().unwrap();
            node.build(builder);
            builder.node_stack.pop();

            edge.build(builder, edge_direction);
        } else {
            let inner = std::mem::replace(&mut builder.code, quote!());
            let arg = builder.node_stack.iter().map(|x| &x.callback_arg);
            let ty = builder.node_stack.iter().map(|x| &x.callback_type);
            builder.callback_arg = quote! {
                mut callback: impl FnMut(#(#ty),*) -> bevy_relationship::ControlFlow<ReturnType>
            };
            let code = quote! {
                #inner
                match callback(#(#arg),*) {
                    bevy_relationship::ControlFlow::Continue => continue,
                    bevy_relationship::ControlFlow::Break => break,
                    bevy_relationship::ControlFlow::Return(r) => return Some(r),
                };
            };
            builder.code = code;
        }
    }
}

impl syn::parse::Parse for PathQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut nodes = vec![input.parse()?];
        let mut edges = vec![];
        while input.peek(Token![-]) || input.peek(Token![<-]) {
            let direction = if input.peek(Token![-]) {
                let _: Token![-] = input.parse()?;
                EdgeDirection::LeftToRight
            } else {
                let _: Token![<-] = input.parse()?;
                EdgeDirection::RightToLeft
            };
            let edge = input.parse()?;
            match direction {
                EdgeDirection::LeftToRight => {
                    let _: Token![->] = input.parse()?;
                }
                EdgeDirection::RightToLeft => {
                    let _: Token![-] = input.parse()?;
                }
            }
            let node = input.parse()?;
            edges.push((edge, direction));
            nodes.push(node);
        }
        Ok(Self { nodes, edges })
    }
}
