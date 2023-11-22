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

use crate::{
    domarg::{DomArg, DomArgKey},
    parse_expr, ParseCodeResult,
};

pub struct DomChildren {
    pub list: Vec<Dom>,
}
impl syn::parse::Parse for DomChildren {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut list = vec![];
        while !input.peek(Token![<]) || !input.peek2(Token![/]) {
            list.push(input.parse()?);
        }
        Ok(Self { list })
    }
}

#[derive(Parse)]
enum DomBundle {
    #[peek(Paren, name = "Paren")]
    Expr {
        #[paren]
        _wrap: Paren,
        #[inside(_wrap)]
        expr: Expr,
    },
    #[peek(Ident, name = "Ident")]
    Ident(Type),
}
impl ToTokens for DomBundle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ty) => tokens.extend(quote!(#ty::default())),
            Self::Expr { expr, .. } => tokens.extend(quote!(#expr)),
        }
    }
}
impl DomBundle {
    pub fn generate_spawn(&self, ty: Option<TokenStream>) -> TokenStream {
        match &self {
            DomBundle::Expr {
                expr: Expr::Tuple(inner),
                ..
            } if inner.elems.is_empty() => {
                quote!(commands.spawn_empty())
            }
            DomBundle::Expr { expr, .. } => {
                if let Some(ty) = ty {
                    quote!(commands.spawn(#expr as #ty))
                } else {
                    quote!(commands.spawn(#expr))
                }
            }
            DomBundle::Ident(bundle_tyle) => {
                if let Some(ty) = ty {
                    quote!(commands.spawn(#bundle_tyle::default() as #ty))
                } else {
                    quote!(commands.spawn(#bundle_tyle::default()))
                }
            }
        }
    }
}

#[derive(Parse)]
struct DomEnd {
    _lt1: Token![<],
    _end1: Token![/],
    pub end_bundle: Option<Ident>,
    _gt1: Token![>],
}

#[derive(Parse)]
pub struct Dom {
    _lt0: Token![<],
    pub bundle: DomBundle,
    #[call(DomArg::parse_map)]
    pub args: BTreeMap<DomArgKey, DomArg>,
    _end0: Option<Token![/]>,
    _gt0: Token![>],
    #[parse_if(_end0.is_none())]
    pub children: Option<DomChildren>,
    #[parse_if(_end0.is_none())]
    pub end_tag: Option<DomEnd>,
}
impl Dom {
    pub fn span(&self) -> Span {
        self._lt0.span().join(self._gt0.span()).unwrap()
    }

    pub fn generate_spawn(&self) -> TokenStream {
        let mut spawn_bundle = self.bundle.generate_spawn(
            self.end_tag
                .as_ref()
                .and_then(|end| end.end_bundle.as_ref())
                .map(|ty| quote!(#ty)),
        );
        let mut components_expr: Vec<_> = self
            .args
            .values()
            .map(|arg| arg.get_component_expr())
            .flatten()
            .collect();
        if components_expr.is_empty() {
            spawn_bundle
        } else {
            quote! {
                #spawn_bundle.insert((#(#components_expr),*))
            }
        }
    }


    // pub fn entity_parent_expr(&self, dom_state: &mut DomState) -> Option<TokenStream> {
    //     if let Some(DomArg::If { .. }) = self.args.args.get(&DomArgKey::If) {
    //         let ident = dom_state.add_ui_state_parent_field(&self);
    //         Some(quote!(widget.#ident))
    //     } else {
    //         None
    //     }
    // }

    // pub fn generate(&self, output: &mut DomState) -> TokenStream {
    //     let mut bundle_state = BlockState::default();
    //     if let Bundle::Expr { expr, .. } = &self.bundle {
    //         parse_expr(expr, &mut bundle_state);
    //     }
    //     let if_instruction = if let Some(DomArg::Instruction(
    //         _,
    //         DomInstruction::If {
    //             expr: condition, ..
    //         },
    //     )) = self.args.args.get("@if")
    //     {
    //         Some(condition)
    //     } else {
    //         None
    //     };
    //     let for_instruction =
    //         if let Some(DomArg::Instruction(_, DomInstruction::For { pat, expr, .. })) =
    //             self.args.args.get("@for")
    //         {
    //             Some((pat, expr))
    //         } else {
    //             None
    //         };
    //     let key_instruction =
    //         if let Some(DomArg::Instruction(_, DomInstruction::Key { expr, ty, .. })) =
    //             self.args.args.get("@key")
    //         {
    //             Some((expr, ty))
    //         } else {
    //             None
    //         };

    //     let init_entity_expr = {
    //         let expr = &self.bundle;
    //         let init_expr = self
    //             .args
    //             .args
    //             .iter()
    //             .filter_map(|(_, arg)| {
    //                 if let DomArg::Component {
    //                     expr, component, ..
    //                 } = arg
    //                 {
    //                     Some(quote!({ let component: #component = #expr; component }))
    //                 } else {
    //                     None
    //                 }
    //             })
    //             .collect::<Vec<_>>();
    //         let mut spawn = match expr {
    //             Bundle::Expr {
    //                 expr: Expr::Tuple(inner),
    //                 ..
    //             } if inner.elems.is_empty() => {
    //                 quote!(commands.spawn_empty())
    //             }
    //             expr => {
    //                 quote!(commands.spawn(#expr))
    //             }
    //         };
    //         if !init_expr.is_empty() {
    //             spawn = quote!(#spawn.insert((#(#init_expr),*)));
    //         }
    //         quote! {#spawn.id()}
    //     };

    //     let mut need_update = false;
    //     let update_stats = {
    //         if let Some(DomArg::Instruction(_, DomInstruction::Id(_, _, _))) =
    //             self.args.args.get("@id")
    //         {
    //             need_update = true;
    //         }
    //         if for_instruction.is_some() {
    //             need_update = true;
    //         }
    //         let bundle_update_condition = bundle_state.generate_condition();
    //         let bundle_update_stats = if bundle_state.use_state.is_empty()
    //             && bundle_state.set_state.is_empty()
    //             && if_instruction.is_none()
    //         {
    //             quote!()
    //         } else {
    //             let bundle_expr = &self.bundle;
    //             let bundle_type = self
    //                 .end_tag
    //                 .as_ref()
    //                 .and_then(|end| end.end_bundle.as_ref())
    //                 .map(|ty| quote!(: #ty));
    //             need_update = true;
    //             quote! {
    //                 if #bundle_update_condition{
    //                     let bundle #bundle_type = #bundle_expr;
    //                     commands.entity(node_entity).insert(bundle);
    //                 }
    //             }
    //         };
    //         let mut update_stats = vec![bundle_update_stats];
    //         for arg in self.args.values() {
    //             if let DomArg::Component {
    //                 expr, component, ..
    //             } = arg
    //             {
    //                 let mut block_state = BlockState::default();
    //                 parse_expr(expr, &mut block_state);
    //                 if !block_state.use_state.is_empty()
    //                     || !block_state.set_state.is_empty()
    //                     || if_instruction.is_some()
    //                 {
    //                     need_update = true;
    //                     let component_expr = expr;
    //                     let condition = block_state.generate_condition();
    //                     update_stats.push(quote_spanned! {arg.span()=>
    //                         if #bundle_update_condition || #condition{
    //                             let component: #component = #component_expr;
    //                             commands.entity(node_entity).insert(component);
    //                         }
    //                     });
    //                 };
    //             }
    //         }
    //         quote! {#(#update_stats)*}
    //     };

    //     let mut dom_entity_field = None;

    //     let update_or_init_stat = if need_update {
    //         let dom_entity_field =
    //             dom_entity_field.get_or_insert_with(|| output.add_ui_state_field(self));

    //         let spawn_condition =
    //             quote!(not_inited || widget.#dom_entity_field == Entity::PLACEHOLDER);
    //         let despawn_state = generate_despawn(quote!(widget.#dom_entity_field));

    //         let calculate_enable_widget = if_instruction.as_ref().map(|if_condition| {
    //             let condition_expr = if_condition;
    //             let mut condition_block_state = BlockState::default();
    //             let condition_update_expr = condition_block_state.generate_condition();
    //             parse_expr(condition_expr, &mut condition_block_state);
    //             Some(quote! {
    //                 let enable_widget = if #condition_update_expr {
    //                     #condition_expr
    //                 } else {
    //                     enable_widget
    //                 };
    //             })
    //         });
    //         quote! {
    //             let not_inited = #spawn_condition;
    //             #calculate_enable_widget
    //             let node_entity = match (enable_widget,not_inited) {
    //                 (true,true) => {
    //                     let node_entity: Entity = {
    //                         #init_entity_expr
    //                     };
    //                     widget.#dom_entity_field = node_entity;
    //                     node_entity
    //                 },
    //                 (true,false) => {
    //                     let node_entity: Entity = widget.#dom_entity_field;
    //                     #update_stats
    //                     Entity::PLACEHOLDER
    //                 }
    //                 (false,false) => {
    //                     #despawn_state
    //                     widget.#dom_entity_field = Entity::PLACEHOLDER;
    //                     Entity::PLACEHOLDER
    //                 }
    //                 _=>{
    //                     Entity::PLACEHOLDER
    //                 }
    //             };
    //         }
    //     } else {
    //         quote! {
    //             let node_entity = if not_inited && enable_widget {
    //                 #init_entity_expr
    //             } else {
    //                 Entity::PLACEHOLDER
    //             };
    //         }
    //     };

    //     if let Some((patten, iterator)) = for_instruction {
    //         let dom_entity_field =
    //             dom_entity_field.get_or_insert_with(|| output.add_ui_state_field(self));
    //         let (sub_widget_query, sub_widget_type) = output.add_loop_state_query(self);
    //         let (key_expr, key_type) = if let Some((key_expr, key_type)) = key_instruction {
    //             (quote!(#key_expr), quote!(#key_type))
    //         } else {
    //             (quote!(index), quote!(usize))
    //         };
    //         let dom_entity_map_field = output.add_ui_state_map_field(self, &key_type);
    //         let despawn_disabled = generate_despawn(quote!(node_entity));
    //         let despawn_removed = generate_despawn(quote!(node_entity));

    //         let mut update_or_init_children_stat = None;
    //         {
    //             let mut sub_widget_fields = HashMap::new();
    //             let mut child_output = DomState {
    //                 widget_name: output.widget_name,
    //                 system_params: output.system_params,
    //                 items: output.items,
    //                 pre_build: output.pre_build,
    //                 widget_fields: &mut sub_widget_fields,
    //             };
    //             for child in self.children.iter().flat_map(|c| c.list.iter()) {
    //                 if update_or_init_children_stat.is_some() {
    //                     panic!("node with `for` instruction can only has one child");
    //                 }
    //                 update_or_init_children_stat = Some(child.generate(&mut child_output));
    //             }
    //             let sub_widget_field_decl = sub_widget_fields.values().map(|w| &w.0);
    //             let sub_widget_init = sub_widget_fields.values().map(|w| &w.1);
    //             output.items.push(quote! {
    //                 #[allow(non_snake_case)]
    //                 #[allow(unused_variables)]
    //                 #[derive(Component, Debug, Reflect)]
    //                 pub struct #sub_widget_type {
    //                     #(#sub_widget_field_decl),*
    //                 }
    //                 impl Default for #sub_widget_type {
    //                     fn default() -> Self{
    //                         Self{
    //                             #(#sub_widget_init),*
    //                         }
    //                     }
    //                 }
    //             });
    //         }
    //         quote_spanned! {self._lt0.span()=>
    //             {
    //                 #update_or_init_stat
    //                 let children_map = &mut widget.#dom_entity_map_field;
    //                 let mut new_children_map = bevy::utils::HashMap::<#key_type, Entity>::new();
    //                 let mut children = Vec::new();
    //                 for old_child in children_map.values() {
    //                     commands.entity(*old_child).remove_parent();
    //                 }
    //                 for (index,#patten) in Iterator::enumerate(#iterator) {
    //                     let key = #key_expr;
    //                     match (enable_widget,children_map.remove(&key)) {
    //                         (true,Some(node_entity)) => {
    //                             let not_inited = false;
    //                             if let Ok(mut widget) = #sub_widget_query.get_mut(node_entity) {
    //                                 #update_or_init_children_stat;
    //                             }
    //                             new_children_map.insert(key, node_entity);
    //                             children.push(node_entity);
    //                         },
    //                         (true,None) => {
    //                             let mut widget = #sub_widget_type::default();
    //                             let not_inited = true;
    //                             let node_entity: Entity = {
    //                                 #update_or_init_children_stat
    //                             };
    //                             new_children_map.insert(key, node_entity);
    //                             children.push(node_entity);
    //                             commands.entity(node_entity).insert(widget);
    //                         },
    //                         (false,Some(node_entity)) => {
    //                             #despawn_disabled
    //                         },
    //                         _=>{}
    //                     }
    //                 }
    //                 for old_child in children_map.values() {
    //                     #despawn_removed
    //                 }
    //                 for child in children.iter() {
    //                     commands.entity(*child).set_parent(widget.#dom_entity_field);
    //                 }
    //                 widget.#dom_entity_map_field = new_children_map;
    //                 node_entity
    //             }
    //         }
    //     } else {
    //         let mut update_or_init_children_stat = vec![];
    //         for child in self.children.iter().flat_map(|c| c.list.iter()) {
    //             let update_or_init_child = child.generate(output);
    //             let node_parent_entity = child.entity_parent_expr(output);
    //             let node_entity_expr = node_parent_entity
    //                 .as_ref()
    //                 .cloned()
    //                 .unwrap_or_else(|| quote!(node_entity));
    //             let update_parent_entity = node_parent_entity.as_ref().map(|parent| {
    //                 quote! {
    //                     if node_entity != Entity::PLACEHOLDER {
    //                         #parent = node_entity;
    //                     }
    //                 }
    //             });
    //             update_or_init_children_stat.push(quote! {
    //                 #update_parent_entity
    //                 let child_entity = #update_or_init_child;
    //                 if child_entity != Entity::PLACEHOLDER {
    //                     commands.entity(child_entity).set_parent(#node_entity_expr);
    //                 }
    //             });
    //         }
    //         quote! {
    //             {
    //                 #update_or_init_stat
    //                 #(#update_or_init_children_stat)*
    //                 node_entity
    //             }
    //         }
    //     }
    // }
}
