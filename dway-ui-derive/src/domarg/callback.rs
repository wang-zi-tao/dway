use super::{DomArgKey, DomDecorator};
use crate::prelude::*;

#[derive(Parse)]
pub struct CallbackSig {
    #[bracket]
    pub wrap: Bracket,
    #[inside(wrap)]
    pub args: Type,
}

#[derive(Parse)]
pub struct AddCallback {
    #[peek(syn::token::Bracket)]
    pub arg_type: Option<CallbackSig>,
    pub vis: Visibility,
    pub func: Ident,
}

impl DomDecorator for AddCallback {
    fn key(&self) -> DomArgKey {
        DomArgKey::System(self.func.to_token_stream().to_string())
    }

    fn update_context(&self, context: &mut WidgetNodeContext) {
        let vis = &self.vis;
        let name = &self.func;
        let ty = self
            .arg_type
            .as_ref()
            .map(|a| a.args.to_token_stream())
            .unwrap_or_else(|| quote!(()));
        context
            .tree_context
            .resources_builder
            .add_field(name, quote!(#vis #name: bevy::ecs::system::SystemId<#ty>));
        context
            .tree_context
            .resources_builder
            .add_field_with_initer(
                name,
                quote!(#vis #name: bevy::ecs::system::SystemId<#ty>),
                quote!(app.world_mut().register_system(#name)),
            );
        let resources_name = &context.tree_context.resources_builder.name;
        context.tree_context.system_querys.insert(
            "resources".to_string(),
            quote! {
                resources: Res<#resources_name>
            },
        );
    }

    fn wrap_update(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        let name = &self.func;
        quote_spanned! {name.span()=>
            let #name = resources.#name;
            #inner
        }
    }
}

#[derive(Parse)]
pub struct Callback {
    #[peek(syn::token::Bracket)]
    pub arg_type: Option<CallbackSig>,
    func: ItemFn,
}
impl DomDecorator for Callback {
    fn key(&self) -> super::DomArgKey {
        DomArgKey::System(self.func.sig.ident.to_string())
    }

    fn update_context(&self, context: &mut WidgetNodeContext) {
        let vis = &self.func.vis;
        let name = &self.func.sig.ident;
        let ty = self
            .arg_type
            .as_ref()
            .map(|a| a.args.to_token_stream())
            .unwrap_or_else(|| quote!(()));
        context
            .tree_context
            .resources_builder
            .add_field(name, quote!(#vis #name: bevy::ecs::system::SystemId<#ty>));
        context
            .tree_context
            .resources_builder
            .add_field_with_initer(
                name,
                quote!(#vis #name: bevy::ecs::system::SystemId<#ty>),
                quote!(app.world_mut().register_system(#name)),
            );
        let resources_name = &context.tree_context.resources_builder.name;
        context
            .tree_context
            .plugin_builder
            .systems
            .push(self.func.to_token_stream());
        context.tree_context.system_querys.insert(
            "resources".to_string(),
            quote! {
                resources: Res<#resources_name>
            },
        );
    }

    fn wrap_update(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        let name = &self.func.sig.ident;
        quote! {
            let #name = resources.#name;
            #inner
        }
    }
}

pub struct OnEvent {
    pub system: Ident,
    pub target: OnEventTarget,
}

pub enum OnEventTarget {
    ThisWidget,
    Self_,
    Entity { expr: Expr },
}

impl syn::parse::Parse for OnEvent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let system: Ident = input.parse()?;
        let target = if input.peek(Token![->]) {
            let _split: Token![->] = input.parse()?;
            if input.peek(Token![self]) {
                let _this: Token![self] = input.parse()?;
                OnEventTarget::Self_
            } else {
                OnEventTarget::Entity {
                    expr: input.parse()?,
                }
            }
        } else {
            OnEventTarget::ThisWidget
        };
        Ok(Self { system, target })
    }
}

impl DomDecorator for OnEvent {
    fn get_component(&self, context: &mut DomContext) -> Option<TokenStream> {
        let system = &self.system;
        let add_callback = match &self.target {
            OnEventTarget::ThisWidget => {
                let is_sub_widget = context
                    .dom_stack
                    .last()
                    .map(|c| c.is_sub_tree)
                    .unwrap_or(false);
                if is_sub_widget {
                    quote_spanned! {self.system.span()=>
                        with_system_to_this(#system)
                    }
                } else {
                    quote_spanned! {self.system.span()=>
                        with_system(widget_entity, #system)
                    }
                }
            }
            OnEventTarget::Self_ => quote_spanned! {self.system.span()=>
                with_system_to_this(#system)
            },
            OnEventTarget::Entity { expr } => quote_spanned! {self.system.span()=>
                with_system(#expr, #system)
            },
        };
        Some(quote_spanned! {self.system.span()=>
            dway_ui_framework::event::EventDispatcher::default().#add_callback
        })
    }
}

#[derive(Parse)]
pub struct BeforeUpdate {
    #[call(Block::parse_within)]
    pub stmts: Vec<Stmt>,
}

impl DomDecorator for BeforeUpdate {
    fn key(&self) -> DomArgKey {
        DomArgKey::BeforeUpdate
    }

    fn wrap_update(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        let stmts = &self.stmts;
        quote! {
            #(#stmts)*
            #inner
        }
    }
}

#[derive(Parse)]
pub struct AfterUpdate {
    #[call(Block::parse_within)]
    pub stmts: Vec<Stmt>,
}

impl DomDecorator for AfterUpdate {
    fn key(&self) -> DomArgKey {
        DomArgKey::AfterUpdate
    }

    fn wrap_update(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        let stmts = &self.stmts;
        quote! {
            #inner
            #(#stmts)*
        }
    }
}

#[derive(Parse)]
pub struct First {
    #[call(Block::parse_within)]
    pub stmts: Vec<Stmt>,
}

impl DomDecorator for First {
    fn before_foreach(&self, _context: &mut WidgetNodeContext) -> Option<TokenStream> {
        let Self { stmts } = self;
        Some(quote! {
            #(#stmts)*
        })
    }
}
