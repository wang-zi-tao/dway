use std::any::TypeId;

use crate::{parser::ParseCodeResult, prelude::*};

use super::{DomArgKey, DomDecorator};

#[derive(Parse)]
pub enum Connect {
    #[peek(token::Minus, name = "-")]
    To {
        #[prefix(Token![-])]
        #[bracket]
        _wrap: Bracket,
        #[inside(_wrap)]
        relation: Type,
        #[prefix(Token![->])]
        #[paren]
        _wrap2: Paren,
        #[inside(_wrap2)]
        expr: Expr,
    },
    #[peek(token::LArrow, name = "<-")]
    From {
        #[prefix(Token![<-])]
        #[bracket]
        _wrap: Bracket,
        #[inside(_wrap)]
        relation: Type,
        #[prefix(Token![-])]
        #[paren]
        _wrap2: Paren,
        #[inside(_wrap2)]
        expr: Expr,
    },
}
impl Connect {
    pub fn expr(&self) -> &Expr {
        match self {
            Connect::To { expr, .. } | Connect::From { expr, .. } => expr,
        }
    }
}

impl DomDecorator for Connect {
    fn key(&self) -> super::DomArgKey {
        match self {
            Connect::To { _wrap, .. } | Connect::From { _wrap, .. } => {
                DomArgKey::Other(TypeId::of::<Self>(), format!("{:?}", _wrap.span))
            }
        }
    }
    fn need_node_entity_field(&self) -> bool {
        let component_state = ParseCodeResult::from_expr(self.expr());
        !component_state.use_state.is_empty()
            || !component_state.set_state.is_empty()
            || !component_state.use_prop.is_empty()
    }

    fn update_context(&self, context: &mut WidgetNodeContext) {
        if self.need_node_entity_field() {
            let field = DomContext::wrap_dom_id("node_", &context.dom_id, "_conntcetion");
            context.tree_context.widget_builder.add_field_with_initer(
                &field,
                quote! {pub #field:Entity},
                quote! {Entity::PLACEHOLDER},
            )
        }
    }

    fn wrap_spawn(
        &self,
        inner: TokenStream,
        context: &mut DomContext,
        need_update: bool,
    ) -> TokenStream {
        let entity_var = context.top().get_node_entity();
        let expr = self.expr();
        let target_entity = if need_update && self.need_node_entity_field() {
            let field = DomContext::wrap_dom_id("node_", &context.top().dom_id, "_conntcetion");
            quote! {{widget.#field = #expr; widget.#field}}
        } else {
            quote!(#expr)
        };
        let stmts = match self {
            Connect::To { relation, .. } => {
                quote! {
                    commands.queue(bevy_relationship::ConnectCommand::<#relation>::new(#entity_var,#target_entity));
                }
            }
            Connect::From { relation, .. } => {
                quote! {
                    commands.queue(bevy_relationship::ConnectCommand::<#relation>::new(#target_entity,#entity_var));
                }
            }
        };
        quote! {
            #inner;
            #stmts
        }
    }

    fn generate_update(&self, context: &mut WidgetNodeContext) -> Option<TokenStream> {
        let expr = self.expr();
        let dependencies = ParseCodeResult::from_expr(expr);
        dependencies.is_changed().map(|check_changed|{
            let entity_var = &context.entity_var;
            let field = DomContext::wrap_dom_id("node_", &context.dom_id, "_conntcetion");
            let target_entity_var = DomContext::wrap_dom_id("__dway_ui_", &context.dom_id, "_target_entity");
            let update = match self {
                Connect::To { relation, .. } => {
                    quote! {
                        commands.queue(bevy_relationship::DisconnectCommand::<#relation>::new(#entity_var,#target_entity_var));
                        commands.queue(bevy_relationship::ConnectCommand::<#relation>::new(#entity_var,#target_entity_var));
                    }
                }
                Connect::From { relation, .. } => {
                    quote! {
                        commands.queue(bevy_relationship::DisconnectCommand::<#relation>::new(#target_entity_var,#entity_var));
                        commands.queue(bevy_relationship::ConnectCommand::<#relation>::new(#target_entity_var,#entity_var));
                    }
                }
            };
            quote!{
                if #check_changed {
                    let #target_entity_var = #expr;
                    if #target_entity_var != widget.#field{
                        #update
                    }
                }
            }
        })
    }
}
