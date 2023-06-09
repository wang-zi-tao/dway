use lazy_static::lazy_static;
use std::collections::{BTreeMap, HashMap};

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use regex::Regex;
use syn::{
    braced, bracketed,
    parse::Parse,
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Bracket, Paren},
    Token, Type,
};

enum NodeQuery {
    WithoutFilter {
        name: Ident,
        _assign: Token!(=),
        ty: Type,
        connections: Vec<Type>,
    },
    WithFilter {
        name: Ident,
        _assign: Token!(=),
        _lt: Token!(<),
        ty: Type,
        _comma: Token!(,),
        _gt: Token!(>),
        filter: Type,
        connections: Vec<Type>,
    },
}
impl NodeQuery {
    fn name(&self) -> &Ident {
        match self {
            NodeQuery::WithoutFilter { name, .. } => &name,
            NodeQuery::WithFilter { name, .. } => &name,
        }
    }
    fn ty(&self) -> &Type {
        match self {
            NodeQuery::WithoutFilter { ty, .. } => &ty,
            NodeQuery::WithFilter { ty, .. } => &ty,
        }
    }
    fn query_type(&self) -> TokenStream2 {
        match self {
            NodeQuery::WithoutFilter { ty, .. } => {
                let span = ty.span();
                quote_spanned! { span=>
                    Query<'w, 's, #ty>
                }
            }
            NodeQuery::WithFilter { ty, filter, .. } => {
                let span = ty.span();
                quote_spanned! { span=>
                    Query<'w, 's, #ty, #filter>
                }
            }
        }
    }
    fn gen_query(&self) -> TokenStream2 {
        match self {
            NodeQuery::WithoutFilter { name, ty, .. } => {
                let span = name.span();
                quote_spanned! {span=>
                    Query<'w, 's, (Entity,#ty)>
                }
            }
            NodeQuery::WithFilter {
                name, ty, filter, ..
            } => {
                let span = name.span();
                quote_spanned! {span=>
                    Query<'w, 's, (Entity,#ty),#filter>
                }
            }
        }
    }
}
impl Parse for NodeQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let assign = input.parse()?;
        if input.peek(Token!(<)) {
            Ok(Self::WithFilter {
                name,
                _assign: assign,
                _lt: input.parse()?,
                ty: input.parse()?,
                _comma: input.parse()?,
                filter: input.parse()?,
                _gt: input.parse()?,
                connections: Default::default(),
            })
        } else {
            Ok(Self::WithoutFilter {
                name,
                _assign: assign,
                ty: input.parse()?,
                connections: Default::default(),
            })
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
            .chain(self.edges.iter().map(|(direction, edge, node)| node))
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
        quote! { FnMut(#(#components),*) }
    }
    pub fn gen_callback_mut(&self, graph: &GraphQuery) -> TokenStream2 {
        let components = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(direction, edge, node)| node))
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
        quote! { FnMut(#(#components),*) }
    }
    pub fn gen_for_each(&self, graph: &GraphQuery) -> TokenStream2 {
        let callback = self.gen_callback(graph);
        let args = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(direction, edge, node)| node))
            .map(|node| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                quote! {&#component_var}
            });
        let inner = self.edges.iter().rev().fold(
            quote! { f(#(#args),*) },
            |inner, (direction, edge, node)| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                let node_var = format_ident!("node_{}", node, span = node.span());
                let peer_field = peer_name(&edge, *direction);
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
            pub fn #for_each_method_name(&self, mut f: impl #callback) {
                for (entity,#component_var) in self.#node_var.iter() {
                    #inner
                }
            }
            pub fn #for_each_from_method_name(&self, entity: bevy::ecs::entity::Entity, mut f: impl #callback) {
                if let Ok((_entity,#component_var)) = self.#node_var.get(entity) {
                    #inner
                }
            }
        }
    }
    pub fn gen_for_each_mut(&self, graph: &GraphQuery) -> TokenStream2 {
        let callback = self.gen_callback_mut(graph);
        let args = std::iter::once(&self.first_node)
            .chain(self.edges.iter().map(|(direction, edge, node)| node))
            .map(|node| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                quote! {&mut #component_var}
            });
        let inner = self.edges.iter().rev().fold(
            quote! { f(#(#args),*) },
            |inner, (direction, edge, node)| {
                let component_var = format_ident!("item_{}", node, span = node.span());
                let node_var = format_ident!("node_{}", node, span = node.span());
                let peer_field = peer_name(&edge, *direction);
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
            pub fn #for_each_method_name(&mut self, mut f:impl #callback) {
                for (entity,mut #component_var) in self.#node_var.iter_mut() {
                    #inner
                }
            }
            pub fn #for_each_from_method_name(&mut self, entity: bevy::ecs::entity::Entity, mut f:impl #callback) {
                if let Ok((_entity,mut #component_var)) = self.#node_var.get_mut(entity) {
                    #inner
                }
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
struct GraphQuery {
    name: Ident,
    _split: Token!(=>),
    _bracket: Bracket,
    _split2: Token!(=>),
    nodes: HashMap<String, NodeQuery>,
    _brace: Brace,
    pathes: HashMap<String, PathQuery>,
}
impl Parse for GraphQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let nodes;
        let pathes;
        Ok(Self {
            name: input.parse()?,
            _split: input.parse()?,
            _bracket: bracketed!(nodes in input),
            nodes: {
                let list = nodes.parse_terminated(NodeQuery::parse, Token!(,))?;
                HashMap::from_iter(list.into_iter().map(|node| (node.name().to_string(), node)))
            },
            _split2: input.parse()?,
            _brace: braced!(pathes in input),
            pathes: {
                let list = pathes.parse_terminated(PathQuery::parse, Token!(,))?;
                HashMap::from_iter(list.into_iter().map(|node| (node.name.to_string(), node)))
            },
        })
    }
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
    let mut graph_query = parse_macro_input!(input as GraphQuery);
    let mut peers = BTreeMap::new();
    for (pathName, path) in graph_query.pathes.iter() {
        let mut last_node = path.first_node.to_string();
        for (direction, edge, node) in path.edges.iter() {
            let peer_filed = peer_name(edge, *direction);
            peers.insert(
                peer_filed.to_string(),
                if *direction {
                    quote_spanned! { edge.span()=>
                        pub #peer_filed: Query<'w, 's, &'static <#edge as bevy_relationship::Relationship>::From>,
                    }
                } else {
                    quote_spanned! { edge.span()=>
                        pub #peer_filed: Query<'w, 's, &'static <#edge as bevy_relationship::Relationship>::To>,
                    }
                },
            );
        }
    }
    for node in graph_query.nodes.iter_mut() {}
    let name = &graph_query.name;
    let nodes = graph_query.nodes.values().map(|node| {
        let name = node.name();
        let field_name = format_ident!("node_{}", name.to_string(), span = name.span());
        let param = node.gen_query();
        let span = name.span();
        quote_spanned!(span=> pub #field_name: #param,)
    });
    let span = graph_query._bracket.span.clone();
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
        #[derive(bevy::ecs::system::SystemParam)]
        pub struct #name<'w, 's> {
            #(#nodes)*
            #(#peers_values)*
        }
        impl<'w, 's> #name<'w, 's>{
            #(#for_each_function)*
            #(#for_each_mut_function)*
        }
    };
    // panic!("{}", output);
    output.into()
}
