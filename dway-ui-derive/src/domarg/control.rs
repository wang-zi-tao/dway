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
        quote_spanned! {expr.span()=>
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
        let old_value = DomContext::wrap_dom_id("node_", dom_id, "_child_inited");
        let field = DomContext::wrap_dom_id("node_", dom_id, "_enable_children");
        tree_context.widget_builder.add_field_with_initer(
            &field,
            quote!(pub #field: bool),
            quote!(false),
        );
        quote_spanned! {expr.span()=>
            let #old_value = widget.#field;
            if #just_inited || #enable_expr_changed {
                widget.#field = #expr;
                if !#just_inited && !widget.#field {
                    commands.entity(#entity_var).queue(dway_ui_framework::command::destroy_children_ui);
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
pub struct DomPat {
    pub name: Ident,
    #[prefix(Token![=>])]
    pub block: Block,
}

#[derive(Parse)]
pub struct DomPatList {
    #[prefix(Token![=>])]
    #[bracket]
    _wrap: Bracket,
    #[inside(_wrap)]
    #[call(Punctuated::parse_terminated)]
    pub pats: Punctuated<DomPat, Token![,]>,
}

#[derive(Parse)]
pub struct For {
    #[call(Pat::parse_multi)]
    pat: Pat,
    #[prefix(Token![:])]
    ty: Type,
    #[prefix(Token![in])]
    expr: Expr,
    #[prefix(Token![=>])]
    pub update: Block,
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
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", dom_id, "_child_list");
        tree_context.widget_builder.add_field_with_initer(
            &dom_entity_list_field,
            quote!(pub #dom_entity_list_field: Vec<Entity>),
            quote!(Vec::new()),
        );
    }
    fn wrap_spawn_children(&self, inner: TokenStream, _context: &mut DomContext) -> TokenStream {
        let Self { pat, expr, .. } = self;
        quote_spanned! {pat.span()=>
            for #pat in #expr{
                 #inner
            }
        }
    }
    fn wrap_sub_widget(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self { pat, update, .. } = self;
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_item");
        quote_spanned! {pat.span()=>
            if let Some(#pat) = #item_var {
                #update
            }
            #inner
        }
    }
    fn wrap_update_children(
        &self,
        child_ident: Option<Ident>,
        inner: TokenStream,
        context: &mut WidgetNodeContext,
    ) -> TokenStream {
        let expr = &self.expr;
        let ty = &self.ty;
        let just_inited = &context.just_inited;
        let entity_var = &context.entity_var;
        let dom_entity_list_field =
            DomContext::wrap_dom_id("node_", &context.dom_id, "_child_list");
        let child_list_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_list");
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_item");
        let lambda_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_lambda");
        let changed = ParseCodeResult::from_expr(&self.expr).changed_bool();

        let backup_state = format_ident!("{}_state",context.tree_context.state_namespace);
        let backup_widget = format_ident!("{}_widget",context.tree_context.state_namespace);
        let state_name = &context.tree_context.state_builder.name;
        let widget_name = &context.tree_context.widget_builder.name;

        quote_spanned! {self.pat.span()=>
            let mut #lambda_var = |
                #child_ident,
                #just_inited,
                commands:&mut Commands,
                #item_var: Option<#ty>,
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                #backup_state: &#state_name,
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                #backup_widget: &#widget_name,
            | {
                #inner
                #child_ident
            };
            if #just_inited || #changed {
                let mut #child_list_var = Vec::<Entity>::new();
                if #just_inited {
                    widget.#dom_entity_list_field.clear();
                } else {
                    commands.entity(#entity_var).queue(dway_ui_framework::command::destroy_children_ui);
                }
                for #item_var in #expr {
                    let #child_ident = #lambda_var(Entity::PLACEHOLDER,true,commands,Some(#item_var),&state,&widget);
                    widget.#dom_entity_list_field.push(#child_ident);
                }
                widget.#dom_entity_list_field = #child_list_var;
            } else {
                for &#child_ident in widget.#dom_entity_list_field.iter() {
                    #lambda_var(#child_ident,#just_inited,commands,None,&state,&widget);
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
    _split: Token![<=],
    #[call(Pat::parse_multi)]
    pat: Pat,
    _in: Token![in],
    expr: Expr,
    #[prefix(Token![=>])]
    pub update: Block,
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
        let Self { ty, .. } = self;
        let WidgetNodeContext {
            tree_context,
            dom_id,
            ..
        } = context;
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", dom_id, "_child_map");
        tree_context.widget_builder.add_field_with_initer(
            &dom_entity_list_field,
            quote_spanned!(ty.span()=> #[reflect(ignore)]pub #dom_entity_list_field: std::collections::BTreeMap<#ty,Entity>),
            quote_spanned!(ty.span()=> std::collections::BTreeMap::new()),
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
        quote_spanned! {pat.span()=>
            let #child_entity_map_var = std::collections::BTreeMap::<#ty,Entity>::new();
            for #item_var @ #pat in #expr{
                 #child_entity_map_var.insert(#key, #item_var);
            }
            for #pat in #child_entity_map_var{
                 #inner
            }
        }
    }
    fn wrap_sub_widget(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self { pat, update, .. } = self;
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_item");
        quote_spanned! {pat.span()=>
            if let Some(#pat) = #item_var {
                #update
            }
            #inner
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
            key, ty, pat, _in, expr, ..
        } = self;
        let child_entity_map_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_entity_map");
        let child_list_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_list");
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &context.dom_id, "_child_map");
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_item");
        let key_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_key");
        let lambda_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_lambda");
        let changed = ParseCodeResult::from_expr(expr).changed_bool();

        let backup_state = format_ident!("{}_state",context.tree_context.state_namespace);
        let backup_widget = format_ident!("{}_widget",context.tree_context.state_namespace);
        let state_name = &context.tree_context.state_builder.name;
        let widget_name = &context.tree_context.widget_builder.name;

        quote_spanned! {_in.span()=>
            let mut #lambda_var = |
                #child_ident,
                #just_inited,
                commands:&mut Commands,
                #item_var,
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                #backup_state: &#state_name,
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                #backup_widget: &#widget_name,
            | {
                #inner
                #child_ident
            };
            if #just_inited {
                widget.#dom_entity_list_field.clear();
            }
            if #just_inited || #changed {
                let mut #child_entity_map_var = std::collections::BTreeMap::<#ty,Entity>::new();
                let mut #child_list_var = Vec::new();
                for #item_var in #expr{
                    let #pat = &#item_var;
                    #child_entity_map_var.insert(#key, #item_var);
                }
                for #item_var in #expr{
                    let #key_var = {
                        let #pat = &#item_var;
                        #key
                    };
                    let #child_ident: Entity = widget.#dom_entity_list_field.remove(&#key_var).unwrap_or(Entity::PLACEHOLDER);
                    let #just_inited = #child_ident == Entity::PLACEHOLDER;
                    let #child_ident = #lambda_var(#child_ident,#just_inited,commands,Some(#item_var),&state,&widget);
                    #child_entity_map_var.insert(#key_var, #child_ident);
                    #child_list_var.push(#child_ident);
                }
                for removeed_children in widget.#dom_entity_list_field.values() {
                    commands.entity(*removeed_children).queue(dway_ui_framework::command::destroy_ui);
                }
                commands.entity(#entity_var).replace_children(&#child_list_var);
                widget.#dom_entity_list_field = #child_entity_map_var;
            } else {
                for &#child_ident in widget.#dom_entity_list_field.values() {
                    #lambda_var(#child_ident,#just_inited,commands,None,&state,&widget);
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
    update: DomPatList,
    _split2: Option<Token![=>]>,
    #[parse_if(_split2.is_some())]
    other_stmt: Option<Block>,
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
            quote!(pub #dom_entity_list_field: bevy::ecs::entity::EntityHashMap<Entity>),
            quote!(bevy::ecs::entity::EntityHashMap::default()),
        );
    }
    fn update_sub_widget_context(&self, context: &mut WidgetNodeContext) {
        context.tree_context.widget_builder.add_field_with_initer(
            &format_ident!("data_entity"),
            quote!(pub data_entity:Entity),
            quote!(Entity::PLACEHOLDER),
        );
    }
    fn wrap_sub_widget(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let just_inited = &context.just_inited;
        let Self { pat, update, other_stmt, .. } = self;
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_item");
        let data_entity_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_data_entity");
        let update_componets = update.pats.iter().map(|p| {
            let DomPat { name, block } = p;
            quote_spanned! {name.span()=>
                if #just_inited || #name.is_changed() {
                    #block
                }
            }
        });
        quote_spanned! {pat.span()=>
            {
                if #just_inited {
                    widget.data_entity = #data_entity_var;
                }
                let #pat = #item_var;
                #(#update_componets)*
                #other_stmt
            }
            #inner
        }
    }
    fn wrap_update_children(
        &self,
        child_ident: Option<Ident>,
        inner: TokenStream,
        context: &mut WidgetNodeContext,
    ) -> TokenStream {
        let just_inited = &context.just_inited;
        let Self {
            expr, method, ty, ..
        } = self;
        let child_entity_map_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_entity_map");
        let dom_entity_list_field = DomContext::wrap_dom_id("node_", &context.dom_id, "_child_map");
        let data_entity_var =
            DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_data_entity");
        let arg_name = format_ident!("query_{}", context.dom_id);
        let item_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_child_item");
        let lambda_var = DomContext::wrap_dom_id("__dway_ui_node_", &context.dom_id, "_lambda");
        let changed = if matches!(&*method.to_string(), "iter" | "iter_mut") {
            BoolExpr::True
        } else {
            expr.as_ref()
                .map(|expr| ParseCodeResult::from_expr(expr).changed_bool())
                .unwrap_or_else(|| BoolExpr::False)
        };
        let get_method = self
            .mutable
            .map(|_| quote!(get_mut))
            .unwrap_or_else(|| quote!(get));
        let item_type = if self.mutable.is_some() {
            quote!(bevy::ecs::query::QueryItem<'_, #ty>)
        } else {
            quote!(bevy::ecs::query::ROQueryItem<'_, #ty>)
        };

        let backup_state = format_ident!("{}_state",context.tree_context.state_namespace);
        let backup_widget = format_ident!("{}_widget",context.tree_context.state_namespace);
        let state_name = &context.tree_context.state_builder.name;
        let widget_name = &context.tree_context.widget_builder.name;

        quote_spanned! {self._in.span=>
            let mut #lambda_var = |
                #child_ident,
                #just_inited,
                commands:&mut Commands,
                #data_entity_var,
                #item_var:#item_type,
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                #backup_state: &#state_name,
                #[allow(non_snake_case)]
                #[allow(unused_variables)]
                #backup_widget: &#widget_name,
            | {
                #inner
                #child_ident
            };
            if #just_inited {
                widget.#dom_entity_list_field.clear();
            }
            if #just_inited || #changed {
                let mut #child_entity_map_var = bevy::ecs::entity::EntityHashMap::<Entity>::default();
                for (#data_entity_var,#item_var) in #arg_name.#method(#expr) {
                    let #child_ident: Entity = widget.#dom_entity_list_field.remove(&#data_entity_var).unwrap_or(Entity::PLACEHOLDER);
                    let #just_inited = #child_ident == Entity::PLACEHOLDER;
                    let #child_ident = #lambda_var(#child_ident,#just_inited,commands,#data_entity_var,#item_var,&state,&widget);
                    #child_entity_map_var.insert(#data_entity_var,#child_ident);
                }
                for (_,removeed_children) in widget.#dom_entity_list_field.drain() {
                    commands.entity(removeed_children).queue(dway_ui_framework::command::destroy_ui);
                }
                widget.#dom_entity_list_field = #child_entity_map_var;
            } else {
                for (&#data_entity_var,&#child_ident) in widget.#dom_entity_list_field.iter() {
                    if let Ok((#data_entity_var,#item_var)) = #arg_name.#get_method(#data_entity_var) {
                        #lambda_var(#child_ident,#just_inited,commands,#data_entity_var,#item_var,&state,&widget);
                    }
                }
            }
        }
    }
}

#[derive(Parse)]
pub struct Command {
    command: Expr,
}

impl DomDecorator for Command{
    fn wrap_spawn(
            &self,
            inner: TokenStream,
            context: &mut DomContext,
            _need_update: bool,
        ) -> TokenStream {
        let entity = context.top().get_node_entity();
        let command = &self.command;
        quote_spanned!{entity.span()=>
            #inner
            commands.entity(#entity).queue(#command);
        }
    }
    fn generate_update(&self, context: &mut WidgetNodeContext) -> Option<TokenStream> {
        let Self { command, .. } = self;
        let entity = &context.entity_var;
        let dependencies = ParseCodeResult::from_expr(command);
        dependencies.is_changed().map(|check_changed| {
            quote_spanned! {entity.span()=>
                if #check_changed {
                    commands.entity(#entity).queue(#command);
                }
            }
        })
    }
}
