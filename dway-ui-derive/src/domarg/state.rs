use crate::{parser::ParseCodeResult, prelude::*};
use super::{DomArgKey, DomDecorator};

#[derive(Parse)]
pub struct UseState {
    #[call(Attribute::parse_outer)]
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    _col: Token![:],
    ty: Type,
    _eq: Option<Token![=]>,
    #[parse_if(_eq.is_some())]
    init: Option<Expr>,
    _before_update: Option<Token![<=]>,
    _before_update2: Option<Token![@]>,
    #[parse_if(_before_update.is_some()||_before_update2.is_some())]
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
            attrs,
            name,
            ty,
            init,
            ..
        } = self;
        let init = init
            .as_ref()
            .map(|e| e.to_token_stream())
            .unwrap_or_else(|| quote_spanned!(name.span()=> Default::default()));
        context.tree_context.state_builder.add_field_with_initer(
            name,
            quote! {#(#attrs)* #vis #name: #ty},
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
            let is_change = ParseCodeResult::from_expr(check_change).changed_bool();
            let field_mut = format_ident!("{}_mut", name, span = name.span());
            quote_spanned! {name.span()=>
                if #just_inited ||#is_change {
                    *state.#field_mut() = #check_change;
                }
            }
        });
        let field_changed = format_ident!("{}_is_changed", name, span = name.span());
        let on_change = on_change.as_ref().map(|on_change| {
            quote_spanned! {on_change.span()=>
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

#[derive(Parse)]
pub struct FieldWithIniter {
    #[call(Field::parse_named)]
    pub raw_field: Field,
    pub eq: Option<Token![=]>,
    #[parse_if(eq.is_some())]
    pub init: Option<Expr>,
}

#[derive(Parse)]
pub struct StructBrace {
    #[brace]
    pub _wrap: Brace,
    #[inside(_wrap)]
    #[call(Punctuated::parse_terminated)]
    pub fields: Punctuated<FieldWithIniter, Token![,]>,
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
                let raw_field = &field.raw_field;
                state_builder.add_field_with_initer(
                    field.raw_field.ident.as_ref().unwrap(),
                    quote!(#raw_field),
                    field
                        .init
                        .as_ref()
                        .map(|e| e.to_token_stream())
                        .unwrap_or_else(|| quote_spanned!(raw_field.span()=> Default::default())),
                );
            }
        }
    }
}

#[derive(Parse)]
pub struct StateReflect {}

impl DomDecorator for StateReflect {
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let span = context.dom_id.span();
        context
            .tree_context
            .state_builder
            .attributes
            .push(quote_spanned! {span=> #[derive(Reflect)]});
        let state_name = context.tree_context.state_builder.name.clone();
        context.tree_context.plugin_builder.stmts.push(quote_spanned! {state_name.span()=>
            app.register_type::<#state_name>();
        });
    }
}

#[derive(Parse)]
pub struct PropReflect {}

impl DomDecorator for PropReflect {
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let prop_name = format_ident!("{}", &context.tree_context.context.namespace);
        context.tree_context.plugin_builder.stmts.push(quote_spanned! {prop_name.span()=>
            app.register_type::<#prop_name>();
        });
    }
}
