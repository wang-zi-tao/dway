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
        let spawn_expr = dom.generate_spawn();

        let spawn_children: Vec<TokenStream> = dom
            .children
            .iter()
            .flat_map(|c| c.list.iter())
            .map(|child| self.generate(child))
            .collect();
        if spawn_children.is_empty() {
            quote! {
                #spawn_expr;
            }
        } else {
            let mut spawn_children = quote! {#(#spawn_children)*};
            for arg in dom.args.values() {
                spawn_children = arg.wrap_for_spawn(spawn_children);
            }
            quote! {
                #spawn_expr.with_children(|commands|{
                    #spawn_children
                });
            }
        }
    }
}

pub fn generate(dom: &Dom) -> TokenStream {
    let root_context = Context::default();
    let mut context = SpawnDomContext {
        dom_context: DomContext::new(&root_context, dom),
    };
    context.generate(dom)
}
