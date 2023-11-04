use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

pub fn generate_despawn(entity: TokenStream) -> TokenStream {
    quote! {
        if commands.get_entity(#entity).is_some() {
            commands
                .entity(#entity)
                .despawn_recursive();
        }
    }
}
pub fn generate_state_change_variable_from_raw(name: &str, span: Span) -> Ident {
    format_ident!("state_changed_{}", name, span = span)
}

pub fn generate_state_change_variable(ident: &Ident) -> Ident {
    generate_state_change_variable_from_raw(&ident.to_string(), ident.span())
}
