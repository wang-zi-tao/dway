use convert_case::Casing;
use derive_syn_parse::Parse;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::HashMap;
use syn::{
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{At, Brace, Paren, RArrow},
    *,
};

pub fn generate_change_detect(structure: &ItemStruct) -> anyhow::Result<TokenStream2> {
    let &ItemStruct {
        attrs,
        vis,
        struct_token,
        ident,
        generics,
        ..
    } = &structure;
    let name = &ident;
    let bitflags_name = format_ident!("{}FieldChangeFlag", name, span = name.span());

    let mut bit_map: HashMap<String, Ident> = HashMap::new();
    let mut functions: Vec<TokenStream2> = vec![];
    let mut bits: Vec<TokenStream2> = vec![];
    let mut fields: Vec<TokenStream2> = vec![];

    for field in &structure.fields {
        let Field { vis, ty, .. } = &field;
        fields.push(quote!(#field));
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| anyhow::format_err!("no field name"))?;
        let bit_name = format_ident!(
            "{}",
            field_name
                .to_string()
                .to_case(convert_case::Case::UpperSnake),
            span = name.span()
        );
        bit_map.insert(field_name.to_string(), bit_name.clone());
        let pos = bits.len();
        bits.push(quote!(const #bit_name = 1 << #pos; ));

        let getter_name = format_ident!("{}", field_name, span = field_name.span());
        let setter_name = format_ident!("{}_mut", field_name, span = field_name.span());
        let changed_name = format_ident!("{}_is_changeed", field_name, span = field_name.span());
        functions.push(quote! {
            #vis fn #getter_name(&self) -> & #ty {
                 &self.#field_name
            }
            #vis fn #setter_name(&mut self) -> &mut #ty {
                 self.changed_flags |= #bitflags_name::#bit_name;
                 &mut self.#field_name
            }
            #vis fn #changed_name(&self) -> bool {
                 self.changed_flags.contains(#bitflags_name::#bit_name)
            }
        });
    }

    Ok(quote! {
        #(#attrs)*
        #vis #struct_token #ident #generics {
             #(#fields,)*
             changed_flags: #bitflags_name,
        };

        bitflags::bitflags! {
            struct #bitflags_name {
                #(#bits)*
            }
        }

        impl #name {
            #(#functions)*
            #vis fn marks_all(&mut self) {
                self.changed_flags = #bitflags_name::all();
            }
            #vis fn is_inner_changed(&mut self) {
                !&self.changed_flags.is_empty()
            }
            #vis fn clear_marks(&mut self) -> bool {
                let empty = &self.changed_flags.is_empty();
                self.changed_flags = #bitflags_name::empty();
                empty
            }
        }
    })
}
