use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, *};

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
    let mut functions: Vec<TokenStream2> = vec![];
    let mut fields: Vec<TokenStream2> = vec![];

    let all = (1 << (structure.fields.len())) - 1;
    let integer_type = match structure.fields.len() {
        0..=7 => quote!(u8),
        8..=15 => quote!(u16),
        16..=31 => quote!(u32),
        32..=63 => quote!(u64),
        64..=127 => quote!(u128),
        _ => {
            anyhow::bail!("too much field")
        }
    };

    for (index, field) in structure.fields.iter().enumerate() {
        let Field {
            attrs,
            vis,
            ident,
            colon_token,
            ty,
            ..
        } = &field;
        fields.push(quote!(#(#attrs)* #ident #colon_token #ty));
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| anyhow::format_err!("no field name"))?;
        let bit = quote!(( 1 << ( #index as #integer_type ) ));

        let getter_name = format_ident!("{}", field_name, span = field_name.span());
        let get_mut_name = format_ident!("{}_mut", field_name, span = field_name.span());
        let setter_name = format_ident!("set_{}", field_name, span = field_name.span());
        let changed_name = format_ident!("{}_is_changed", field_name, span = field_name.span());
        functions.push(quote_spanned! {field.span()=>
            #[allow(dead_code)]
            #vis fn #getter_name(&self) -> & #ty {
                 &self.#field_name
            }
            #[allow(dead_code)]
            #vis fn #get_mut_name(&mut self) -> &mut #ty {
                 self.__dway_changed_flags |= #bit;
                 &mut self.#field_name
            }
            #[allow(dead_code)]
            #vis fn #setter_name(&mut self, value: #ty) {
                 self.__dway_changed_flags |= #bit;
                 self.#field_name = value;
            }
            #[allow(dead_code)]
            #vis fn #changed_name(&self) -> bool {
                (self.__dway_changed_flags & #bit) != 0
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = structure.generics.split_for_impl();

    Ok(quote! {

        #(#attrs)*
        #vis #struct_token #ident #generics {
             #(#fields,)*
             __dway_changed_flags: #integer_type,
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #(#functions)*
            #vis fn marks_all(&mut self) {
                self.__dway_changed_flags = #all as #integer_type;
            }
            #vis fn is_inner_changed(&mut self) -> bool {
                self.__dway_changed_flags != 0
            }
            #vis fn clear_marks(&mut self) -> bool {
                let empty = self.__dway_changed_flags == 0;
                self.__dway_changed_flags = 0;
                !empty
            }
        }
    })
}
