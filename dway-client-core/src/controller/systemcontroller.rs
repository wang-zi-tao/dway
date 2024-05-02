use system_shutdown::{hibernate, logout, reboot, shutdown, sleep};

use crate::prelude::*;

#[derive(Event)]
pub enum SystemControllRequest {
    Reboot,
    Shutdown,
    Logout,
    Sleep,
    Hibernate,
}

pub fn receive_system_controll_request(mut events: EventReader<SystemControllRequest>) {
    for event in events.read() {
        match event {
            SystemControllRequest::Reboot => {
                if let Err(e) = reboot() {
                    error!("failed reboot the machine reboot: {e}");
                }
            }
            SystemControllRequest::Shutdown => {
                if let Err(e) = shutdown() {
                    error!("failed to shut down the machine: {e}");
                }
            }
            SystemControllRequest::Logout => {
                if let Err(e) = logout() {
                    error!("failed to log out the user: {e}");
                }
            }
            SystemControllRequest::Sleep => {
                if let Err(e) = sleep() {
                    error!("failed to put the machine to sleep: {e}");
                }
            }
            SystemControllRequest::Hibernate => {
                if let Err(e) = hibernate() {
                    error!("failed to hibernate the machine : {e}");
                }
            }
        };
    }
}
