use xcursor::{
    parser::{parse_xcursor, Image},
    CursorTheme,
};

pub struct Cursor {
    icons: Vec<Image>,
    size: u32,
}
impl Cursor {
    pub(crate) fn load() -> Self {
        todo!()
    }

    pub(crate) fn get_image(&self, arg: i32, zero: std::time::Duration) -> Image {
        todo!()
    }
}
