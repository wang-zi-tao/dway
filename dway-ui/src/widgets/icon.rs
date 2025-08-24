use bevy::{ecs::entity::EntityHashSet};
use dway_server::apps::icon::{LinuxIcon, LinuxIconKind};

use crate::prelude::*;

#[derive(Component, Reflect, Debug, Default)]
#[require(Node, UiSvg)]
pub struct UiIcon {
    pub handle: Handle<LinuxIcon>,
}
impl From<Handle<LinuxIcon>> for UiIcon {
    fn from(value: Handle<LinuxIcon>) -> Self {
        Self { handle: value }
    }
}

#[allow(clippy::type_complexity)]
pub fn uiicon_render(
    mut uiicon_query: Query<(
        Entity,
        Ref<UiIcon>,
        Option<&mut ImageNode>,
        &mut UiSvg,
        &mut UiMeshHandle,
    )>,
    icons: Res<Assets<LinuxIcon>>,
    mut padding_entity: Local<EntityHashSet>,
    mut commands: Commands,
) {
    for (e, icon, image, mut svg, mut mesh) in uiicon_query.iter_mut() {
        if !icon.is_changed() && padding_entity.is_empty() && !padding_entity.remove(&e) {
            continue;
        };
        if let Some(linux_icon) = icons.get(icon.handle.id()) {
            match &linux_icon.handle {
                LinuxIconKind::Image(h) => {
                    if let Some(mut image) = image {
                        if &image.image != h {
                            image.image = h.clone();
                            svg.set_if_neq(Default::default());
                            mesh.set_if_neq(Default::default());
                        }
                    } else {
                        commands.entity(e).insert(ImageNode::new(h.clone()));
                    }
                }
                LinuxIconKind::Svg(h) => {
                    if &**svg != h {
                        *svg = h.clone().into();
                        if image.is_some(){
                            commands.entity(e).remove::<ImageNode>();
                        }
                    }
                }
            };
        } else {
            padding_entity.insert(e);
        }
    }
}

pub struct UiIconPlugin;
impl Plugin for UiIconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            uiicon_render.before(UiFrameworkSystems::UpdateWidgets),
        )
        .register_type::<UiIcon>();
    }
}
