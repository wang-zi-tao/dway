use proc_macro::Ident;
use std::{collections::HashMap, rc::Rc};
use syn::Expr;

use crate::syntax::node::NodeQuery;

structstruck::strike! {
    pub struct ComponentModel {
        pub name: String,
        pub node: Rc<NodeQuery>,
        pub workd_query: enum {

        },
    }
}

pub struct BundleModel {
    pub components: Vec<Rc<ComponentModel>>,
}

pub struct NodeModel {
    pub bundles: Vec<Rc<BundleModel>>,
    pub filter: Option<Expr>,
    pub name: Option<Ident>,
}

pub struct PathModel {
    pub nodes: Vec<Rc<NodeModel>>,
}

structstruck::strike! {
    pub struct GraphModel {
        pub bundles: Vec<Rc<BundleModel>>,
        pub component: HashMap<String, Rc<ComponentModel>>,
        pub component_bundle_index: HashMap<String, Rc<BundleModel>>,
    }
}

impl GraphModel {
    pub fn add_node(&mut self, _node: Rc<NodeQuery>) {
        todo!()
    }
}
