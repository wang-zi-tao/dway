use crate::{parser::ParseCodeResult, prelude::*};

use super::{DomArgKey, DomDecorator};

#[derive(Parse)]
pub struct Id {
    pub id: LitStr,
}
impl DomDecorator for Id {
    fn key(&self) -> DomArgKey {
        DomArgKey::Id
    }
    fn need_node_entity_field(&self) -> bool {
        true
    }
}

#[derive(Parse)]
pub struct If {
    expr: Expr,
}
impl DomDecorator for If {
    fn key(&self) -> super::DomArgKey {
        super::DomArgKey::If
    }

    fn need_node_entity_field(&self) -> bool {
        true
    }
    fn wrap_spawn_children(&self, inner: TokenStream, _context: &mut DomContext) -> TokenStream {
        let expr = &self.expr;
        quote! {
            if #expr {
                #inner
            }
        }
    }
    fn wrap_update_children(
        &self,
        _child_entity: Option<Ident>,
        inner: TokenStream,
        context: &mut WidgetNodeContext,
    ) -> TokenStream {
        let expr = &self.expr;
        let enable_expr_stat = ParseCodeResult::from_expr(expr);
        let enable_expr_changed = enable_expr_stat.changed_bool();
        let WidgetNodeContext {
            tree_context,
            dom_id,
            entity_var,
            just_inited,
            ..
        } = context;
        let old_value = DomContext::wrap_dom_id("node_", &dom_id, "_child_inited");
        let field = DomContext::wrap_dom_id("node_", &dom_id, "_enable_children");
        tree_context.widget_builder.add_field_with_initer(
            &field,
            quote!(pub #field: bool),
            quote!(false),
        );
        quote! {
            let #old_value = widget.#field;
            if #just_inited || #enable_expr_changed {
                widget.#field = #expr;
                if !#just_inited && !widget.#field {
                    commands.entity(#entity_var).despawn_descendants();
                }
            };
            if widget.#field {
                let #just_inited = #just_inited || !#old_value;
                #inner
            }
        }
    }
}

#[derive(Parse)]
pub struct For {
    #[call(Pat::parse_multi)]
    pat: Pat,
    _in: Token![in],
    expr: Expr,
}
impl DomDecorator for For {
    fn key(&self) -> super::DomArgKey {
        DomArgKey::For
    }

    fn need_node_entity_field(&self) -> bool {
        true
    }
    fn need_sub_widget(&self) -> bool {
        true
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let WidgetNodeContext {
            tree_context,
            dom_id,
            ..
        } = context;
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &dom_id, "_child_map");
        tree_context.widget_builder.add_field_with_initer(
            &dom_entity_list_field,
            quote!(pub #dom_entity_list_field: Vec<Entity>),
            quote!(Vec::new()),
        );
    }
    fn wrap_spawn_children(&self, inner: TokenStream, _context: &mut DomContext) -> TokenStream {
        let Self { pat, _in, expr } = self;
        quote! {
            for #pat in #expr{
                 #inner
            }
        }
    }
    fn wrap_update_children(
        &self,
        child_entity: Option<Ident>,
        inner: TokenStream,
        context: &mut WidgetNodeContext,
    ) -> TokenStream {
        let Self { pat, _in, expr } = self;
        let WidgetNodeContext {
            dom_id,
            just_inited,
            ..
        } = context;
        let child_entity_var = DomContext::wrap_dom_id("__dway_ui_node_", &dom_id, "_child_entity");
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &dom_id, "_child_list");
        let changed = ParseCodeResult::from_expr(expr).changed_bool();
        quote! {
            if #just_inited || #changed {
                widget.#dom_entity_list_field.clear();
                for #pat in #expr {
                    #inner
                    widget.#dom_entity_list_field.push(#child_entity);
                }
            } else {
                for &#child_entity_var in widget.#dom_entity_list_field.iter() {
                    #inner
                }
            }
        }
    }
}

#[derive(Parse)]
pub struct Map {
    key: Expr,
    _col: Token![:],
    ty: Type,
    _split: Token![<-],
    #[call(Pat::parse_multi)]
    pat: Pat,
    _in: Token![in],
    expr: Expr,
}

