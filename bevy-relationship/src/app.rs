use bevy::{prelude::App, reflect::GetTypeRegistration};

use crate::{ConnectableMut, Relationship};

pub trait AppExt {
    fn register_relation<R>(&mut self) -> &mut Self
    where
        R: Relationship + 'static,
        R::From: GetTypeRegistration + ConnectableMut,
        R::To: GetTypeRegistration + ConnectableMut;
}
impl AppExt for App {
    fn register_relation<R>(&mut self) -> &mut Self
    where
        R: Relationship + 'static,
        R::From: GetTypeRegistration + ConnectableMut,
        R::To: GetTypeRegistration + ConnectableMut,
    {
        self.register_type::<R::From>();
        self.register_type::<R::To>();
        self
    }
}
