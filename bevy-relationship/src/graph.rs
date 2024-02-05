#[derive(Debug, Default, Clone)]
pub enum ControlFlow<T = ()> {
    #[default]
    Continue,
    Break,
    Return(T),
}

impl<T> ControlFlow<T>{
    pub fn new()->ControlFlow<()>{
        ControlFlow::<()>::Continue
    }
}