impl DomDecorator for Map {
    fn key(&self) -> DomArgKey {
        DomArgKey::For
    }
    fn need_node_entity_field(&self) -> bool {
        true
    }
    fn need_sub_widget(&self) -> bool {
        true
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            key, ty, pat, expr, ..
        } = self;
        let WidgetNodeContext {
            tree_context,
            dom_id,
            ..
        } = context;
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &dom_id, "_child_map");
        tree_context.widget_builder.add_field_with_initer(
            &dom_entity_list_field,
            quote!(pub #dom_entity_list_field: bevy::utils::HashMap<#ty,Entity>),
            quote!(bevy::utils::HashMap::new()),
        );
    }
    fn wrap_spawn_children(&self, inner: TokenStream, context: &mut DomContext) -> TokenStream {
        let Self {
            key, ty, pat, expr, ..
        } = self;
        let dom_id = &context.dom_stack.last().unwrap().dom_id;
        let child_entity_map_var =
            DomContext::wrap_dom_id("__dway_ui_node_", dom_id, "_child_entity_map");
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", dom_id, "_item");
        quote! {
            let #child_entity_map_var = std::collections::BTreeMap::<#ty,Entity>::new();
            for #item_var @ #pat in #expr{
                 #child_entity_map_var.insert(#key, #item_var);
            }
            for #pat in #child_entity_map_var{
                 #inner
            }
        }
    }
    fn wrap_update_children(
        &self,
        child_ident: Option<Ident>,
        inner: TokenStream,
        context: &mut WidgetNodeContext,
    ) -> TokenStream {
        let entity_var = &context.entity_var;
        let just_inited = &context.just_inited;
        let Self {
            key, ty, pat, expr, ..
        } = self;
        let child_entity_map_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_child_entity_map");
        let child_list_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_child_list");
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &context.dom_id, "_child_map");
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_item");
        let lambda_var = DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_lambda");
        let changed = ParseCodeResult::from_expr(expr).changed_bool();
        quote! {
            let #lambda_var = |#child_ident,#pat| {
                #inner
                #child_ident
            };
            if #just_inited {
                widget.#dom_entity_list_field.clear();
            }
            if #just_inited || #changed {
                let mut #child_entity_map_var = std::collections::BTreeMap::<#ty,Entity>::new();
                for #item_var @ #pat in #expr{
                    #child_entity_map_var.insert(#key, #item_var);
                }
                let mut #child_list_var = Vec::<Entity>::with_capacity(#child_entity_map_var.len());
                for #item_var in #child_entity_map_var.values(){
                    if let Some(#child_ident) = widget.#dom_entity_list_field.remove(&#key) {
                        let #child_ident = #lambda_var(#child_ident, #item_var);
                        #child_list_var.push(#child_ident);
                    } else {
                        let #child_ident = #lambda_var(Entity::PLACEHOLDER, #item_var);
                        #child_list_var.push(#child_ident);
                    }
                }
                for (_,removeed_children) in widget.#dom_entity_list_field.drain() {
                    commands.entity(removeed_children).despawn_recursive();
                }
                commands.entity(#entity_var).replace_children(&#child_list_var);
                widget.#dom_entity_list_field = #child_entity_map_var;
            } else {
                for (#item_var,&#entity_var) in widget.#dom_entity_list_field.iter() {
                    #lambda_var(#entity_var, #item_var);
                }
            }
        }
    }
}

fn parse_optional_expr(input: ParseStream) -> syn::Result<Option<Expr>> {
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input.parse()?))
    }
}

#[derive(Parse)]
pub struct ForQuery {
    mutable: Option<Token![mut]>,
    #[call(Pat::parse_multi)]
    pat: Pat,
    _in: Token![in],
    _query: Ident,
    _lt: Token![<],
    ty: Type,
    _gt: Token![>],
    _split: Token![::],
    method: Ident,
    #[paren]
    _wrap2: Paren,
    #[inside(_wrap2)]
    #[call(parse_optional_expr)]
    expr: Option<Expr>,
}

impl DomDecorator for ForQuery {
    fn key(&self) -> DomArgKey {
        DomArgKey::For
    }
    fn need_node_entity_field(&self) -> bool {
        true
    }
    fn need_sub_widget(&self) -> bool {
        true
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self { ty, mutable, .. } = self;
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &context.dom_id, "_child_map");
        let arg_name = format_ident!("query_{}", context.dom_id);
        context.tree_context.system_querys.insert(
            arg_name.to_string(),
            quote!(#mutable #arg_name: Query<(Entity,#ty)>),
        );
        context.tree_context.widget_builder.add_field_with_initer(
            &dom_entity_list_field,
            quote!(pub #dom_entity_list_field: bevy::utils::HashMap<Entity,Entity>),
            quote!(bevy::utils::HashMap::new()),
        );
    }
    fn wrap_update_children(
        &self,
        child_ident: Option<Ident>,
        inner: TokenStream,
        context: &mut WidgetNodeContext,
    ) -> TokenStream {
        let entity_var = &context.entity_var;
        let just_inited = &context.just_inited;
        let Self {
            pat,
            expr,
            method,
            _in,
            ..
        } = self;
        let child_entity_map_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_child_entity_map");
        let child_list_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_child_list");
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &context.dom_id, "_child_map");
        let data_entity_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &(&context.dom_id), "_data_entity");
        let arg_name = format_ident!("query_{}", context.dom_id);
        let changed = expr
            .as_ref()
            .map(|expr| ParseCodeResult::from_expr(expr).changed_bool())
            .unwrap_or_else(|| BoolExpr::False);
        quote_spanned! {_in.span=>
            if #just_inited {
                widget.#dom_entity_list_field.clear();
            }
            if !#just_inited || #changed {
                let mut #child_entity_map_var = bevy::utils::HashMap::<Entity,Entity>::new();
                let mut #child_list_var = Vec::<Entity>::new();
                for (#data_entity_var,#pat) in #arg_name.#method(#expr) {
                    let #child_ident: Entity = widget.#dom_entity_list_field.remove(&#data_entity_var).unwrap_or(Entity::PLACEHOLDER);
                    let #just_inited = #child_ident == Entity::PLACEHOLDER;
                    #inner
                    #child_entity_map_var.insert(#data_entity_var,#child_ident);
                    #child_list_var.push(#child_ident);
                }
                for (_,removeed_children) in widget.#dom_entity_list_field.drain() {
                    commands.entity(removeed_children).despawn_recursive();
                }
                commands.entity(#entity_var).replace_children(&#child_list_var);
                widget.#dom_entity_list_field = #child_entity_map_var;
            } else {
                let mut #child_entity_map_var = bevy::utils::HashMap::<Entity,Entity>::new();
                let mut #child_list_var = Vec::<Entity>::new();
                for (#data_entity_var,#pat) in #arg_name.#method(#expr) {
                    let #child_ident = Entity::PLACEHOLDER;
                    let #just_inited = true;
                    #inner
                    #child_list_var.push(#child_ident);
                    widget.#dom_entity_list_field.insert(#data_entity_var,#child_ident);
                }
                widget.#dom_entity_list_field = #child_entity_map_var;
            }
        }
    }
}
