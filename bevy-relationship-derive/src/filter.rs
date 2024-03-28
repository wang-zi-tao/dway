use derive_syn_parse::Parse;
use syn::{Expr};

#[derive(Parse)]
pub struct Filter {
    pub expr: Expr,
}
