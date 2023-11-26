pub mod callback;
pub mod control;
pub mod data;
pub mod relation;
pub mod state;
pub mod plugin;
pub mod ui;

use derive_syn_parse::Parse;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap},
};
use syn::{
    ext::IdentExt,
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{At, Brace, If, Paren, RArrow},
    *,
};

use crate::{
    dom::Dom,
    domcontext::{
        widget_context::{WidgetDomContext, WidgetNodeContext},
        DomContext,
    },
};

use self::{control::Id, data::InsertComponent, ui::Style};

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DomArgKey {
    AfterUpdate,
    BeforeUpdate,
    Id,
    Key,
    Component(String),
    Resource(String),
    State(String),
    StateComponent,
    BundleStructure,
    QueryComponent(String),
    Argument(String),
    Handle(String),
    System(String),
    WorldQuery(String),
    Plugin,
    For,
    If,
    Other(TypeId,String),
}

pub trait DomDecorator: Any {
    fn key(&self) -> DomArgKey;
    fn need_node_entity_field(&self) -> bool {
        false
    }
    fn need_sub_widget(&self) -> bool {
        false
    }
    fn update_context(&self, _context: &mut WidgetNodeContext) {}
    fn get_component(&self) -> Option<TokenStream> {
        None
    }
    fn generate_update(&self, _context: &mut WidgetNodeContext) -> Option<TokenStream> {
        None
    }
    fn wrap_sub_widget(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        inner
    }
    fn wrap_spawn_children(&self, inner: TokenStream, _context: &mut DomContext) -> TokenStream {
        inner
    }
    fn wrap_spawn(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        inner
    }
    fn wrap_update(&self, inner: TokenStream, _context: &mut WidgetNodeContext) -> TokenStream {
        inner
    }
    fn wrap_update_children(
        &self,
        child_ident: Option<Ident>,
        inner: TokenStream,
        _context: &mut WidgetNodeContext,
    ) -> TokenStream {
        inner
    }
}

pub struct DomArg {
    span: Span,
    pub inner: Box<dyn DomDecorator>,
}

impl DomArg {
    pub fn key(&self) -> DomArgKey {
        self.inner.key()
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn parse_map(input: ParseStream) -> syn::Result<BTreeMap<DomArgKey, Self>> {
        let mut map = BTreeMap::default();
        while input.peek(Token![@]) || input.peek(Ident) {
            let arg: Self = input.parse()?;
            let key = arg.key();
            map.insert(key, arg);
        }
        Ok(map)
    }

    pub fn parse_vec(input: ParseStream) -> syn::Result<Vec<DomArg>> {
        let mut vec = Vec::new();
        while input.peek(Token![@]) || input.peek(Ident) {
            let arg: Self = input.parse()?;
            vec.push(arg);
        }
        Ok(vec)
    }

    pub fn get_component_expr(&self) -> Option<TokenStream> {
        self.inner.get_component()
    }

    pub fn need_node_entity(&self) -> bool {
        self.inner.need_node_entity_field()
    }
}
impl syn::parse::Parse for DomArg {
    fn parse(input: ParseStream) -> Result<Self> {
        if !input.peek(Token![@]) {
            let content;
            let component = input.parse()?;
            let _: Token![=] = input.parse()?;
            if input.peek(Brace) {
                let _wrap = braced!(content in input);
            } else if input.peek(token::Bracket) {
                let _wrap = bracketed!(content in input);
            } else {
                let _wrap = parenthesized!(content in input);
            }
            let expr = content.parse()?;
            Ok(Self {
                span: content.span(),
                inner: Box::new(InsertComponent { component, expr }),
            })
        } else {
            let _: Token![@] = input.parse()?;
            let instruction: Ident = Ident::parse_any(input)?;
            Ok(match &*instruction.to_string() {
                "id" => {
                    let _: Token![=] = input.parse()?;
                    let id: LitStr = input.parse()?;
                    Self {
                        span: id.span(),
                        inner: Box::new(Id { id }),
                    }
                }
                "style" => {
                    let _: Token![=] = input.parse()?;
                    let inner: Style = input.parse()?;
                    Self {
                        span: inner.style.span(),
                        inner: Box::new(inner),
                    }
                }
                name => {
                    let content;
                    if input.peek(Brace) {
                        let _wrap = braced!(content in input);
                    } else if input.peek(token::Bracket) {
                        let _wrap = bracketed!(content in input);
                    } else {
                        let _wrap = parenthesized!(content in input);
                    }
                    let span = content.span();
                    let inner: Box<dyn DomDecorator> = match name {
                        "if" => Box::new(content.parse::<control::If>()?),
                        "for" => Box::new(content.parse::<control::For>()?),
                        "map" => Box::new(content.parse::<control::Map>()?),
                        "for_query" => Box::new(content.parse::<control::ForQuery>()?),
                        "arg" => Box::new(content.parse::<data::Argument>()?),
                        "conponent" => Box::new(content.parse::<data::QueryComponent>()?),
                        "query" => Box::new(content.parse::<data::Query>()?),
                        "query_many" => Box::new(content.parse::<data::QueryMany>()?),
                        "res" => Box::new(content.parse::<data::Res>()?),
                        "background_color" => Box::new(content.parse::<ui::BackgroundColor>()?),
                        "handle" => Box::new(content.parse::<ui::Handle>()?),
                        "callback" => Box::new(content.parse::<callback::Callback>()?),
                        "before" => Box::new(content.parse::<callback::BeforeUpdate>()?),
                        "after" => Box::new(content.parse::<callback::AfterUpdate>()?),
                        "use_state" => Box::new(content.parse::<state::UseState>()?),
                        "state_component" => Box::new(content.parse::<state::StateComponent>()?),
                        "bundle" => Box::new(content.parse::<state::BundleStructure>()?),
                        "plugin" => Box::new(content.parse::<plugin::Plugin>()?),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                quote_spanned! {span=> #instruction },
                                "unsupportde instruction",
                            ));
                        }
                    };
                    Self { span, inner }
                }
            })
        }
    }
}
