use dway_server::macros::Resource;
use getset::Getters;
use sysinfo::{get_current_pid, ProcessRefreshKind, RefreshKind, System, Users};

#[derive(Resource, Getters)]
pub struct UserInfo {
    #[getset(get="pub")]
    name: Option<String>,
    #[getset(get_copy="pub")]
    uid: Option<u32>,
    #[getset(get_copy="pub")]
    gid: Option<u32>,
}

impl Default for UserInfo {
    fn default() -> Self {
        let system = System::new_with_specifics(RefreshKind::default().with_processes(
            ProcessRefreshKind::new().with_user(sysinfo::UpdateKind::OnlyIfNotSet),
        ));
        let users = Users::new_with_refreshed_list();
        let user = get_current_pid()
            .ok()
            .and_then(|pid| system.process(pid))
            .and_then(|process| process.user_id())
            .and_then(|uid| users.get_user_by_id(uid));
        Self {
            name: user.map(|u| u.name().to_string()),
            uid: user.map(|u| **u.id()),
            gid: user.map(|u| *u.group_id()),
        }
    }
}
