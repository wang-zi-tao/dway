use crate::prelude::*;

pub trait ResourceWrapper {
    type Resource: WlResource;
    fn get_resource(&self) -> &Self::Resource;
}

structstruck::strike!{
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    pub struct MimeData{
        pub kind: enum MimeDataKind{
            Text,
            Image,
            Html,
            Other(String)
        },
        pub data: Vec<u8>,
    }
}

#[derive(Resource, Debug, Reflect)]
pub struct Clipboard{
    pub mine_data: Option<MimeData>
}
