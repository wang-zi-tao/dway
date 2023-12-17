use crate::dom::Dom;
use proc_macro2::TokenStream;
use quote::quote;

use super::{Context, DomContext};

pub struct SpawnDomContext<'l> {
    pub dom_context: DomContext<'l>,
}

impl<'l> std::ops::DerefMut for SpawnDomContext<'l> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dom_context
    }
}

impl<'l> std::ops::Deref for SpawnDomContext<'l> {
    type Target = DomContext<'l>;

    fn deref(&self) -> &Self::Target {
        &self.dom_context
    }
}

impl<'l> SpawnDomContext<'l> {
    pub fn generate(&mut self, dom: &'l Dom) -> TokenStream {
        self.push(dom);
        let spawn_expr = dom.generate_spawn();

        let spawn_children: Vec<TokenStream> = dom
            .children
            .iter()
            .flat_map(|c| c.list.iter())
            .map(|child| self.generate(child))
            .collect();

        let entity = self.top().get_node_entity();

        let tokens = if spawn_children.is_empty() {
            quote! {
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                let #entity = #spawn_expr;
            }
        } else {
            let spawn_children = dom
                .args
                .iter()
                .fold(quote! {#(#spawn_children)*}, |inner, arg| {
                    arg.inner.wrap_spawn_children(inner, &mut self.dom_context)
                });
            quote! {
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                let #entity = #spawn_expr.with_children(|commands|{
                    #spawn_children
                });
            }
        };
        let tokens = dom.args.iter().fold(tokens, |inner, arg| {
            arg.inner.wrap_spawn(inner, &mut self.dom_context, false)
        });
        self.pop();
        tokens
    }
}

pub fn generate(dom: &Dom) -> TokenStream {
    let mut root_context = Context::default();
    let mut context = SpawnDomContext {
        dom_context: DomContext::new(&mut root_context),
    };
    context.generate(dom)
}
