use std::sync::{Arc, Mutex};

use bevy::{prelude::*, utils::HashMap};
use send_wrapper::SendWrapper;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::Session;
use uuid::Uuid;

#[derive(Resource)]
pub struct SeatSessions {
    pub sessions: HashMap<String, Entity>,
}

pub struct SeatSessionInner {
    pub raw: LibSeatSession,
}
unsafe impl Send for SeatSessionInner {}

#[derive(Component)]
pub struct SeatSession {
    pub inner: Arc<Mutex<SeatSessionInner>>,
}

#[derive(Bundle)]
pub struct SeatSessionBundle {
    pub seat: SeatSession,
    pub name: Name,
}

pub fn setup(
    mut seat_session_set: NonSendMut<SeatSessions>,
    mut commands: Commands,
    // seat_sessions:Query<(&mut SeatSession)>,
) {
    let (session, notifier) = match LibSeatSession::new() {
        Ok(ret) => ret,
        Err(err) => {
            error!("Could not initialize a session: {}", err);
            return;
        }
    };
    let seat_name = session.seat();
    let entity = commands
        .spawn((
            Name::new(seat_name.clone()),
            SeatSession {
                inner: Arc::new(Mutex::new(SeatSessionInner { raw: session })),
            },
        ))
        .id();
    seat_session_set.sessions.insert(seat_name, entity);
}
