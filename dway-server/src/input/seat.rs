

use bevy_relationship::{relationship, AppExt};
use wayland_server::protocol::wl_seat::Capability;

use crate::{
    input::{keyboard::WlKeyboardBundle, pointer::WlPointerBundle, touch::WlTouchBundle},
    prelude::*,
    state::{create_global_system_config, EntityFactory}, wl::surface::WlSurface,
};

use super::{
    grab::{Grab, GrabPlugin},
    keyboard::WlKeyboard,
    pointer::WlPointer,
    touch::WlTouch,
};

#[derive(Component,Reflect,FromReflect)]
pub struct WlSeat {
    #[reflect(ignore)]
    pub raw: wl_seat::WlSeat,
    #[reflect(ignore)]
    pub grab_by: Option<wl_surface::WlSurface>,
    pub pointer_position: Option<IVec2>,
    pub enabled:bool,
}

impl WlSeat {
    pub fn new(raw: wl_seat::WlSeat) -> Self {
        Self {
            raw,
            grab_by: None,
            pointer_position: None,
            enabled: true,
        }
    }
    pub fn enable(&mut self){
        self.enabled=true;
    }
    pub fn disable(&mut self){
        self.enabled=false;
    }
    pub fn grab(&mut self, surface: &WlSurface) {
        debug!(surface=%WlResource::id(&surface.raw),"set grab");
        self.grab_by = Some(surface.raw.clone());
    }
    pub fn unset_grab(&mut self) {
        debug!("unset grab");
        self.grab_by = None;
    }
    pub fn grab_raw(&mut self, surface: &wl_surface::WlSurface) {
        debug!(surface=%WlResource::id(surface),"set grab");
        self.grab_by = Some(surface.clone());
    }
    pub fn can_focus_on(&mut self, surface: &WlSurface) -> bool {
        if let Some(s) = &self.grab_by {
            if s.is_alive() {
                s == &surface.raw
            } else {
                self.grab_by = None;
                true
            }
        } else {
            true
        }
    }
}
#[derive(Bundle)]
pub struct WlSeatBundle {
    pub seat: WlSeat,
    pub grab: Grab,
}

impl WlSeatBundle {
    pub fn new(seat: WlSeat) -> Self {
        Self {
            seat,
            grab: Default::default(),
        }
    }
}
relationship!(SeatHasPointer=>PointerList-<SeatOfPoint);
relationship!(SeatHasKeyboard=>KeyboardList-<SeatOfKeyboard);
relationship!(SeatHasTouch=>TouchList-<SeatOfTouch);
relationship!(FocusOnSurface=>FoucsOn--FocusBy);
relationship!(ActivePopup=>ActivePopupList-<PopupGrabBy);

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_seat::WlSeat: Entity] => SeatDelegate);

impl
    wayland_server::Dispatch<wayland_server::protocol::wl_seat::WlSeat, bevy::prelude::Entity, DWay>
    for SeatDelegate
{
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_seat::WlSeat,
        request: <wayland_server::protocol::wl_seat::WlSeat as WlResource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_seat::Request::GetPointer { id } => {
                let entity = state
                    .spawn(
                        (id, data_init, |o| WlPointerBundle::new(WlPointer::new(o)))
                            .with_parent(*data),
                    )
                    .id();
                state.connect::<SeatHasPointer>(*data, entity);
            }
            wl_seat::Request::GetKeyboard { id } => {
                let entity = state
                    .spawn(
                        (id, data_init, |kbd, world: &mut World| {
                            WlKeyboardBundle::new(WlKeyboard::new(kbd, world.resource()).unwrap())
                        })
                            .with_parent(*data),
                    )
                    .id();
                state.connect::<SeatHasKeyboard>(*data, entity);
            }
            wl_seat::Request::GetTouch { id } => {
                state.spawn(
                    (id, data_init, |o| WlTouchBundle::new(WlTouch::new(o))).with_parent(*data),
                );
            }
            wl_seat::Request::Release => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
impl wayland_server::GlobalDispatch<wayland_server::protocol::wl_seat::WlSeat, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wayland_server::protocol::wl_seat::WlSeat>,
        _global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            o.capabilities(Capability::all());
            WlSeatBundle::new(WlSeat::new(o))
        });
    }
}

pub struct WlSeatPlugin;
impl Plugin for WlSeatPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(create_global_system_config::<wl_seat::WlSeat, 7>());
        app.register_relation::<SeatHasPointer>();
        app.register_relation::<SeatHasKeyboard>();
        app.register_relation::<SeatHasTouch>();
        app.register_relation::<FocusOnSurface>();
        app.register_relation::<ActivePopup>();
        app.register_type::<WlPointer>();
        app.add_plugin(super::keyboard::WlKeyboardPlugin);
        app.add_plugin(GrabPlugin);
        app.register_type::<WlSeat>();
    }
}
