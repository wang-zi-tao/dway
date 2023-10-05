use bevy::{prelude::App, reflect::GetTypeRegistration};

use crate::{ConnectableMut, Relationship, RelationshipRegister};

pub trait AppExt {
    fn register_relation<R>(&mut self)
    where
        R: Relationship + 'static,
        R::From: GetTypeRegistration + ConnectableMut,
        R::To: GetTypeRegistration + ConnectableMut;
}
impl AppExt for App {
    fn register_relation<R>(&mut self)
    where
        R: Relationship + 'static,
        R::From: GetTypeRegistration + ConnectableMut,
        R::To: GetTypeRegistration + ConnectableMut,
    {
        self.register_type::<R::From>();
        self.register_type::<R::To>();
        RelationshipRegister::register::<R>(&mut self.world);
    }
}
