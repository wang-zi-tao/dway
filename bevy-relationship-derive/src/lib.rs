use lazy_static::lazy_static;
use std::collections::{BTreeMap, HashMap};

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use regex::Regex;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    token::{Brace, Bracket},
    Token, Type,
};

#[derive(syn_derive::Parse)]
enum NodeQueryKind {
    #[parse(peek = Token!(<))]
    WithFilter {
        _lt: Token!(<),
        ty: Type,
        _comma: Token!(,),
        filter: Type,
        _gt: Token!(>),
    },
    WithoutFilter {
        ty: Type,
    },
}

#[derive(syn_derive::Parse)]
struct NodeQuery {
    name: Ident,
    _assign: Token!(=),
    kind: NodeQueryKind,
}
impl NodeQuery {
    fn ty(&self) -> &Type {
        match &self.kind {
            NodeQueryKind::WithoutFilter { ty, .. } => ty,
            NodeQueryKind::WithFilter { ty, .. } => ty,
        }
    }
    fn query_type(&self) -> TokenStream2 {
        match &self.kind {
            NodeQueryKind::WithoutFilter { ty, .. } => {
                let span = ty.span();
                quote_spanned! { span=>
                    Query<'w, 's, #ty>
                }
            }
            NodeQueryKind::WithFilter { ty, filter, .. } => {
                let span = ty.span();
                quote_spanned! { span=>
                    Query<'w, 's, #ty, #filter>
                }
            }
        }
    }
    fn gen_query(&self) -> TokenStream2 {
        let name = &self.name;
        match &self.kind {
            NodeQueryKind::WithoutFilter { ty, .. } => {
                let span = name.span();
                quote_spanned! {span=>
                    Query<'w, 's, (Entity,#ty)>
                }
            }
            NodeQueryKind::WithFilter { ty, filter, .. } => {
                let span = name.span();
                quote_spanned! {span=>
                    Query<'w, 's, (Entity,#ty),#filter>
                }
            }
        }
    }
}
struct PathQuery {
    name: Ident,
    _assign: Token!(=),
    first_node: Ident,
    edges: Vec<(bool, Type, Ident)>,
}
impl PathQuery {
    pub fn gen_callback(&self, graph: &GraphQuery) -> TokenStream2 {
        let components = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(_direction, _edge, node)| node))
            .map(|node| {
                graph
                    .nodes
                    .get(&node.to_string())
                    .map(|node_query| {
                        let ty = node_query.ty();
                        quote_spanned! {ty.span()=> &bevy::ecs::query::ROQueryItem<#ty> }
                    })
                    .unwrap_or_else(
                        || quote_spanned! {node.span()=> compile_error!("node not found") },
                    )
            });
        quote! { FnMut(#(#components),*) -> bevy_relationship::ControlFlow<R> }
    }
    pub fn gen_callback_mut(&self, graph: &GraphQuery) -> TokenStream2 {
        let components = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(_direction, _edge, node)| node))
            .map(|node| {
                graph
                    .nodes
                    .get(&node.to_string())
                    .map(|node_query| {
                        let ty = node_query.ty();
                        quote_spanned! {ty.span()=> &mut bevy::ecs::query::QueryItem<#ty> }
                    })
                    .unwrap_or_else(
                        || quote_spanned! {node.span()=> compile_error!("node not found") },
                    )
            });
        quote! { FnMut(#(#components),*) -> bevy_relationship::ControlFlow<R> }
    }
    pub fn gen_for_each(&self, graph: &GraphQuery) -> TokenStream2 {
        let callback = self.gen_callback(graph);
        let args = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(_direction, _edge, node)| node))
            .map(|node| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                quote! {&#component_var}
            });
        let inner = self.edges.iter().rev().fold(
            quote! {
                match f(#(#args),*) {
                    bevy_relationship::ControlFlow::Continue => continue,
                    bevy_relationship::ControlFlow::Break => break,
                    bevy_relationship::ControlFlow::Return(r) => return Some(r),
                }
            },
            |inner, (direction, edge, node)| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                let node_var = format_ident!("node_{}", node, span = node.span());
                let peer_field = peer_name(edge, *direction);
                quote_spanned! {node.span()=>
                    if let Ok(peer) = self.#peer_field.get(entity) {
                        for entity in bevy_relationship::Connectable::iter(peer) {
                            if let Ok((_entity,#component_var)) = self.#node_var.get(entity){
                                #inner
                            }
                        }
                    }
                }
            },
        );
        let node = &self.first_node;
        let component_var = format_ident!("item_{}", node, span = node.span());
        let node_var = format_ident!("node_{}", node, span = node.span());
        let span = self.name.span();
        let for_each_method_name = format_ident!("for_each_{}", self.name.to_string(), span = span);
        let for_each_from_method_name =
            format_ident!("for_each_{}_from", self.name.to_string(), span = span);
        quote_spanned! { span=>
            pub fn #for_each_method_name<R>(&self, mut f: impl #callback) -> Option<R> {
                for (entity,#component_var) in self.#node_var.iter() {
                    #inner
                }
                None
            }
            pub fn #for_each_from_method_name<R>(&self, entity: bevy::ecs::entity::Entity, mut f: impl #callback) -> Option<R> {
                if let Ok((_entity,#component_var)) = self.#node_var.get(entity) {
                    #inner
                }
                None
            }
        }
    }
    pub fn gen_for_each_mut(&self, graph: &GraphQuery) -> TokenStream2 {
        let callback = self.gen_callback_mut(graph);
        let args = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(_direction, _edge, node)| node))
            .map(|node| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                quote! {&mut #component_var}
            });
        let inner = self.edges.iter().rev().fold(
            quote! {
                match f(#(#args),*) {
                    bevy_relationship::ControlFlow::Continue => continue,
                    bevy_relationship::ControlFlow::Break => break,
                    bevy_relationship::ControlFlow::Return(r) => return Some(r),
                }
            },
            |inner, (direction, edge, node)| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                let node_var = format_ident!("node_{}", node, span = node.span());
                let peer_field = peer_name(edge, *direction);
                quote_spanned! {node.span()=>
                    if let Ok(peer) = self.#peer_field.get(entity) {
                        for entity in bevy_relationship::Connectable::iter(peer) {
                            if let Ok((_entity,mut #component_var)) = self.#node_var.get_mut(entity){
                                #inner
                            }
                        }
                    }
                }
            },
        );
        let node = &self.first_node;
        let component_var = format_ident!("item_{}", node, span = node.span());
        let node_var = format_ident!("node_{}", node, span = node.span());
        let span = self.name.span();
        let for_each_method_name =
            format_ident!("for_each_{}_mut", self.name.to_string(), span = span);
        let for_each_from_method_name =
            format_ident!("for_each_{}_mut_from", self.name.to_string(), span = span);
        quote_spanned! { span=>
            pub fn #for_each_method_name<R>(&mut self, mut f:impl #callback) -> Option<R> {
                for (entity,mut #component_var) in self.#node_var.iter_mut() {
                    #inner
                }
                None
            }
            pub fn #for_each_from_method_name<R>(&mut self, entity: bevy::ecs::entity::Entity, mut f:impl #callback) -> Option<R> {
                if let Ok((_entity,mut #component_var)) = self.#node_var.get_mut(entity) {
                    #inner
                }
                None
            }
        }
    }
}
impl Parse for PathQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _assign: input.parse()?,
            first_node: input.parse()?,
            edges: {
                let mut edges = Vec::new();
                while input.peek(Token!(<)) || input.peek(Token!(-)) {
                    let edge = if input.peek(Token!(<)) {
                        let _: Token!(<) = input.parse()?;
                        let _: Token!(-) = input.parse()?;
                        let content;
                        bracketed!(content in input);
                        let ty = content.parse()?;
                        let _: Token!(-) = input.parse()?;
                        let name = input.parse()?;
                        (false, ty, name)
                    } else {
                        let _: Token!(-) = input.parse()?;
                        let content;
                        bracketed!(content in input);
                        let ty = content.parse()?;
                        let _: Token!(-) = input.parse()?;
                        let _: Token!(>) = input.parse()?;
                        let name = input.parse()?;
                        (true, ty, name)
                    };
                    edges.push(edge);
                }
                edges
            },
        })
    }
}
fn parse_node_map(input: ParseStream) -> syn::Result<HashMap<String, NodeQuery>> {
    let list = input.parse_terminated(NodeQuery::parse, Token!(,))?;
    Ok(HashMap::from_iter(
        list.into_iter().map(|node| (node.name.to_string(), node)),
    ))
}
fn parse_path_map(input: ParseStream) -> syn::Result<HashMap<String, PathQuery>> {
    let list = input.parse_terminated(PathQuery::parse, Token!(,))?;
    Ok(HashMap::from_iter(
        list.into_iter().map(|node| (node.name.to_string(), node)),
    ))
}
#[derive(syn_derive::Parse)]
struct GraphQuery {
    name: Ident,
    _split: Token!(=>),
    #[syn(bracketed)]
    _bracket: Bracket,
    #[syn(in = _bracket)]
    #[syn(in = _bracket)]
    #[parse(parse_node_map)]
    nodes: HashMap<String, NodeQuery>,
    _split2: Token!(=>),
    #[syn(braced)]
    _brace: Brace,
    #[syn(in = _brace)]
    #[parse(parse_path_map)]
    pathes: HashMap<String, PathQuery>,
}

