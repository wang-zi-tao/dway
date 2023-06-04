use crate::prelude::*;

pub trait ResourceWrapper {
    type Resource: WlResource;
    fn get_resource(&self) -> &Self::Resource;
}
