use std::{
    borrow::Borrow,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use bevy_math::Vec2;
use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use failure::{format_err, Error, Fail, Fallible};
use slog::{debug, error, warn};
use smithay::{
    backend::renderer::{
        element::{
            default_primary_scanout_output_compare, Id, RenderElementPresentationState,
            RenderElementState, RenderElementStates,
        },
        utils::{CommitCounter, RendererSurfaceStateUserData, RendererSurfaceState},
        Frame, ImportAll, ImportDma, ImportDmaWl, ImportEgl, ImportMem, ImportMemWl, Renderer,
        Texture,
    },
    desktop::{
        space::SpaceElement,
        utils::{
            surface_primary_scanout_output, update_surface_primary_scanout_output,
            with_surfaces_surface_tree,
        },
        PopupManager,
    },
    reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface, Resource},
    utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform},
    wayland::{
        compositor::{with_states, with_surface_tree_downward},
        fractional_scale::with_fractional_scale,
    },
};

use crate::math::rectangle_to_rect;

use super::{
    shell::WindowElement,
    surface::{get_component_locked, try_with_states_locked, with_states_locked, DWaySurfaceData, try_get_component_locked},
    DWayState,
};

// #[derive(Debug)]
// pub enum RenderError{
//     #[fail(display = "{}", _0)]
//     OtherError(#[cause] failure::Error),
// }
#[derive(Debug)]
pub enum RenderError {
    Other(failure::Error),
}
impl Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl std::error::Error for RenderError {}

#[derive(Clone, Default, Debug)]
pub struct DummyRenderer {
    pub images: Vec<(
        Rectangle<f64, Buffer>,
        DummyTexture,
        Rectangle<i32, Physical>,
        Vec<Rectangle<i32, Physical>>,
    )>,
    pub size: Size<i32, Physical>,
    pub transform: Transform,
    pub last_commit_map: HashMap<ObjectId, CommitCounter>,
}

#[derive(Clone)]
pub struct DummyTexture {
    pub size: Size<u32, Logical>,
    pub data: Vec<u8>,
}
impl std::fmt::Debug for DummyTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DummyTextureInner")
            .field("size", &self.size)
            .field("data_length", &self.data.len())
            .finish()
    }
}

impl Texture for DummyTexture {
    fn width(&self) -> u32 {
        self.size.w
    }

    fn height(&self) -> u32 {
        self.size.h
    }
}
impl ImportMem for DummyRenderer {
    fn import_memory(
        &mut self,
        data: &[u8],
        size: smithay::utils::Size<i32, smithay::utils::Buffer>,
        flipped: bool,
    ) -> Result<<Self as Renderer>::TextureId, <Self as Renderer>::Error> {
        todo!()
    }

    fn update_memory(
        &mut self,
        texture: &<Self as Renderer>::TextureId,
        data: &[u8],
        region: Rectangle<i32, smithay::utils::Buffer>,
    ) -> Result<(), <Self as Renderer>::Error> {
        todo!()
    }
}
impl ImportMemWl for DummyRenderer {
    fn import_shm_buffer(
        &mut self,
        buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
        surface: Option<&smithay::wayland::compositor::SurfaceData>,
        damage: &[Rectangle<i32, smithay::utils::Buffer>],
    ) -> Result<
        <Self as smithay::backend::renderer::Renderer>::TextureId,
        <Self as smithay::backend::renderer::Renderer>::Error,
    > {
        use smithay::wayland::shm::with_buffer_contents;
        let ret = with_buffer_contents(buffer, |slice, data| {
            let offset = data.offset as u32;
            let width = data.width as u32;
            let height = data.height as u32;
            let stride = data.stride as u32;
            let mut buffer = Vec::with_capacity((width * height * 4) as usize);
            for h in 0..height {
                buffer.extend_from_slice(
                    &slice[(offset + h * stride) as usize
                        ..(offset + h * stride + 4 * width) as usize],
                );
            }
            (width, height, buffer)
        });

        match ret {
            Ok((width, height, data)) => Ok(DummyTexture {
                size: (width, height).into(),
                data,
            }),
            Err(e) => Err(RenderError::Other(e.into())),
        }
    }
}
impl ImportEgl for DummyRenderer {
    fn bind_wl_display(
        &mut self,
        display: &smithay::reexports::wayland_server::DisplayHandle,
    ) -> Result<(), smithay::backend::egl::Error> {
        todo!()
    }

