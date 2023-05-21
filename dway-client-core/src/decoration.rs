use bevy::prelude::*;
use dway_server::{
    components::{
        GlobalPhysicalRect, SurfaceId, SurfaceOffset, WaylandWindow, WindowDecoration, WindowIndex,
        WindowMark, WlSurfaceWrapper,
    },
    events::CreateWindow,
    surface::ImportedSurface,
};

#[derive(Default)]
pub struct DWayDecorationPlugin {}
impl Plugin for DWayDecorationPlugin {
    fn build(&self, app: &mut App) {
        use DWayClientSystem::*;
        app.add_system(
            add_decoration
                .run_if(on_event::<CreateWindow>())
                .in_set(CreateComponent),
        );
    }
}

use crate::{
    window::{Frontends, WindowUiRoot},
    DWayClientSystem,
};

pub fn add_decoration(
    mut events: EventReader<CreateWindow>,
    mut commands: Commands,
    window_index: Res<WindowIndex>,
    surface_query: Query<
        (Entity, &SurfaceId, &Frontends, Option<&WindowDecoration>),
        With<WindowMark>,
    >,
    mut frontend_query: Query<&WindowUiRoot, Added<WindowUiRoot>>,
    mut style_query: Query<(Entity, &mut Style)>,
) {
    for CreateWindow(id) in events.iter() {
        if let Some((entity, id, frontends, decoration)) = window_index.query(id, &surface_query) {
            if let Some(decoraration) = decoration {
                for frontend_entity in frontends.iter() {
                    let Ok(ui) = frontend_query.get_mut(*frontend_entity)else {
                        continue;
                    };
                    let Ok((entity,  mut style)) = style_query.get_mut(ui.input_rect_entity)else {
                        continue;
                    };
                    style.flex_direction = FlexDirection::ColumnReverse;
                    commands.entity(entity).with_children(|c| {
                        c.spawn(NodeBundle {
                            style: Style {
                                size: Size::new(Default::default(), Val::Px(32.0)),
                                position_type: PositionType::Absolute,
                                position: UiRect {
                                    top: Val::Px(-32.0),
                                    ..Default::default()
                                },
                                margin: UiRect {
                                    bottom: Val::Px(32.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            background_color: Color::ANTIQUE_WHITE.into(),
                            ..Default::default()
                        });
                    });
                }
            }
        }
    }
}
