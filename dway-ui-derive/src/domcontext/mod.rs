pub mod spawn_context;
pub mod widget_context;

use crate::{
    dom::Dom,
    domarg::{control, DomArgKey},
};
use convert_case::Casing;
use quote::format_ident;
use std::any::Any;
use syn::*;

#[derive(Default)]
pub struct Context {
    pub namespace: String,
}

impl Context {}

pub struct NodeContext<'l> {
    pub dom: &'l Dom,
    pub dom_id: Ident,
}
impl<'l> NodeContext<'l> {
    pub fn get_var(&self, name: &str) -> Ident {
        DomContext::wrap_dom_id("__dway_ui_node_", &self.dom_id, name)
    }
    pub fn get_field(&self, name: &str) -> Ident {
        DomContext::wrap_dom_id("node_", &self.dom_id, name)
    }
    pub fn get_node_entity(&self) -> Ident {
        self.get_var("_entity")
    }
}

pub struct DomContext<'l> {
    pub context: &'l mut Context,
    pub dom_list: Vec<&'l Dom>,
    pub dom_stack: Vec<NodeContext<'l>>,
}

impl<'l> std::ops::DerefMut for DomContext<'l> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<'l> std::ops::Deref for DomContext<'l> {
    type Target = &'l mut Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<'l> DomContext<'l> {
    pub fn new(context: &'l mut Context) -> Self {
        Self {
            context,
            dom_list: Default::default(),
            dom_stack: vec![],
        }
    }

    pub fn push(&mut self, dom: &'l Dom) {
        let dom_id = self.get_dom_id(dom, false);
        self.dom_stack.push(NodeContext { dom, dom_id })
    }

    pub fn top(&self) -> &NodeContext {
        self.dom_stack.last().unwrap()
    }

    pub fn pop(&mut self) {
        self.dom_stack.pop();
    }

    fn get_dom_id(&mut self, dom: &'l Dom, upper_case: bool) -> Ident {
        self.dom_list.push(dom);
        if let Some(control::Id { id: lit }) = dom
            .args
            .iter()
            .find(|f| f.inner.key() == DomArgKey::Id)
            .map(|a| {
                (&*a.inner as &dyn Any)
                    .downcast_ref::<control::Id>()
                    .unwrap()
            })
        {
            format_ident!("{}", lit.value(), span = dom.span())
        } else if upper_case {
            format_ident!("N{}", self.dom_list.len(), span = dom.span())
        } else {
            format_ident!("n{}", self.dom_list.len(), span = dom.span())
        }
    }
    pub fn wrap_dom_id(prefix: &str, ident: &Ident, suffix: &str) -> Ident {
        format_ident!(
            "{}{}{}",
            prefix,
            ident.to_string().to_case(convert_case::Case::Snake),
            suffix
        )
    }
}
