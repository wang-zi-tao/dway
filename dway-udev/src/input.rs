use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use smithay::{
    backend::{
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        session::libseat::LibSeatSession,
    },
    reexports::input::Libinput,
};

use crate::seat::SeatSession;

pub struct LibInputInner {
    pub raw: Libinput,
}
unsafe impl Send for LibInputInner {}
#[derive(Component)]
pub struct LibInput {
    pub libinput: Arc<Mutex<LibInputInner>>,
    pub seat_entity: Entity,
}

pub fn add_seat(
    new_seat: Query<(Entity, &SeatSession, &Name), Added<SeatSession>>,
    mut commands: Commands,
) {
    for (seat_entity, seat, name) in new_seat.iter() {
        let raw_seat = &mut seat.inner.lock().unwrap().raw;
        let mut libinput_context = Libinput::new_with_udev::<
            LibinputSessionInterface<LibSeatSession>,
        >(raw_seat.clone().into());
        libinput_context.udev_assign_seat(&name).unwrap();
        commands.spawn(LibInput {
            libinput: Arc::new(Mutex::new(LibInputInner {
                raw: libinput_context,
            })),
            seat_entity,
        });
    }
}
