use crate::{parse_color_str, parser::ParseCodeResult, prelude::*};
use syn::LitStr;

use super::{DomArgKey, DomDecorator};

#[derive(Parse)]
pub struct Style {
    pub style: LitStr,
}

impl DomDecorator for Style {
    fn key(&self) -> super::DomArgKey {
        super::DomArgKey::Component("Style".to_string())
    }
    fn get_component(&self) -> Option<TokenStream> {
        Some(crate::style::generate(&self.style))
    }
}

#[derive(Parse)]
pub struct BackgroundColor {
    pub lit: LitStr,
}

impl DomDecorator for BackgroundColor {
    fn key(&self) -> super::DomArgKey {
        super::DomArgKey::Component("BackgroundColor".to_string())
    }
    fn get_component(&self) -> Option<TokenStream> {
        let lit = &self.lit;
        let Some([r, g, b, a]) = parse_color_str(&lit.value()) else {
            return Some(quote_spanned!(lit.span()=> compile_error!("failed to parse color")));
        };
        Some(quote_spanned!(lit.span()=> BackgroundColor(Color::rgba_u8(#r,#g,#b,#a))))
    }
}

#[derive(Parse)]
pub struct Handle {
    ty: Type,
    _col: Token![=>],
    expr: Expr,
}

impl DomDecorator for Handle {
    fn key(&self) -> super::DomArgKey {
        DomArgKey::Component(format!("Handle<{}>", self.ty.to_token_stream()))
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        let ty = &self.ty;
        let ident = format_ident!(
            "assets_{}",
            convert_type_name(&self.ty),
            span = self.ty.span()
        );
        context.tree_context.system_querys.insert(
            ident.to_string(),
            quote_spanned! {ident.span()=>
            #[allow(non_snake_case)]
            mut #ident: ResMut<Assets<#ty>>
            },
        );
    }
    fn need_node_entity_field(&self) -> bool {
        let component_state = ParseCodeResult::from_expr(&self.expr);
        !component_state.use_state.is_empty()
            || !component_state.set_state.is_empty()
            || !component_state.use_prop.is_empty()
    }
    fn get_component(&self) -> Option<TokenStream> {
        let Self { expr, .. } = self;
        let ident = format_ident!(
            "assets_{}",
            convert_type_name(&self.ty),
            span = self.ty.span()
        );
        Some(quote_spanned!(self._col.span()=> MaterialNode(#ident.add(#expr))))
    }
    fn generate_update(&self, context: &mut WidgetNodeContext) -> Option<TokenStream> {
        let Self { expr, .. } = self;
        let dependencies = ParseCodeResult::from_expr(expr);
        dependencies.is_changed().map(|check_changed| {
            let entity = &context.entity_var;
            let ident = format_ident!(
                "assets_{}",
                convert_type_name(&self.ty),
                span = self.ty.span()
            );
            quote_spanned! {expr.span()=>
                if #check_changed {
                    commands.entity(#entity).insert(MaterialNode(#ident.add(#expr)));
                }
            }
        })
    }
}
