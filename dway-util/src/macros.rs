
#[macro_export]
macro_rules! try_or {
    ($expr:expr ,$message:expr, $else_expr:expr) => {
        match (||{$expr})() {
            Ok(o) => o,
            Err(e) => {
                error!("{}: {e}", $message);
                $else_expr
            }
        }
    };
}
