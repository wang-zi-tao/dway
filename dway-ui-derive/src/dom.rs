use crate::domarg::DomArg;
use derive_syn_parse::Parse;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse::ParseStream, spanned::Spanned, token::Paren, *};

pub struct DomChildren {
    pub list: Vec<Dom>,
}
impl syn::parse::Parse for DomChildren {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut list = vec![];
        while !input.peek(Token![<]) || !input.peek2(Token![/]) {
            list.push(input.parse()?);
        }
        Ok(Self { list })
    }
}

#[derive(Parse)]
pub(crate) enum DomBundle {
    #[peek(Paren, name = "Paren")]
    Expr {
        #[paren]
        _wrap: Paren,
        #[inside(_wrap)]
        expr: Expr,
    },
    #[peek(Ident, name = "Ident")]
    Ident(Type),
}
impl ToTokens for DomBundle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ty) => tokens.extend(quote!(#ty::default())),
            Self::Expr { expr, .. } => tokens.extend(quote!(#expr)),
        }
    }
}
impl DomBundle {
    pub fn generate_spawn(&self, ty: Option<TokenStream>) -> TokenStream {
        match &self {
            DomBundle::Expr {
                expr: Expr::Tuple(inner),
                ..
            } if inner.elems.is_empty() => {
                quote!(commands.spawn_empty())
            }
            DomBundle::Expr { expr, .. } => {
                if let Some(ty) = ty {
                    quote!(commands.spawn(#expr as #ty))
                } else {
                    quote!(commands.spawn(#expr))
                }
            }
            DomBundle::Ident(bundle_tyle) => {
                if let Some(ty) = ty {
                    quote!(commands.spawn(#bundle_tyle::default() as #ty))
                } else {
                    quote!(commands.spawn(#bundle_tyle::default()))
                }
            }
        }
    }
}

#[derive(Parse)]
pub(crate) struct DomEnd {
    _lt1: Token![<],
    _end1: Token![/],
    pub end_bundle: Option<Ident>,
    _gt1: Token![>],
}

#[derive(Parse)]
pub struct Dom {
    _lt0: Token![<],
    pub bundle: DomBundle,
    #[call(DomArg::parse_vec)]
    pub args: Vec<DomArg>,
    _end0: Option<Token![/]>,
    _gt0: Token![>],
    #[parse_if(_end0.is_none())]
    pub children: Option<DomChildren>,
    #[parse_if(_end0.is_none())]
    pub end_tag: Option<DomEnd>,
}
impl Dom {
    pub fn span(&self) -> Span {
        self._lt0.span().join(self._gt0.span()).unwrap()
    }

    pub fn parse_vec(input: ParseStream) -> syn::Result<Vec<Self>> {
        let mut vec = Vec::new();
        while !input.is_empty() {
            let arg: Self = input.parse()?;
            vec.push(arg);
        }
        Ok(vec)
    }

    pub fn generate_spawn(&self) -> TokenStream {
        let spawn_bundle = self.bundle.generate_spawn(
            self.end_tag
                .as_ref()
                .and_then(|end| end.end_bundle.as_ref())
                .map(|ty| quote_spanned!(ty.span()=>#ty)),
        );
        let components_expr: Vec<_> = self
            .args
            .iter()
            .filter_map(|arg| arg.get_component_expr())
            .collect();
        if components_expr.is_empty() {
            spawn_bundle
        } else {
            quote! {
                #spawn_bundle.insert((#(#components_expr),*))
            }
        }
    }
}
