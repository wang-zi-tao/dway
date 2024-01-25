use derive_syn_parse::Parse;
use syn::{Expr, Token};

#[derive(Parse)]
pub struct Filter {
    pub expr: Expr,
}
