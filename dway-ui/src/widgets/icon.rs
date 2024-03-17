use crate::prelude::*;
use bevy::utils::HashSet;
use dway_server::apps::icon::{LinuxIcon, LinuxIconKind};
use dway_ui_framework::{
    make_bundle,
    render::mesh::{UiMesh, UiMeshHandle},
    widgets::svg::UiSvgExt,
};

#[derive(Component, Reflect, Debug, Default)]
pub struct UiIcon {
    pub handle: Handle<LinuxIcon>,
}
impl From<Handle<LinuxIcon>> for UiIcon {
    fn from(value: Handle<LinuxIcon>) -> Self {
        Self { handle: value }
    }
}

make_bundle! {
    @from icon: UiIcon,
    @addon UiIconExt,
    UiIconBundle{
        pub icon: UiIcon,
        pub svg: UiSvgExt,
        pub image: UiImage,
    }
}

pub fn uiicon_render(
    mut uiicon_query: Query<(
        Entity,
        Ref<UiIcon>,
        &mut UiImage,
        &mut UiSvg,
        &mut UiMeshHandle,
    )>,
    icons: Res<Assets<LinuxIcon>>,
    mut padding_entity: Local<HashSet<Entity>>,
) {
    uiicon_query.for_each_mut(
        |(e, icon, mut image, mut svg, mut mesh)| {
            if !icon.is_changed() && padding_entity.is_empty() && !padding_entity.remove(&e){
                return
            };
            if let Some(linux_icon) = icons.get(icon.handle.id()) {
                match &linux_icon.handle {
                    LinuxIconKind::Image(h) => {
                        if &image.texture != h {
                            image.texture = h.clone();
                            svg.set_if_neq(Default::default());
                            mesh.set_if_neq(Default::default());
                        }
                    }
                    LinuxIconKind::Svg(h) => {
                        if &**svg != h {
                            *svg = h.clone().into();
                            if image.texture != Handle::<Image>::default() {
                                image.texture = Default::default();
                            }
                        }
                    }
                };
            } else {
                padding_entity.insert(e);
            }
        },
    );
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
