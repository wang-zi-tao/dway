pub mod icon;

use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use dway_util::try_or;
use gettextrs::{dgettext, setlocale, LocaleCategory};

use crate::{apps::icon::Icon, prelude::*, schedule::DWayServerSet, xdg::toplevel::XdgToplevel};

use self::icon::IconLoader;

#[derive(Resource, Default, Reflect)]
pub struct DesktopEntriesSet {
    pub list: Vec<Entity>,
    pub by_id: HashMap<String, Entity>,
}
impl DesktopEntriesSet {
    pub fn register(&mut self, entry: &DesktopEntry, entity: Entity) {
        self.list.push(entity);
        self.by_id.insert(entry.appid.clone(), entity);
    }
}

#[derive(Component, Debug, Reflect)]
pub struct DesktopEntry {
    pub appid: String,
    pub path: PathBuf,
    pub groups: HashMap<String, HashMap<String, (String, HashMap<String, String>)>>,
    pub ubuntu_gettext_domain: Option<String>,
}

impl DesktopEntry {
    pub fn new(entry: freedesktop_desktop_entry::DesktopEntry) -> Self {
        Self {
            appid: entry.appid.to_string(),
            path: entry.path.to_path_buf(),
            groups: entry
                .groups
                .into_iter()
                .map(|(groupid, group)| {
                    (
                        groupid.to_string(),
                        group
                            .into_iter()
                            .map(|(key, (value, locate_map))| {
                                (
                                    key.to_string(),
                                    (
                                        value.to_string(),
                                        locate_map
                                            .into_iter()
                                            .map(|(locate, v)| (locate.to_string(), v.to_string()))
                                            .collect(),
                                    ),
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
            ubuntu_gettext_domain: entry.ubuntu_gettext_domain.map(|s| s.to_string()),
        }
    }

    pub fn action_entry(&self, action: &str, key: &str) -> Option<&str> {
        let group = self
            .groups
            .get(["Desktop Action ", action].concat().as_str());

        group.and_then(|group| group.get(key)).map(|key| &*key.0)
    }

    pub fn get_without_locale(&self, group: &str, key: &str) -> Option<&str> {
        Some(&self.groups.get(group)?.get(key)?.0)
    }

    pub fn get_in_current_locale(&self, group: &str, key: &str) -> Option<Cow<str>> {
        let locale = current_locale::current_locale().ok();
        self.get(group, key, locale.as_deref())
    }

    pub fn get(&self, group: &str, key: &str, locale: Option<&str>) -> Option<Cow<str>> {
        let (default_value, value_map) = self.groups.get(group)?.get(key)?;
        if let Some(locale) = locale {
            if let Some(value) = value_map.get(locale) {
                return Some(Cow::from(value));
            }
            if let Some(pos) = locale.find('_') {
                if let Some(value) = value_map.get(&locale[..pos]) {
                    return Some(Cow::from(value));
                }
            }
        }
        if let Some(ubuntu_gettext_domain) = &self.ubuntu_gettext_domain {
            setlocale(LocaleCategory::LcAll, "");
            return Some(Cow::from(dgettext(ubuntu_gettext_domain, key)));
        }
        Some(Cow::from(default_value))
    }

    pub fn exec(&self) -> Option<&str> {
        self.get_without_locale("Desktop Entry", "Exec")
    }

    pub fn icon(&self) -> Option<&str> {
        self.get_without_locale("Desktop Entry", "Icon")
    }

    pub fn name(&self) -> Option<Cow<str>> {
        self.get_in_current_locale("Desktop Entry", "Name")
    }
}

pub fn scan_desktop_file(mut entries: ResMut<DesktopEntriesSet>, mut commands: Commands) {
    let dirs = freedesktop_desktop_entry::default_paths();
    let iter = freedesktop_desktop_entry::Iter::new(dirs);
    for path in iter {
        try_or!(
            {
                let data = std::fs::read_to_string(&path)?;
                let raw_entry = freedesktop_desktop_entry::DesktopEntry::decode(&path, &data)?;
                let entry = DesktopEntry::new(raw_entry);
                let mut entity_mut = commands.spawn_empty();
                if let Some(icon) = entry.icon() {
                    entity_mut.insert(Icon::new(icon));
                }
                entries.register(&entry, entity_mut.id());
                entity_mut.insert(entry);
                Result::<()>::Ok(())
            },
            "failed to load desktop entries",
            continue
        );
    }
}

relationship!(ToplevelConnectAppEntry=>AppRef>-WindowList);

pub fn attach_to_app(
    toplevel_query: Query<(Entity, &XdgToplevel), Changed<XdgToplevel>>,
    register: Res<DesktopEntriesSet>,
    mut commands: Commands,
) {
    toplevel_query.for_each(|(entity, toplevel)| {
        if let Some(app_id) = &toplevel.app_id {
            if let Some(entry_entity) = register.by_id.get(&*app_id) {
                commands
                    .entity(entity)
                    .connect_to::<ToplevelConnectAppEntry>(*entry_entity);
            }
        }
    })
}

pub struct DesktopEntriesPlugin;
impl Plugin for DesktopEntriesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DesktopEntry>();
        app.register_type::<Icon>();
        app.register_type::<DesktopEntriesSet>();
        app.init_resource::<DesktopEntriesSet>();
        app.init_resource::<IconLoader>();
        app.register_relation::<ToplevelConnectAppEntry>();
        app.add_startup_system(scan_desktop_file);
        app.add_system(attach_to_app.in_set(DWayServerSet::UpdateAppInfo));
    }
}
