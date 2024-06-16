#[derive(Debug, Default, Clone)]
pub enum ControlFlow<T = ()> {
    #[default]
    Continue,
    Break,
    Return(T),
}

impl ControlFlow<()> {
    pub fn new() -> ControlFlow<()> {
        ControlFlow::<()>::Continue
    }

    pub fn continue_iter() -> ControlFlow<()> {
        ControlFlow::<()>::Continue
    }

    pub fn break_iter() -> ControlFlow<()> {
        ControlFlow::<()>::Break
    }
}

impl<T> ControlFlow<T> {
    pub fn return_from_iter(value: T) -> ControlFlow<T> {
        ControlFlow::Return(value)
    }
}