    fn unbind_wl_display(&mut self) {
        todo!()
    }

    fn egl_reader(&self) -> Option<&smithay::backend::egl::display::EGLBufferReader> {
        todo!()
    }

    fn import_egl_buffer(
        &mut self,
        buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
        surface: Option<&smithay::wayland::compositor::SurfaceData>,
        damage: &[Rectangle<i32, smithay::utils::Buffer>],
    ) -> Result<
        <Self as smithay::backend::renderer::Renderer>::TextureId,
        <Self as smithay::backend::renderer::Renderer>::Error,
    > {
        todo!()
    }
}
impl ImportDma for DummyRenderer {
    fn import_dmabuf(
        &mut self,
        dmabuf: &smithay::backend::allocator::dmabuf::Dmabuf,
        damage: Option<&[Rectangle<i32, smithay::utils::Buffer>]>,
    ) -> Result<<Self as Renderer>::TextureId, <Self as Renderer>::Error> {
        todo!()
    }
}
impl ImportDmaWl for DummyRenderer {}

pub struct DummyFrame<'r> {
    render: &'r mut DummyRenderer,
}
impl<'r> Frame for DummyFrame<'r> {
    type Error = RenderError;

    type TextureId = DummyTexture;

    fn id(&self) -> usize {
        0
    }

