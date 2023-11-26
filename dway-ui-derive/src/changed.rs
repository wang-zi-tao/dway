use anyhow::anyhow;
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
    
    let integer_type = match structure.fields.len() {
        0..=7 => quote!(u8),
        8..=15 => quote!(u16),
        16..=31 => quote!(u32),
        32..=63 => quote!(u64),
        64..=127 => quote!(u128),
        _ => { anyhow::bail!("too much field") },
    };

    for field in &structure.fields {
        let Field { attrs, vis, ident, colon_token, ty, .. } = &field;
        fields.push(quote!(#(#attrs)* #ident #colon_token #ty));
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
        let get_mut_name = format_ident!("{}_mut", field_name, span = field_name.span());
        let setter_name = format_ident!("set_{}", field_name, span = field_name.span());
        let changed_name = format_ident!("{}_is_changed", field_name, span = field_name.span());
        functions.push(quote_spanned! {field.span()=>
            #vis fn #getter_name(&self) -> & #ty {
                 &self.#field_name
            }
            #vis fn #get_mut_name(&mut self) -> &mut #ty {
                 self.__dway_changed_flags |= #bitflags_name::#bit_name;
                 &mut self.#field_name
            }
            #vis fn #setter_name(&mut self, value: #ty) {
                 self.__dway_changed_flags |= #bitflags_name::#bit_name;
                 self.#field_name = value;
            }
            #vis fn #changed_name(&self) -> bool {
                 self.__dway_changed_flags.contains(#bitflags_name::#bit_name)
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = structure.generics.split_for_impl();

    Ok(quote! {
        bitflags::bitflags! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
            #vis struct #bitflags_name : #integer_type {
                #(#bits)*
            }
        }

        #(#attrs)*
        #vis #struct_token #ident #generics {
             #(#fields,)*
             __dway_changed_flags: #bitflags_name,
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #(#functions)*
            #vis fn marks_all(&mut self) {
                self.__dway_changed_flags = #bitflags_name::all();
            }
            #vis fn is_inner_changed(&mut self) -> bool {
                !self.__dway_changed_flags.is_empty()
            }
            #vis fn clear_marks(&mut self) -> bool {
                let empty = self.__dway_changed_flags.is_empty();
                self.__dway_changed_flags = #bitflags_name::empty();
                empty
            }
        }
    })
}
