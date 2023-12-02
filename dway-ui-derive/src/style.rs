use std::str::FromStr;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::LitStr;

fn parse_val(prefix: &str, style: &str) -> TokenStream {
    let style = &style.replace(prefix, "");
    if style == "fill" {
        quote!(Val::Percent(100.0))
    } else if style == "auto" {
        quote!(Val::Auto)
    } else if style.contains('%') {
        let style = &style.replace('%', "");
        let Ok(value) = style.parse::<f32>() else {
            let message = format!("invalid value: {style:?}");
            return quote!(compile_error!(#message));
        };
        quote!(Val::Percent(#value))
    } else {
        let Ok(value) = style.parse::<f32>() else {
            let message = format!("invalid value: {style:?}");
            return quote!(compile_error!(#message));
        };
        quote!(Val::Px(#value))
    }
}

fn parse_field_value(prefix: &str, style: &str, field: &str) -> TokenStream {
    let ident = format_ident!("{}", field);
    let expr = parse_val(prefix, style);
    quote!(#ident: #expr)
}

fn parse_field_rect(prefix: &str, style: &str, field: &str) -> TokenStream {
    let ident = format_ident!("{}", field);
    let expr = parse_val(prefix, style);
    quote!(#ident: UiRect::all(#expr))
}

fn parse_align(prefix: &str, style: &str, field: &str, ty: &str) -> TokenStream {
    let ident = format_ident!("{}", field);
    let ty = format_ident!("{}", ty);
    let style = &*style.replace(prefix, "");
    let variant = match style {
        "default" | "start" | "end" | "flex-start" | "flex-end" | "center" | "baseline"
        | "stretch" | "space-between" | "space-evenly" | "space-around" => {
            let member = format_ident!("{}", style.to_case(Case::Pascal));
            quote!(#member)
        }
        _ => {
            let message = format!("invalid value: {style:?}");
            quote!(compile_error!(#message))
        }
    };
    quote!(#ident: #ty::#variant)
}

pub fn generate(input: &LitStr) -> TokenStream {
    let mut fields = vec![];
    for component in input.value().split(' ') {
        let tokens = match component {
            "w-full" => quote!(width:Val::Percent(100.0)),
            "h-full" => quote!(height:Val::Percent(100.0)),
            "full" => quote!(width:Val::Percent(100.0),height:Val::Percent(100.0)),
            "absolute" => quote!(position_type:PositionType::Absolute),
            "flex-row" => quote!(flex_direction:FlexDirection::Row),
            "flex-row-rev" => quote!(flex_direction:FlexDirection::RowReverse),
            "flex-col" => quote!(flex_direction:FlexDirection::Column),
            "flex-col-rev" => quote!(flex_direction:FlexDirection::ColumnReverse),
            "items-center" => quote!(align_items:AlignItems::Center),
            "align-center" => quote!(align_self:AlignSelf::Center),
            "justify-center" => quote!(justify_content:JustifyContent::Center),
            o if o.starts_with("align-items:") => {
                parse_align("align-items:", o, "align_items", "AlignItems")
            }
            o if o.starts_with("justify-items:") => {
                parse_align("justify-items:", o, "justify_items", "JustifyItems")
            }
            o if o.starts_with("align-self:") => {
                parse_align("align-self:", o, "align_self", "AlignSelf")
            }
            o if o.starts_with("justify-self:") => {
                parse_align("justify-self:", o, "justify_self", "JustifySelf")
            }
            o if o.starts_with("align-content:") => {
                parse_align("align-content:", o, "align_content", "AlignContent")
            }
            o if o.starts_with("justify-content:") => {
                parse_align("justify-content:", o, "justify_content", "JustifyContent")
            }
            o if o.starts_with("w-") => parse_field_value("w-", o, "width"),
            o if o.starts_with("h-") => parse_field_value("h-", o, "height"),
            o if o.starts_with("min-w-") => parse_field_value("min-w-", o, "min_width"),
            o if o.starts_with("min-h-") => parse_field_value("min-h-", o, "max_height"),
            o if o.starts_with("max-w-") => parse_field_value("max-w-", o, "max_width"),
            o if o.starts_with("max-h-") => parse_field_value("max-h-", o, "max_height"),
            o if o.starts_with("m-") => parse_field_rect("m-", o, "margin"),
            o if o.starts_with("left-") => parse_field_value("left-", o, "left"),
            o if o.starts_with("right-") => parse_field_value("right-", o, "right"),
            o if o.starts_with("top-") => parse_field_value("top-", o, "top"),
            o if o.starts_with("bottom-") => parse_field_value("bottom-", o, "bottom"),
            o if o.contains(':') => TokenStream::from_str(o).unwrap_or_else(|e| {
                let message = format!("invalid style: {o:?} error: {e:?}");
                quote!(error: compile_error!(#message))
            }),
            o => {
                let message = format!("unknown style: {o:?}");
                quote!(error: compile_error!(#message))
            }
        };
        fields.push(tokens);
    }
    quote_spanned! {input.span()=>
        Style {
            #(#fields,)*
            ..Style::default()
        }
    }
}
