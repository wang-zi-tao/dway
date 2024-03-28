use derive_syn_parse::Parse;
use proc_macro2::Ident;
use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned,
    token::{self, Bracket, Paren}, Token,
};

use crate::{edge::EdgeQuery, filter::Filter, node::NodeQuery, query::QueryBuilder};

#[derive(Parse)]
pub enum PathDirection {
    #[peek(token::FatArrow,name = "=>")]
    LeftToRight(Token![=>]),
    #[peek(token::Le,name = "<=")]
    RightToLeft(Token![<=]),
    #[peek(token::Eq,name = "=")]
    Both(Token![=]),
}

pub struct PathDecl {
    pub name: Ident,
    pub direction: PathDirection,
    pub path: PathQuery,
}

#[derive(Clone, Copy)]
pub enum EdgeDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Parse)]
pub struct PathEdgeQuery {
    #[paren]
    pub paren: Paren,
    // #[inside(paren)]
    // pub multi: Option<Token![*]>,
    // #[inside(paren)]
    // #[parse_if(multi.is_some())]
    // pub range: Option<ExprRange>,
    #[inside(paren)]
    pub edge: EdgeQuery,
    #[inside(paren)]
    pub where_: Option<Token![where]>,
    #[inside(paren)]
    #[parse_if(where_.is_some())]
    pub filter: Option<Filter>,
}

impl PathEdgeQuery {
    pub fn build(&self, builder: &mut QueryBuilder, direction: &EdgeDirection) {
        if let (Some(where_), Some(Filter { expr })) = (&self.where_, &self.filter) {
            let code = std::mem::replace(&mut builder.code, quote!());
            let code = quote_spanned! {where_.span=>
                if #expr {
                    #code
                }
            };
            builder.code = code;
        }

        self.edge.build(builder, direction);
    }
}

#[derive(Parse)]
pub struct PathNodeQuery {
    #[bracket]
    pub bracket: Bracket,
    #[inside(bracket)]
    pub name: Ident,
    #[inside(bracket)]
    pub col: Token![:],
    #[inside(bracket)]
    pub node: NodeQuery,
    #[inside(bracket)]
    pub where_: Option<Token![where]>,
    #[inside(bracket)]
    #[parse_if(where_.is_some())]
    pub filter: Option<Filter>,
}

impl PathNodeQuery {
    pub fn build(&self, builder: &mut QueryBuilder) {
        if let (Some(where_), Some(Filter { expr })) = (&self.where_, &self.filter) {
            let code = std::mem::replace(&mut builder.code, quote!());
            let code = quote_spanned! {where_.span=>
                if #expr {
                    #code
                }
            };
            builder.code = code;
        }
        self.node.build(builder, &self.name);
    }
}

pub struct PathQuery {
    pub nodes: Vec<PathNodeQuery>,
    pub edges: Vec<(PathEdgeQuery, EdgeDirection)>,
}

impl PathQuery {
    pub fn build_foreach(&self, query: &mut QueryBuilder, mutable: bool) {
        self.build_foreach_inner(query, mutable, &self.nodes, &self.edges)
    }
    fn build_foreach_inner(
        &self,
        builder: &mut QueryBuilder,
        mutable: bool,
        nodes: &[PathNodeQuery],
        edges: &[(PathEdgeQuery, EdgeDirection)],
    ) {
        if !nodes.is_empty() && !edges.is_empty() {
            self.build_foreach_inner(builder, mutable, &nodes[1..], &edges[1..]);

            let node = nodes.last().unwrap();
            let (edge, edge_direction) = edges.last().unwrap();
            node.build(builder);
            edge.build(builder, edge_direction);
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
