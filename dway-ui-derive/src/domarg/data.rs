use syn_derive::ToTokens;

use crate::{parser::ParseCodeResult, prelude::*};

use super::{DomArgKey, DomDecorator};

pub struct InsertComponent {
    pub component: Type,
    pub expr: Expr,
}
impl DomDecorator for InsertComponent {
    fn key(&self) -> DomArgKey {
        DomArgKey::Component(self.component.to_token_stream().to_string())
    }
    fn need_node_entity_field(&self) -> bool {
        let component_state = ParseCodeResult::from_expr(&self.expr);
        !component_state.use_state.is_empty() || !component_state.set_state.is_empty()
    }
    fn get_component(&self) -> Option<TokenStream> {
        let Self { component, expr } = self;
        Some(quote! {
            {let value: #component = #expr;value}
        })
    }
    fn generate_update(&self, context: &mut WidgetNodeContext) -> Option<TokenStream> {
        let Self {
            expr, component, ..
        } = self;
        let entity = &context.entity_var;
        let dependencies = ParseCodeResult::from_expr(expr);
        dependencies.is_changed().map(|check_changed| {
            quote! {
                if #check_changed {
                    commands.entity(#entity).insert({let value: #component = #expr;value});
                }
            }
        })
    }
}

#[derive(Parse)]
pub struct Argument {
    mutable: Option<Token![mut]>,
    name: Ident,
    #[prefix(Token!(:))]
    ty: Type,
    _after_change: Option<Token![=>]>,
    #[parse_if(_after_change.is_some())]
    block: Option<Block>,
}
impl DomDecorator for Argument {
    fn key(&self) -> DomArgKey {
        DomArgKey::Argument(self.name.to_string())
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            mutable, name, ty, ..
        } = self;
        context
            .tree_context
            .system_querys
            .insert(self.name.to_string(), quote!(#mutable #name: #ty));
    }
    fn wrap_update(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self { name, block, .. } = self;
        let WidgetNodeContext {
            parent_just_inited, ..
        } = context;
        let chech_change = block.as_ref().map(|b| {
            quote! {
                if #parent_just_inited || #name.is_changed() {
                    #b
                }
            }
        });
        quote! {
            #chech_change
            #inner
        }
    }
}

#[derive(Parse, ToTokens)]
pub enum WorldQueryType {
    #[peek(Ident, name = "Entity")]
    Entity(Ident),
    #[peek(And, name = "&")]
    Ref {
        _ref: Token![&],
        is_mut: Option<Token![mut]>,
        ty: Type,
    },
}

#[derive(Parse)]
pub enum QueryInner {
    #[peek(Lt, name = "Entity")]
    WithoutFilter {
        _s: Token![<],
        #[paren]
        _wrap: Paren,
        #[inside(_wrap)]
        #[call(Punctuated::parse_terminated)]
        world_query: Punctuated<WorldQueryType, Token![,]>,
        _filter_start: Token![,],
        filter: Type,
        _e: Token![>],
    },
    #[peek(Paren, name = "Entity")]
    WIthFilter {
        #[paren]
        _wrap: Paren,
        #[inside(_wrap)]
        #[call(Punctuated::parse_terminated)]
        world_query: Punctuated<WorldQueryType, Token![,]>,
    },
}

#[derive(Parse)]
pub struct QueryComponent {
    mutable: Option<Token![mut]>,
    query_name: Ident,
    _col: Token![<-],
    ty: Type,
    #[bracket]
    _wrap: Bracket,
    #[inside(_wrap)]
    entity: Expr,
    _after_change: Token![->],
    block: Block,
}
impl DomDecorator for QueryComponent {
    fn key(&self) -> DomArgKey {
        DomArgKey::QueryComponent(self.query_name.to_string())
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            query_name,
            ty,
            mutable,
            ..
        } = self;
        let arg_name = format_ident!("query_{}", query_name, span = query_name.span());
        context
            .tree_context
            .system_querys
            .insert(arg_name.to_string(), quote!(#mutable #arg_name: #ty));
    }
    fn wrap_update(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self {
            query_name,
            mutable,
            entity,
            block,
            ..
        } = self;
        let arg_name = format_ident!("query_{}", query_name, span = query_name.span());
        let query = if mutable.is_none() {
            quote!(#arg_name.get(#entity))
        } else {
            quote!(#arg_name.get_mut(#entity))
        };
        let parent_just_inited = &context.parent_just_inited;
        quote! {
            let mut #query_name = #query;
            if let Ok(#query_name) = &mut #query_name {
                if #parent_just_inited || #query_name.is_changed() {
                   #block
                }
            }
            #inner
        }
    }
}

#[derive(Parse)]
pub struct Res {
    mutable: Option<Token![mut]>,
    name: Ident,
    _col: Token![:],
    ty: Type,
    _after_change: Option<Token![->]>,
    #[parse_if(_after_change.is_some())]
    on_change: Option<Block>,
}

impl DomDecorator for Res {
    fn key(&self) -> super::DomArgKey {
        DomArgKey::Resource(self.name.to_string())
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            name, ty, mutable, ..
        } = self;
        context
            .tree_context
            .system_querys
            .insert(name.to_string(), quote!(#mutable #name: #ty));
    }
    fn wrap_update(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self {
            name, on_change, ..
        } = self;
        if on_change.is_some() {
            let just_inited = &context.just_inited;
            quote! {
                if #just_inited || #name.is_changed() {
                    #on_change
                }
                #inner
            }
        } else {
            inner
        }
    }
}

#[derive(Parse)]
pub struct QueryMany {
    mutable: Option<Token![mut]>,
    query_name: Ident,
    _split: Token![<-],
    ty: Type,
    #[bracket]
    _wrap: Bracket,
    #[inside(_wrap)]
    entity: Expr,
    _after_change: Token![->],
    block: Block,
}
impl DomDecorator for QueryMany {
    fn key(&self) -> DomArgKey {
        DomArgKey::QueryComponent(self.query_name.to_string())
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            query_name,
            ty,
            mutable,
            ..
        } = self;
        let arg_name = format_ident!("query_{}", query_name, span = query_name.span());
        context
            .tree_context
            .system_querys
            .insert(arg_name.to_string(), quote!(#mutable #arg_name: #ty));
    }
    fn wrap_update(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self {
            query_name,
            mutable,
            entity,
            block,
            ..
        } = self;
        let arg_name = format_ident!("query_{}", query_name, span = query_name.span());
        let query = if mutable.is_none() {
            quote!(#arg_name.iter_many(#entity))
        } else {
            quote!(#arg_name.iter_many_mut(#entity))
        };
        let parent_just_inited = &context.parent_just_inited;
        quote! {
            let mut #query_name = #query;
            if let Ok(#query_name) = &mut #query_name {
                if #parent_just_inited || #query_name.is_changed() {
                   #block
                }
            }
            #inner
        }
    }
}

#[derive(Parse)]
pub struct Query {
    mutable: Option<Token![mut]>,
    query_name: Ident,
    _split: Token![:],
    #[paren]
    _wrap_pat: Paren,
    #[inside(_wrap_pat)]
    #[call(Punctuated::parse_terminated)]
    idents: Punctuated<Ident, Token![,]>,
    _split1: Token![<-],
    ty: Type,
    #[bracket]
    _wrap: Bracket,
    #[inside(_wrap)]
    entity: Expr,
    _after_change: Token![->],
    block: Block,
}

impl DomDecorator for Query {
    fn key(&self) -> DomArgKey {
        DomArgKey::QueryComponent(self.query_name.to_string())
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            query_name,
            ty,
            mutable,
            ..
        } = self;
        let arg_name = format_ident!("query_{}", query_name, span = query_name.span());
        context
            .tree_context
            .system_querys
            .insert(arg_name.to_string(), quote!(#mutable #arg_name: #ty));
    }
    fn wrap_update(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        let Self {
            query_name,
            mutable,
            entity,
            block,
            idents,
            ..
        } = self;
        let arg_name = format_ident!("query_{}", query_name, span = query_name.span());
        let query = if mutable.is_none() {
            quote!(#arg_name.get(#entity))
        } else {
            quote!(#arg_name.get_mut(#entity))
        };
        let idents = idents.iter();
        quote! {
            if let Ok((#(mut #idents),*)) = #query {
                #block
            }
            #inner
        }
    }
}
