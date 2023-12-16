#![feature(iter_map_windows)]
#![feature(trait_upcasting)]

mod builder;
mod changed;
mod dom;
mod domarg;
mod domcontext;
mod generate;
mod parser;
mod prelude;
mod style;

use crate::dom::*;

use derive_syn_parse::Parse;
use domcontext::widget_context::WidgetDeclare;

use proc_macro::TokenStream;

use quote::{format_ident, quote, quote_spanned};

use syn::{
    spanned::Spanned,
    *,
};

#[derive(Parse)]
struct SpawnDomInput {
    pub commands: Expr,
    split: Token![=>],
    pub dom: Dom,
}

#[proc_macro]
pub fn spawn(input: TokenStream) -> TokenStream {
    let SpawnDomInput {
        commands,
        split,
        dom,
    } = parse_macro_input!(input as SpawnDomInput);
    let stats = domcontext::spawn_context::generate(&dom);
    TokenStream::from(quote_spanned!(split.span()=> {
        let commands = #commands;
        #stats
    }))
}

fn parse_color_str(input: &str) -> Option<[u8; 4]> {
    let re =
        regex_macro::regex!("#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})?");
    let cap = re.captures(input)?;
    let r = u8::from_str_radix(cap.get(1)?.as_str(), 16).ok()?;
    let g = u8::from_str_radix(cap.get(2)?.as_str(), 16).ok()?;
    let b = u8::from_str_radix(cap.get(3)?.as_str(), 16).ok()?;
    let a = cap
        .get(4)
        .and_then(|f| u8::from_str_radix(f.as_str(), 16).ok())
        .unwrap_or(0xff);
    Some([r, g, b, a])
}

#[proc_macro]
pub fn color(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let Some([r, g, b, a]) = parse_color_str(&lit.value()) else {
        return TokenStream::from(
            quote_spanned!(lit.span()=> compile_error!("failed to parse color")),
        );
    };
    TokenStream::from(quote_spanned!(lit.span()=> Color::rgba_u8(#r,#g,#b,#a)))
}

#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let style = style::generate(&lit);
    TokenStream::from(quote_spanned!(lit.span()=> #style))
}

#[proc_macro]
pub fn node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Ident);
    let ident = format_ident!("node_{}_entity", input, span = input.span());
    TokenStream::from(quote_spanned!(input.span()=> { widget.#ident }))
}

#[proc_macro_attribute]
pub fn change_detact(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let output = changed::generate_change_detect(&input).unwrap_or_else(|e| {
        let message = e.to_string();
        quote!(compile_error!(#message))
    });
    TokenStream::from(output)
}

#[proc_macro]
pub fn auto_expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as WidgetDeclare);
    let plugin = domcontext::widget_context::generate(&input);
    let output = quote! {
        #plugin
    };
    TokenStream::from(output)
}

#[proc_macro]
pub fn dway_widget(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as WidgetDeclare);
    input.args.push(parse_quote!(@bundle{{
        pub node: Node,
        pub style: Style,
        pub focus_policy: bevy::ui::FocusPolicy,
        pub transform: Transform,
        pub global_transform: GlobalTransform,
        pub visibility: Visibility,
        pub inherited_visibility: InheritedVisibility,
        pub view_visibility: ViewVisibility,
        pub z_index: ZIndex,
    }}));
    let plugin = domcontext::widget_context::generate(&input);
    let output = quote! {
        #plugin
    };
    TokenStream::from(output)
}