    fn clear(
        &mut self,
        color: [f32; 4],
        at: &[Rectangle<i32, smithay::utils::Physical>],
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render_texture_from_to(
        &mut self,
        texture: &Self::TextureId,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, smithay::utils::Physical>,
        damage: &[Rectangle<i32, smithay::utils::Physical>],
        src_transform: smithay::utils::Transform,
        alpha: f32,
    ) -> Result<(), Self::Error> {
        self.render
            .images
            .push((src, texture.clone(), dst, damage.to_vec()));
        Ok(())
    }

    fn transformation(&self) -> smithay::utils::Transform {
        Transform::Normal
    }

    fn finish(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Renderer for DummyRenderer {
    type Error = RenderError;

    type TextureId = DummyTexture;

    type Frame<'r> = DummyFrame<'r>;

    fn id(&self) -> usize {
        0
    }

    fn downscale_filter(
        &mut self,
        filter: smithay::backend::renderer::TextureFilter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn upscale_filter(
        &mut self,
        filter: smithay::backend::renderer::TextureFilter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn render(
        &mut self,
        output_size: smithay::utils::Size<i32, smithay::utils::Physical>,
        dst_transform: smithay::utils::Transform,
    ) -> Result<Self::Frame<'_>, Self::Error> {
        Ok(DummyFrame { render: self })
    }
}
pub fn render_surface(
    dway: &mut DWayState,
    surface: &WlSurface,
    geo: Rectangle<i32, Physical>,
    bbox: Rectangle<i32, Physical>,
) -> Fallible<RenderElementStates> {
    let scale = Scale { x: 1, y: 1 };
    let mut render_state = RenderElementStates {
        states: Default::default(),
    };
    let render = &mut dway.render;
    let Some( uuid ) = try_with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid)else{
        warn!(dway.log,"surface {:?} has no uuid",surface.id());
        return Ok(render_state);
    };
    with_states(&surface, |states| {
        let Some( surface_state ) = states
            .data_map
            .get::<RendererSurfaceStateUserData>()
            .map(|d|d.borrow()) else{
            return Ok(false);
        };
        let mut surface_data = get_component_locked::<DWaySurfaceData>(states);
        let last_commit = render.last_commit_map.get(&surface.id()).cloned();
        let damages = surface_state.damage_since(last_commit);
        render
            .last_commit_map
            .insert(surface.id(), surface_state.current_commit());
        if !surface_data.need_rerender && damages.is_empty() {
            let size = dway.output.physical_properties().size;
            render_state.states.insert(
                Id::from_wayland_resource(surface),
                RenderElementState {
                    visible_area: (size.w * size.h) as usize,
                    presentation_state: RenderElementPresentationState::Rendering,
                },
            );
            return Fallible::Ok(false);
        }
        surface_data.need_rerender = false;

        let texture = if let Some(buffer) = surface_state.wl_buffer() {
            let buffer = render.import_buffer(buffer, Some(states), &damages);
            match buffer {
                Some(Ok(m)) => m,
                Some(Err(err)) => {
                    slog::error!(dway.log, "Error loading buffer: {}", err);
                    return Err(err.into());
                }
                None => {
                    slog::error!(dway.log, "Unknown buffer format for: {:?}", buffer);
                    return Fallible::Ok(false);
                }
            }
        } else {
            slog::warn!(dway.log, "no buffer on {:?}", surface.id());
            return Fallible::Ok(false);
        };
        render_state.states.insert(
            Id::from_wayland_resource(surface),
            RenderElementState {
                visible_area: (texture.size.w * texture.size.h) as usize,
                presentation_state: RenderElementPresentationState::Rendering,
            },
        );

        dway.sender.send(WindowMessage {
            uuid,
            time: SystemTime::now(),
            data: WindowMessageKind::UpdateImage {
                image: ImageBuffer(
                    Vec2::new(texture.size.w as f32, texture.size.h as f32),
                    texture.data,
                ),
                geo: rectangle_to_rect(geo.to_f64()),
                bbox: rectangle_to_rect(bbox.to_f64()),
            },
        })?;
        Fallible::Ok(true)
    })?;
    dway.render.images.clear();
    Ok(render_state)
}

pub fn render_element(dway: &mut DWayState, element: &WindowElement) -> Fallible<()> {
    let Some(surface)=element.wl_surface()else{
        // slog::debug!(dway.log, "no wl_surface on {:?}",element.id());
        return Ok(());
    };
    let mut render_state = RenderElementStates {
        states: Default::default(),
    };
    let scale = Scale { x: 1, y: 1 };
    // let geo = element.geometry().to_physical(scale);
    // let bbox = element.bbox().to_physical(scale);
    let (geo, bbox) = match element {
        WindowElement::Wayland(w) => {
            DWaySurfaceData::get_physical_geometry_bbox(&element).unwrap_or_default()
        }
        WindowElement::X11(w) => {
            let geo = w.geometry().to_physical(scale);
            (geo, geo)
        }
    };
    // dbg!((geo,bbox));
    for (popup, popup_offset) in PopupManager::popups_for_surface(&surface) {
        let offset: Point<i32, Physical> = (element.geometry().loc + popup_offset
            - popup.geometry().loc)
            .to_physical_precise_round(Scale { x: 1.0, y: 1.0 });
        let geo = Rectangle::from_loc_and_size(geo.loc + offset, geo.size);
        let bbox = Rectangle::from_loc_and_size(bbox.loc + offset, bbox.size);
        let render_result = render_surface(dway, popup.wl_surface(), geo, bbox)?;
        render_state.states.extend(render_result.states.into_iter());
    }
    let render_result = render_surface(dway, &surface, geo, bbox)?;
    render_state.states.extend(render_result.states.into_iter());
    // with_surfaces_surface_tree(&surface, |surface, state| {
    //     if let Err(e) = render_surface(dway, &element, &surface, geo, bbox) {
    //         error!(dway.log, "error while render {:?} : {:?}", surface.id(), e);
    //     }
    // });
    element.with_surfaces(|surface, states| {
        let primary_scanout_output = update_surface_primary_scanout_output(
            surface,
            &dway.output,
            states,
            &render_state,
            default_primary_scanout_output_compare,
        );

        if let Some(output) = primary_scanout_output {
            with_fractional_scale(states, |fraction_scale| {
                fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
            });
        }
    });
    element.send_frame(
        &dway.output,
        dway.clock.now(),
        Some(Duration::from_secs(1)),
        surface_primary_scanout_output,
    );
    Ok(())
}
pub fn render_desktop(dway: &mut DWayState) -> Fallible<()> {
    for element in dway.element_map.values().cloned().collect::<Vec<_>>() {
        render_element(dway, &element)?;
    }
    Ok(())
}