fn peer_name(peer: &Type, direction: bool) -> Ident {
    lazy_static! {
        static ref RE: Regex = Regex::new("[^a-zA-Z0-9_]").unwrap();
    }
    format_ident!(
        "peer_{}{}",
        RE.replace(&peer.into_token_stream().to_string(), "_"),
        if direction { "" } else { "_rev" },
        span = peer.span()
    )
}

#[proc_macro]
pub fn graph_query(input: TokenStream) -> TokenStream {
    let graph_query = parse_macro_input!(input as GraphQuery);
    let mut peers = BTreeMap::new();
    for (_path_name, path) in graph_query.pathes.iter() {
        let _last_node = path.first_node.to_string();
        for (direction, edge, _node) in path.edges.iter() {
            let peer_filed = peer_name(edge, *direction);
            peers.insert(
                peer_filed.to_string(),
                if *direction {
                    quote_spanned! { edge.span()=>
                        #[allow(non_snake_case)]
                        pub #peer_filed: Query<'w, 's, &'static <#edge as bevy_relationship::Relationship>::From>,
                    }
                } else {
                    quote_spanned! { edge.span()=>
                        #[allow(non_snake_case)]
                        pub #peer_filed: Query<'w, 's, &'static <#edge as bevy_relationship::Relationship>::To>,
                    }
                },
            );
        }
    }
    let name = &graph_query.name;
    let nodes = graph_query.nodes.values().map(|node| {
        let name = node.name.clone();
        let field_name = format_ident!("node_{}", name.to_string(), span = name.span());
        let param = node.gen_query();
        let span = name.span();
        quote_spanned!(span=> 
            #[allow(non_snake_case)]
            pub #field_name: #param,)
    });
    let span = graph_query._bracket.span;
    let for_each_function = graph_query
        .pathes
        .values()
        .map(|path| path.gen_for_each(&graph_query));
    let for_each_mut_function = graph_query
        .pathes
        .values()
        .map(|path| path.gen_for_each_mut(&graph_query));
    let peers_values = peers.values();
    let output = quote_spanned! {span=>
        #[allow(non_snake_case)]
        #[derive(bevy::ecs::system::SystemParam)]
        pub struct #name<'w, 's> {
            #(#nodes)*
            #(#peers_values)*
        }
        #[allow(dead_code)]
        #[allow(non_snake_case)]
        impl<'w, 's> #name<'w, 's>{
            #(#for_each_function)*
            #(#for_each_mut_function)*
        }
    };
    // panic!("{}", output);
    output.into()
}
