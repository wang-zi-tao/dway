
#[derive(Debug,Default,Clone)]
pub enum ControlFlow<T=()> {
    #[default]
    Continue,
    Break,
    Return(T),
}
