use crate::{
    builder::{ComponentBuilder, PluginBuilder, ResourceBuilder},
    dom::Dom,
    domarg::DomArg,
    domcontext::Context,
    generate::BoolExpr,
    parser::check_stmts,
};
use convert_case::Casing;
use derive_syn_parse::Parse;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use std::collections::BTreeMap;
use syn::*;

use super::DomContext;

pub struct WidgetNodeContext<'l, 'w: 'l, 'g: 'w> {
    pub tree_context: &'l mut WidgetDomContext<'w, 'g>,
    pub dom: &'l Dom,
    pub dom_id: Ident,
    pub entity_var: Ident,
    pub parent_entity: Ident,
    pub just_inited: Ident,
    pub parent_just_inited: Ident,
}

pub struct WidgetDomContext<'l: 'g, 'g> {
    pub dom_context: &'l mut DomContext<'g>,
    pub state_builder: ComponentBuilder,
    pub widget_builder: ComponentBuilder,
    pub bundle_builder: ComponentBuilder,
    pub resources_builder: ResourceBuilder,
    pub plugin_builder: PluginBuilder,
    pub world_query: BTreeMap<String, (TokenStream, TokenStream)>,
    pub system_querys: BTreeMap<String, TokenStream>,
    pub state_namespace: String,
}

impl<'l, 'g> std::ops::DerefMut for WidgetDomContext<'l, 'g> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dom_context
    }
}

impl<'l, 'g> std::ops::Deref for WidgetDomContext<'l, 'g> {
    type Target = &'l mut DomContext<'g>;

    fn deref(&self) -> &Self::Target {
        &self.dom_context
    }
}

