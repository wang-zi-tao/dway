#[macro_export]
macro_rules! try_or {
    ($expr:expr ,$message:expr, $else_expr:expr) => {
        match (|| $expr)() {
            Ok(o) => o,
            Err(e) => {
                error!("{}: {e}", $message);
                $else_expr
            }
        }
    };
}

#[macro_export]
macro_rules! update {
    ($to:expr , $from:expr) => {
        if $to != $from {
            $to = $from;
        }
    };

    ($to:expr , $from:expr, $b:block) => {
        if $to != $from {
            $b
            $to = $from;
        }
    };
}
