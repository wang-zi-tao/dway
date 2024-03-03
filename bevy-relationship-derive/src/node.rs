use crate::{filter::Filter, query::QueryBuilder};
use derive_syn_parse::Parse;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use quote::quote_spanned;
use syn::{
    braced, parenthesized,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Paren},
    Token, Type,
};

#[derive(Parse)]
pub struct NodeQueryField {
    pub name: Ident,
    pub col: Token![:],
    pub ty: NodeQuery,
}

pub enum NodeQuery {
    Entity(Ident),
    Reference {
        reference: Token![&],
        mutable: Option<Token![mut]>,
        ty: Type,
    },
    Option {
        option: Ident,
        start: Token![<],
        query: Box<NodeQuery>,
        end: Token![>],
    },
    Ref {
        name: Ident,
        start: Token![<],
        query: Box<NodeQuery>,
        end: Token![>],
    },
    Mut {
        name: Ident,
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
    pub fn to_type(&self, builder: &mut QueryBuilder, name: &Ident) -> TokenStream {
        match self {
            NodeQuery::Entity(ident) => quote!(#ident),
            NodeQuery::Reference {
                reference,
                mutable,
                ty,
            } => quote_spanned!(ty.span()=> #reference 'static #mutable #ty),
            NodeQuery::Option {
                option,
                start,
                query,
                end,
            } => {
                let query_type = query.to_type(builder, name);
                quote_spanned!(name.span()=> #option #start #query_type #end)
            }
            NodeQuery::Ref {
                name,
                start,
                query,
                end,
            } => {
                let query_type = query.to_type(builder, name);
                quote_spanned!(name.span()=> #name #start #query_type #end)
            }
            NodeQuery::Mut {
                name,
                start,
                query,
                end,
            } => {
                let query_type = query.to_type(builder, name);
                quote_spanned!(name.span()=> #name #start #query_type #end)
            }
            NodeQuery::Other { ty } => {
                quote_spanned!(name.span()=> #ty)
            }
            NodeQuery::Tuple { paren, elements } => {
                let elements_type = elements.iter().map(|elem| elem.to_type(builder, name));
                quote_spanned!(paren.span=> (#(#elements_type),*))
            }
            NodeQuery::Structure { brace, fields } => {
                let structure_name = builder.alloc_name(&name.to_string(), brace.span.span());
                let structure_fields = fields.iter().map(|f| {
                    let name = &f.name;
                    let ty = f.ty.to_type(builder, name);
                    quote!(#name: #ty)
                });
                let structure = quote! {
                    #[derive(WorldQuery)]
                    #[world_query(derive(Debug))]
                    struct #structure_name {
                        #(#structure_fields),*
                    }
                };
                builder.add_item(&structure_name, structure);
                quote_spanned!(brace.span=> #structure_name)
            }
        }
    }

    pub fn build(&self, builder: &mut QueryBuilder, name: &Ident) {
        let span = name.span();
        let inner = std::mem::replace(&mut builder.code, quote!());
        let ty = self.to_type(builder, name);
        let query = builder.add_query(&quote_spanned!(span=> (Entity,#ty)));
        builder.code = quote_spanned! {span=>
            if let Some((entity,#name)) = self.#query.get(entity) {
                #inner
            }
        }
    }
}

impl syn::parse::Parse for NodeQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![&]) {
            let reference: Token![&] = input.parse()?;
            let mutable = if input.peek(Token![mut]) {
                let mutable: Token![mut] = input.parse()?;
                Some(mutable)
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
        } else {
            let ident: Ident = input.parse()?;
            match &*ident.to_string() {
                "Entity" => Ok(Self::Entity(ident)),
                "Option" => Ok(Self::Option {
                    option: ident,
                    start: input.parse()?,
                    query: input.parse()?,
                    end: input.parse()?,
                }),
                "Ref" => Ok(Self::Ref {
                    name: ident,
                    start: input.parse()?,
                    query: input.parse()?,
                    end: input.parse()?,
                }),
                "Mut" => Ok(Self::Mut {
                    name: ident,
                    start: input.parse()?,
                    query: input.parse()?,
                    end: input.parse()?,
                }),
                _ => Ok(Self::Other { ty: input.parse()? }),
            }
        }
    }
}

pub enum Node {
    WithFilter {
        _lt: Token!(<),
        ty: NodeQuery,
        _comma: Token!(,),
        filter: Type,
        _gt: Token!(>),
    },
    WithoutFilter {
        ty: NodeQuery,
    },
}

impl syn::parse::Parse for Node {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![<]) {
            Ok(Self::WithFilter {
                _lt: input.parse()?,
                ty: input.parse()?,
                _comma: input.parse()?,
                filter: input.parse()?,
                _gt: input.parse()?,
            })
        } else {
            Ok(Self::WithoutFilter { ty: input.parse()? })
        }
    }
}
