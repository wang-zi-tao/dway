use crate::{dom::Dom, domarg::DomArg, generate::BoolExpr, ParseCodeResult};
use derive_syn_parse::Parse;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashMap};
use syn::{
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{At, Brace, Paren, RArrow},
    *,
};

use super::DomContext;

pub struct WidgetDomContext<'l> {
    pub dom_context: DomContext<'l>,
}

impl<'l> std::ops::DerefMut for WidgetDomContext<'l> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dom_context
    }
}

impl<'l> std::ops::Deref for WidgetDomContext<'l> {
    type Target = DomContext<'l>;

    fn deref(&self) -> &Self::Target {
        &self.dom_context
    }
}

impl<'l> WidgetDomContext<'l> {
    pub fn generate(
        &mut self,
        dom: &'l Dom,
        parent_entity: &Ident,
        parent_just_inited: &Ident,
        enabled: &BoolExpr,
    ) -> TokenStream {
        let dom_id = self.get_dom_id(dom, false);

        let entity_var = DomContext::wrap_dom_id("__dway_ui_node_", &dom_id, "_entity");
        let just_init_var = DomContext::wrap_dom_id("__dway_ui_node_", &dom_id, "_just_inited");

        let (child_enabled_var, child_update_enabled) =
            if let Some(DomArg::If { expr, .. }) = dom.args.get(&crate::domarg::DomArgKey::If) {
                let enable_expr_stat = ParseCodeResult::from_expr(expr);
                let enable_expr_changed = enable_expr_stat.is_changed();
                let var = DomContext::wrap_dom_id("__dway_ui_node_", &dom_id, "_children_enable");
                (
                    BoolExpr::RuntimeValue(quote!(#var)),
                    Some(quote_spanned! {dom.span()=>
                        let enable_expr = if #enable_expr_changed {
                            #entity_var == Entity::PLACEHOLDER
                        } else {
                            #enabled && #expr
                        };
                    }),
                )
            } else {
                (enabled.clone(), None)
            };

        let need_node_entity = dom.args.values().any(|arg| arg.need_node_entity());
        let entity_expr = if need_node_entity {
            let field = DomContext::wrap_dom_id("node_", &dom_id, "_entity");
            quote!(state.#field)
        }else {
            quote!(Entity::PLACEHOLDER)
        };

        let prepare_stat = quote! {
            let mut #entity_var = #entity_expr;
            let mut #just_init_var = false;
            #child_update_enabled
        };

        let init_stat = {
            let spawn_expr = dom.generate_spawn();
            quote_spanned! {dom.span()=>
                let #entity_var = #spawn_expr.set_parent(#parent_entity).id();
                #just_init_var = true;
            }
        };
        let update_component_stat = dom
            .args
            .values()
            .map(|arg| arg.generate_update(&entity_var))
            .collect::<Vec<_>>();
        let update_stat = if update_component_stat.is_empty() {
            None
        } else {
            Some(quote_spanned! {dom.span()=>
                #(#update_component_stat)*
            })
        };
        let despawn_stat = {
            quote_spanned! {dom.span()=>
                if #entity_var != Entity::PLACEHOLDER {
                    commands.entity(#entity_var).despawn_recursive();
                }
            }
        };

        let process_node_stat = enabled.map(
            BoolExpr::RuntimeValue(quote!(#parent_just_inited ))
                .to_if_else(quote! { #init_stat }, update_stat.as_ref())
                .as_ref(),
            quote_spanned! {dom.span()=>
                if #parent_just_inited {
                    if #entity_var != Entity::PLACEHOLDER {
                        #despawn_stat
                    }
                }
            },
        );

        let spawn_children: Vec<TokenStream> = dom
            .children
            .iter()
            .flat_map(|c| c.list.iter())
            .map(|child| self.generate(child, &entity_var, &just_init_var, &child_enabled_var))
            .collect();
        if let Some(DomArg::For {  pat, expr, .. }) = dom.args.get(&crate::domarg::DomArgKey::For){

        }

        quote_spanned! {dom.span()=>
            #prepare_stat
            #process_node_stat
            #(#spawn_children)*
        }
    }
}
