use std::sync::{Mutex, Arc};

use bevy::{prelude::*, utils::HashMap};
use send_wrapper::SendWrapper;
use smithay::backend::session::libseat::LibSeatSession;
use uuid::Uuid;


#[derive(Resource)]
pub struct SeatSessions{
    pub sessions:HashMap<Uuid,SeatSession>,
}

#[derive(Component)]
pub struct SeatSession{
    pub raw:Arc<SendWrapper<Mutex<LibSeatSession>>>,
}

// pub fn setup(
//     seat_session_set:NonSendMut<SeatSessions>,
//     commands:Commands,
//     // seat_sessions:Query<(&mut SeatSession)>,
// ){
//     let (session, notifier) = match LibSeatSession::new(None) {
//         Ok(ret) => ret,
//         Err(err) => {
//             error!("Could not initialize a session: {}", err);
//             return;
//         }
//     };
// }
