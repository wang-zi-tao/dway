use convert_case::Casing;
use derive_syn_parse::Parse;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::{
    braced, parenthesized,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Paren},
    Token, Type,
};

use crate::builder::QueryBuilder;

#[derive(Parse)]
pub struct NodeQueryField {
    pub name: Ident,
    pub _col: Token![:],
    pub ty: NodeQuery,
}

mod kw {
    use syn::custom_keyword;

    custom_keyword!(Ref);
    custom_keyword!(Has);
    custom_keyword!(Entity);
    custom_keyword!(Option);
}

pub enum NodeQuery {
    Entity(kw::Entity),
    Reference {
        reference: Token![&],
        mutable: Option<Token![mut]>,
        ty: Type,
    },
    Option {
        kw: kw::Option,
        start: Token![<],
        query: Box<NodeQuery>,
        end: Token![>],
    },
    Ref {
        kw: kw::Ref,
        start: Token![<],
        query: Box<NodeQuery>,
        end: Token![>],
    },
    Has {
        kw: kw::Has,
        start: Token![<],
        query: Box<NodeQuery>,
        end: Token![>],
    },
    Other {
        ty: Type,
    },
    Tuple {
        paren: Paren,
        elements: Punctuated<NodeQuery, Token![,]>,
    },
    Structure {
        brace: Brace,
        fields: Punctuated<NodeQueryField, Token![,]>,
    },
}

impl NodeQuery {
    pub fn to_item_type(&self, builder: &mut QueryBuilder, name: &Ident) -> TokenStream {
        let query_data_type = self.to_type(builder, name);
        if builder.mutable {
            quote!(&mut bevy::ecs::query::QueryItem<#query_data_type>)
        } else {
            quote!(&bevy::ecs::query::ROQueryItem<#query_data_type>)
        }
    }

    pub fn to_type(&self, builder: &mut QueryBuilder, name: &Ident) -> TokenStream {
        match self {
            NodeQuery::Entity(ident) => quote!(#ident),
            NodeQuery::Reference {
                reference,
                mutable,
                ty,
            } => quote_spanned!(ty.span()=> #reference 'static #mutable #ty),
            NodeQuery::Option {
                kw,
                start,
                query,
                end,
            } => {
                let query_type = query.to_type(builder, name);
                quote_spanned!(name.span()=> #kw #start #query_type #end)
            }
            NodeQuery::Ref {
                kw,
                start,
                query,
                end,
            } => {
                let query_type = query.to_type(builder, name);
                quote_spanned!(name.span()=> #kw #start 'static, #query_type #end)
            }
            NodeQuery::Has {
                kw,
                start,
                query,
                end,
            } => {
                let query_type = query.to_type(builder, name);
                quote_spanned!(name.span()=> #kw #start #query_type #end)
            }
            NodeQuery::Other { ty } => {
                quote_spanned!(name.span()=> #ty)
            }
            NodeQuery::Tuple { paren, elements } => {
                let elements_type = elements.iter().map(|elem| elem.to_type(builder, name));
                quote_spanned!(paren.span=> (#(#elements_type),*))
            }
            NodeQuery::Structure { brace, fields } => {
                let structure_name = format_ident!(
                    "{}Query",
                    &name.to_string().to_case(convert_case::Case::Pascal),
                    span = brace.span.span(),
                );
                let structure_fields = fields.iter().map(|f| {
                    let name = &f.name;
                    let ty = f.ty.to_type(builder, name);
                    quote!(#name: #ty)
                });
                let structure = quote! {
                    #[derive(bevy::ecs::query::QueryData)]
                    #[query_data(mutable)]
                    struct #structure_name {
                        #(#structure_fields),*
                    }
                };
                builder.add_item(&structure_name, structure);
                quote_spanned!(brace.span=> #structure_name)
            }
        }
    }

    pub fn build(&self, builder: &mut QueryBuilder, name: &Ident, query_filter: Option<&Type>) {
        let mutable = builder.mutable;
        let span = name.span();
        let inner = std::mem::replace(&mut builder.code, quote!());
        let ty = self.to_type(builder, name);
        let query = builder.add_query(&quote_spanned!(span=> (Entity,#ty)), query_filter);
        let mut_flag = mutable.then_some(quote_spanned!(span=>mut));
        builder.code = if builder.node_stack.len() == 1 && !builder.has_begin_node {
            let iter_method = if mutable {
                quote!(iter_mut)
            } else {
                quote!(iter)
            };
            quote_spanned! {span=>
                #[allow(unused_variables)]
                for (entity,#mut_flag #name) in self.#query.#iter_method() {
                    #inner
                }
            }
        } else {
            let get_method = if mutable {
                quote!(get_mut)
            } else {
                quote!(get)
            };
            quote_spanned! {span=>
                #[allow(unused_variables)]
                if let Ok((entity,#mut_flag #name)) = self.#query.#get_method(entity) {
                    #inner
                }
            }
        };
    }
}

impl syn::parse::Parse for NodeQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![&]) {
            let reference: Token![&] = input.parse()?;
            let mutable = if input.peek(Token![mut]) {
                Some(input.parse()?)
            } else {
                None
            };
            Ok(Self::Reference {
                reference,
                mutable,
                ty: input.parse()?,
            })
        } else if input.peek(Paren) {
            let content;
            Ok(Self::Tuple {
                paren: parenthesized!(content in input),
                elements: content.parse_terminated(NodeQuery::parse, Token![,])?,
            })
        } else if input.peek(Brace) {
            let content;
            Ok(Self::Structure {
                brace: braced!(content in input),
                fields: content.parse_terminated(NodeQueryField::parse, Token![,])?,
            })
        } else if input.peek(kw::Entity) {
            Ok(Self::Entity(input.parse()?))
        } else if input.peek(kw::Option) {
            Ok(Self::Option {
                kw: input.parse()?,
                start: input.parse()?,
                query: input.parse()?,
                end: input.parse()?,
            })
        } else if input.peek(kw::Ref) {
            Ok(Self::Ref {
                kw: input.parse()?,
                start: input.parse()?,
                query: input.parse()?,
                end: input.parse()?,
            })
        } else if input.peek(kw::Has) {
            Ok(Self::Has {
                kw: input.parse()?,
                start: input.parse()?,
                query: input.parse()?,
                end: input.parse()?,
            })
        } else {
            Ok(Self::Other { ty: input.parse()? })
        }
    }
}
