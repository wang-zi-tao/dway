use bevy::{
    prelude::{App, Last},
    reflect::GetTypeRegistration,
};

use crate::{apply_disconnection, ConnectableMut, ConnectionEventReceiver, Relationship};

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
        if !self.world.contains_non_send::<ConnectionEventReceiver>() {
            self.init_non_send_resource::<ConnectionEventReceiver>();
            self.add_systems(Last, apply_disconnection);
        }
    }
}
