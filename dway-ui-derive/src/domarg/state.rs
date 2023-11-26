use crate::{parser::ParseCodeResult, prelude::*};

use super::{DomArgKey, DomDecorator};

#[derive(Parse)]
pub struct UseState {
    vis: Token![pub],
    name: Ident,
    _col: Token![:],
    ty: Type,
    _eq: Option<Token![=]>,
    #[parse_if(_eq.is_some())]
    init: Option<Expr>,
    _before_update: Option<Token![<=]>,
    #[parse_if(_before_update.is_some())]
    check_change: Option<Expr>,
    _after_change: Option<Token![=>]>,
    #[parse_if(_after_change.is_some())]
    on_change: Option<Block>,
}

impl DomDecorator for UseState {
    fn key(&self) -> DomArgKey {
        DomArgKey::State(self.name.to_string())
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let Self {
            vis,
            name,
            ty,
            init,
            ..
        } = self;
        let init = init
            .as_ref()
            .map(|e| e.to_token_stream())
            .unwrap_or_else(|| quote!(Default::default()));
        context.tree_context.state_builder.add_field_with_initer(
            name,
            quote! {#vis #name: #ty},
            quote! {#init},
        );
    }
    fn wrap_update(&self, inner: TokenStream, context: &mut WidgetNodeContext) -> TokenStream {
        let Self {
            name,
            check_change,
            on_change,
            ..
        } = self;
        let just_inited = &context.parent_just_inited;
        let check_change = check_change.as_ref().map(|check_change| {
            let is_change = ParseCodeResult::from_expr(&check_change).changed_bool();
            let field_mut = format_ident!("{}_mut", name, span = name.span());
            quote! {
                if #just_inited ||#is_change {
                    *state.#field_mut() = #check_change;
                }
            }
        });
        let field_changed = format_ident!("{}_is_changed", name, span = name.span());
        let on_change = on_change.as_ref().map(|on_change| {
            quote! {
                if #just_inited || state.#field_changed() {
                    #on_change
                }
            }
        });
        quote! {
            #check_change
            #on_change
            #inner
        }
    }
}

fn parse_fields(input: ParseStream<'_>) -> Result<Punctuated<Field, Token![,]>> {
    input.parse_terminated(Field::parse_named, Token![,])
}

#[derive(Parse)]
pub struct StructBrace {
    #[brace]
    pub _wrap: Brace,
    #[inside(_wrap)]
    #[call(parse_fields)]
    pub fields: Punctuated<Field, Token![,]>,
}

#[derive(Parse)]
pub struct StateComponent {
    #[call(Attribute::parse_outer)]
    pub attr: Vec<Attribute>,
    pub structure: Option<Token![struct]>,
    pub name: Option<Ident>,
    #[peek(Brace)]
    pub fields: Option<StructBrace>,
}

impl DomDecorator for StateComponent {
    fn key(&self) -> DomArgKey {
        DomArgKey::StateComponent
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let state_builder = &mut context.tree_context.state_builder;
        state_builder
            .attributes
            .extend(self.attr.iter().map(|a| a.to_token_stream()));
        if let Some(name) = &self.name {
            state_builder.name = name.clone();
        }
        if let Some(fields) = &self.fields {
            for field in &fields.fields {
                state_builder.add_field_with_initer(
                    field.ident.as_ref().unwrap(),
                    quote!(#field),
                    quote!(Default::default()),
                );
            }
        }
    }
}

#[derive(Parse)]
pub struct BundleStructure {
    #[call(Attribute::parse_outer)]
    pub attr: Vec<Attribute>,
    pub structure: Option<Token![struct]>,
    pub name: Option<Ident>,
    #[peek(Brace)]
    pub fields: Option<StructBrace>,
}

impl DomDecorator for BundleStructure {
    fn key(&self) -> DomArgKey {
        DomArgKey::BundleStructure
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let bundle_builder = &mut context.tree_context.bundle_builder;
        bundle_builder
            .attributes
            .extend(self.attr.iter().map(|a| a.to_token_stream()));
        if let Some(name) = &self.name {
            bundle_builder.name = name.clone();
        }
        if let Some(fields) = &self.fields {
            for field in &fields.fields {
                bundle_builder.add_field_with_initer(
                    field.ident.as_ref().unwrap(),
                    quote!(#field),
                    quote!(Default::default()),
                );
            }
        }
    }
}