impl<'l, 'g> WidgetDomContext<'l, 'g> {
    pub fn generate(
        &mut self,
        dom: &'l Dom,
        parent_entity: &Ident,
        parent_just_inited: &Ident,
        export_entity: bool,
    ) -> (Ident, TokenStream) {
        self.push(dom);
        let node_context = self.top();
        let dom_id = node_context.dom_id.clone();

        let entity_var = node_context.get_var("_entity");
        let just_init_var = node_context.get_var("_just_inited");

        let need_node_entity = dom.args.iter().any(|arg| arg.need_node_entity());
        let (entity_expr, set_entity_stat) = if need_node_entity {
            let field = node_context.get_field("_entity");
            self.widget_builder.add_field_with_initer(
                &field,
                quote!(pub #field: Entity),
                quote!(Entity::PLACEHOLDER),
            );
            (
                quote!(widget.#field),
                Some(quote!(widget.#field = #entity_var)),
            )
        } else {
            (quote!(Entity::PLACEHOLDER), None)
        };

        let mut context = WidgetNodeContext {
            tree_context: self,
            dom,
            dom_id: dom_id.clone(),
            entity_var: entity_var.clone(),
            parent_entity: parent_entity.clone(),
            just_inited: just_init_var.clone(),
            parent_just_inited: parent_just_inited.clone(),
        };

        dom.args
            .iter()
            .for_each(|arg| arg.inner.update_context(&mut context));

        let set_entity_var_stmt = if export_entity {
            quote!()
        } else {
            quote! {
                #[allow(unused_variables)]
                let mut #entity_var = #entity_expr;
            }
        };

        let prepare_stat = quote! {
            #set_entity_var_stmt
            #[allow(unused_variables)]
            let mut #just_init_var = false;
        };

        let init_stat = {
            let spawn_expr = dom.generate_spawn();
            let init_stat = quote_spanned! {dom.span()=>
                #entity_var = #spawn_expr.set_parent(#parent_entity).id();
                #just_init_var = true;
                #set_entity_stat
            };
            let init_stat = dom.args.iter().fold(init_stat, |inner, arg| {
                arg.inner
                    .wrap_spawn(inner, context.tree_context.dom_context, true)
            });
            init_stat
        };

        let update_component_stat = dom
            .args
            .iter()
            .map(|arg| arg.inner.generate_update(&mut context))
            .collect::<Vec<_>>();
        let update_stat = if update_component_stat.is_empty() {
            None
        } else {
            Some(quote_spanned! {dom.span()=>
                #(#update_component_stat)*
            })
        };

        let process_node_stat = dom.args.iter().rev().fold(
            BoolExpr::RuntimeValue(quote!(#parent_just_inited ))
                .to_if_else(quote! { #init_stat }, update_stat.as_ref())
                .unwrap_or_default(),
            |stat, arg| arg.inner.wrap_update(stat, &mut context),
        );
        let process_node_stat = quote! {
            let (#entity_var,#just_init_var) = {
                #process_node_stat
                (#entity_var,#just_init_var)
            };
        };

        std::mem::drop(context);

        let (child_ident, spawn_children): (Option<Ident>, TokenStream) = if dom
            .args
            .iter()
            .any(|arg| arg.inner.need_sub_widget())
        {
            let lambda_var =
                DomContext::wrap_dom_id("__dway_ui_node_", &dom_id, "_subwidget_lambda");
            let sub_widget_state_type = format_ident!(
                "{}SubState{}",
                &self.namespace,
                dom_id,
                span = dom_id.span()
            );
            let sub_widget_type = format_ident!(
                "{}SubWidget{}",
                &self.namespace,
                dom_id,
                span = dom_id.span()
            );
            let sub_widget_query =
                format_ident!("sub_widget_{}_query", dom_id, span = dom_id.span());
            self.system_querys.insert(
                    sub_widget_query.to_string(),
                    quote!(mut #sub_widget_query: Query<( &mut #sub_widget_type,&mut #sub_widget_state_type )>),
                );

            let mut state_namespace = dom_id.to_string().to_case(convert_case::Case::Snake);
            let mut state_builder = ComponentBuilder::new(sub_widget_state_type.clone());
            state_builder.generate_init = true;
            state_builder
                .init
                .insert("__dway_changed_flags".to_string(), quote!(!0));
            state_builder.attributes.push(quote! {
                #[derive(Component)]
                #[dway_ui_derive::change_detact]
            });
            let mut widget_builder = ComponentBuilder::new(sub_widget_type.clone());
            widget_builder.attributes.push(quote!(
                #[derive(Component, Reflect, Debug)]
            ));
            widget_builder.generate_init = true;

            std::mem::swap(&mut state_namespace, &mut self.state_namespace);
            std::mem::swap(&mut state_builder, &mut self.state_builder);
            std::mem::swap(&mut widget_builder, &mut self.widget_builder);

            let mut context = WidgetNodeContext {
                tree_context: self,
                dom,
                dom_id: dom_id.clone(),
                entity_var: entity_var.clone(),
                parent_entity: parent_entity.clone(),
                just_inited: just_init_var.clone(),
                parent_just_inited: parent_just_inited.clone(),
            };
            dom.args
                .iter()
                .for_each(|arg| arg.inner.update_sub_widget_context(&mut context));

            let spawn_children = dom
                .children
                .iter()
                .flat_map(|c| c.list.iter())
                .map(|child| self.generate(child, &entity_var, &just_init_var, true))
                .collect::<Vec<_>>();

            std::mem::swap(&mut state_namespace, &mut self.state_namespace);
            std::mem::swap(&mut state_builder, &mut self.state_builder);
            std::mem::swap(&mut widget_builder, &mut self.widget_builder);
            let widget_name = &widget_builder.name;
            self.plugin_builder.stmts.push(quote! {
                app.register_type::<#widget_name>();
            });
            self.plugin_builder.components.push(state_builder);
            self.plugin_builder.components.push(widget_builder);

            let ident = spawn_children.first().map(|f| f.0.clone());
            let spawn_children = spawn_children
                .iter()
                .map(|(_key, value)| value)
                .collect::<Vec<_>>();
            let spawn_children = quote! {
                #(#spawn_children)*
            };

            let mut context = WidgetNodeContext {
                tree_context: self,
                dom,
                dom_id: dom_id.clone(),
                entity_var: entity_var.clone(),
                parent_entity: parent_entity.clone(),
                just_inited: just_init_var.clone(),
                parent_just_inited: parent_just_inited.clone(),
            };
            let spawn_children = dom.args.iter().rev().fold(spawn_children, |stat, arg| {
                arg.inner.wrap_sub_widget(stat, &mut context)
            });

            let spawn_children = quote! {
                let mut #lambda_var = |mut #ident, widget: &mut #sub_widget_type, state:&mut #sub_widget_state_type| {
                    #spawn_children
                    #ident
                };
                let #ident = if let Ok((mut widget,mut state)) = #sub_widget_query.get_mut(#ident) {
                    let #ident = #lambda_var(#ident, &mut widget, &mut state);
                    state.clear_marks();
                    #ident
                }else{
                    let mut state = #sub_widget_state_type::default();
                    let mut widget = #sub_widget_type::default();
                    let #ident = #lambda_var(#ident, &mut widget, &mut state);
                    state.clear_marks();
                    commands.entity(#ident).insert((state,widget));
                    #ident
                };
            };
            (ident, spawn_children)
        } else {
            let spawn_children = dom
                .children
                .iter()
                .flat_map(|c| c.list.iter())
                .map(|child| self.generate(child, &entity_var, &just_init_var, false))
                .collect::<Vec<_>>();

            let ident = spawn_children.first().map(|f| f.0.clone());
            let spawn_children = spawn_children
                .iter()
                .map(|(_key, value)| value)
                .collect::<Vec<_>>();
            (ident, quote!( #(#spawn_children)* ))
        };

        let mut context = WidgetNodeContext {
            tree_context: self,
            dom,
            dom_id: dom_id.clone(),
            entity_var: entity_var.clone(),
            parent_entity: parent_entity.clone(),
            just_inited: just_init_var.clone(),
            parent_just_inited: parent_just_inited.clone(),
        };

        let spawn_children = dom.args.iter().rev().fold(spawn_children, |stat, arg| {
            arg.inner
                .wrap_update_children(child_ident.clone(), stat, &mut context)
        });
        self.pop();

        (
            entity_var,
            check_stmts(quote_spanned! {dom.span()=>
                #prepare_stat
                #process_node_stat
                #spawn_children
            }),
        )
    }
}

#[derive(Parse)]
pub struct WidgetDeclare {
    pub name: Ident,
    #[prefix(Token![=>])]
    #[call(DomArg::parse_vec)]
    pub args: Vec<DomArg>,
    #[call(Dom::parse_vec)]
    pub dom: Vec<Dom>,
}

pub fn generate(decl: &WidgetDeclare) -> PluginBuilder {
    let WidgetDeclare { name, args, dom } = decl;
    let state_name = format_ident!("{}State", name, span = name.span());
    let widget_name = format_ident!("{}Widget", name, span = name.span());
    let resource_name = format_ident!("{}Resource", name, span = name.span());
    let bundle_name = format_ident!("{}Bundle", name, span = name.span());
    let plugin_name = format_ident!("{}Plugin", name, span = name.span());
    let systems_name = format_ident!("{}Systems", name, span = name.span());
    let system_name = format_ident!(
        "{}_render",
        name.to_string().to_case(convert_case::Case::Snake),
        span = name.span()
    );
    let parent_just_inited = format_ident!("not_inited");
    let parent_entity = format_ident!("this_entity");
    let mut root_context = Context::default();
    root_context.namespace = name.to_string();
    let mut dom_context = DomContext::new(&mut root_context);

    let mut context = WidgetDomContext {
        state_builder: ComponentBuilder::new(state_name.clone()),
        widget_builder: ComponentBuilder::new(widget_name.clone()),
        bundle_builder: ComponentBuilder::new(bundle_name),
        resources_builder: ResourceBuilder::new(resource_name),
        plugin_builder: PluginBuilder::new(plugin_name),
        dom_context: &mut dom_context,
        world_query: Default::default(),
        system_querys: Default::default(),
        state_namespace: "root".to_string(),
    };

    context.state_builder.generate_init = true;
    context
        .state_builder
        .init
        .insert("__dway_changed_flags".to_string(), quote!(!0));
    context.state_builder.attributes.push(quote! {
        #[derive(Component)]
        #[dway_ui_derive::change_detact]
    });

    context.widget_builder.attributes.push(quote! {
        #[derive(Component, Reflect)]
    });
    context.widget_builder.add_field_with_initer(
        &format_ident!("inited"),
        quote! {pub inited: bool},
        quote! {false},
    );
    context.widget_builder.generate_init = true;

    context.plugin_builder.other_items.push(quote! {});
    context.plugin_builder.other_items.push(quote! {
        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        pub enum #systems_name {
            Render
        }
    });

    #[cfg(feature="css")]
    {
        let css_name = name.to_string().to_case(convert_case::Case::Kebab);
        context.plugin_builder.stmts.push(quote! {
            bevy_ecss::RegisterComponentSelector::register_component_selector::<#name>(app, #css_name);
        });
    }

    context.plugin_builder.stmts.push(quote! {
        app.add_systems(Update, #system_name
            .run_if(|query:Query<(),With<#name>>|{!query.is_empty()})
            .in_set(#systems_name::Render));
    });

    context.resources_builder.generate_init = true;

    let mut before_foreach = Vec::new();

    let output = {
        let outputs = dom.iter().map(|dom| {
            context
                .generate(dom, &parent_entity, &parent_just_inited, false)
                .1
        });
        let mut output = quote!(#(#outputs)*);
        let entity_var = format_ident!("this_entity");
        let dom_id = format_ident!("_root");
        let mut node_context = WidgetNodeContext {
            tree_context: &mut context,
            dom: &dom[0],
            dom_id,
            entity_var,
            parent_entity: parent_entity.clone(),
            just_inited: parent_just_inited.clone(),
            parent_just_inited: parent_just_inited.clone(),
        };
        for arg in args.iter().rev() {
            arg.inner.update_context(&mut node_context);
        }
        let mut update = quote!();
        for arg in args.iter().rev() {
            update = arg.inner.wrap_update(update, &mut node_context);
            output = arg
                .inner
                .wrap_update_children(None, output, &mut node_context);
            if let Some(tokens) = arg.inner.before_foreach(&mut node_context){
                before_foreach.push(tokens);
            }
        }
        quote!( #update #output )
    };

    let state_name = context.state_builder.name.clone();
    let widget_name = context.widget_builder.name.clone();

    context
        .bundle_builder
        .attributes
        .push(quote!(#[derive(Bundle)]));

    context.bundle_builder.generate_init = true;
    context.bundle_builder.add_field_with_initer(
        &format_ident!("state"),
        quote!(pub state: #state_name),
        quote!(Default::default()),
    );
    context.bundle_builder.add_field_with_initer(
        &format_ident!("widget"),
        quote!(pub widget: #widget_name),
        quote!(Default::default()),
    );
    context
        .bundle_builder
        .add_field(&format_ident!("prop"), quote!(pub prop: #name));

    { 
        let state_name = &context.state_builder.name;
        let bundle_name = &context.bundle_builder.name;
        context.plugin_builder.other_items.push(quote!{
            impl From<#name> for #bundle_name {
                fn from(prop: #name) -> Self {
                    Self {
                        prop,
                        ..Default::default()
                    }
                }
            }

            impl #bundle_name {
                pub fn from_prop(prop: #name) -> Self {
                    Self {
                        prop,
                        ..Default::default()
                    }
                }

                pub fn from_prop_state(prop: #name, state: #state_name) -> Self {
                    Self {
                        prop,
                        state,
                        ..Default::default()
                    }
                }
            }
        }); 
    }

    context
        .plugin_builder
        .stmts
        .push(quote! {app.register_type::<#widget_name>();});
    context.plugin_builder.components.extend([
        context.state_builder,
        context.widget_builder,
        context.bundle_builder,
    ]);
    if !context.resources_builder.fields.is_empty() {
        context
            .resources_builder
            .attributes
            .push(quote!(#[derive(Resource)]));
        context
            .plugin_builder
            .resources
            .push(context.resources_builder);
    }

    let mut plugin_builder = context.plugin_builder;

    let system_args = context.system_querys.values().cloned().collect::<Vec<_>>();
    let world_query = context.world_query;
    let this_query = world_query.values().map(|(_, ty)| ty);
    let this_query_var = world_query.values().map(|(pat, _)| pat);
    let system = quote! {
        #[allow(unused_braces)]
        #[allow(non_snake_case)]
        pub fn #system_name(
            mut this_query: Query<(
                Entity,
                Ref<#name>,
                &mut #state_name,
                &mut #widget_name,
                #(#this_query),*
            )>,
            mut __dway_ui_commands: Commands,
            #(#system_args),*
        ) {
            let commands = &mut __dway_ui_commands;
            #(#before_foreach)*
            for (
                this_entity,
                prop,
                mut state,
                mut widget,
                #(#this_query_var),*
            ) in this_query.iter_mut() {
                let __dway_prop_changed = prop.is_changed();
                let #parent_just_inited = !widget.inited;
                #output
                if #parent_just_inited {
                    widget.inited = true;
                }
                state.clear_marks();
            }
        }
    };

    plugin_builder.systems.push(system);
    plugin_builder
}
