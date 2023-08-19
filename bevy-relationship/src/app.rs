use bevy::{
    prelude::App,
    reflect::GetTypeRegistration,
};

use crate::Relationship;

pub trait AppExt {
    fn register_relation<R>(&mut self)
    where
        R: Relationship,
        R::From: GetTypeRegistration,
        R::To: GetTypeRegistration;
}
impl AppExt for App {
    fn register_relation<R>(&mut self)
    where
        R: Relationship,
        R::From: GetTypeRegistration,
        R::To: GetTypeRegistration,
    {
        self.register_type::<R::From>();
        self.register_type::<R::To>();
    }
}
