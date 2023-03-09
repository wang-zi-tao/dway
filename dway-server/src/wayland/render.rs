use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    rc::Rc,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use failure::Fallible;
use slog::{error, trace, warn};
use smithay::{
    backend::{
        drm::{DrmError, DrmNode},
        renderer::{
            damage::{
                DamageTrackedRenderer, DamageTrackedRendererError, DamageTrackedRendererMode,
            },
            element::{
                default_primary_scanout_output_compare,
                surface::WaylandSurfaceRenderElement,
                texture::TextureBuffer,
                utils::{
                    ConstrainAlign, ConstrainScaleBehavior, CropRenderElement,
                    RelocateRenderElement, RescaleRenderElement,
                },
                AsRenderElements, Id, RenderElementPresentationState, RenderElementState,
                RenderElementStates,
            },
            gles2::Gles2Renderbuffer,
            multigpu::MultiTexture,
            utils::{CommitCounter, RendererSurfaceStateUserData},
            Bind, Frame, ImportAll, ImportDma, ImportDmaWl, ImportEgl, ImportMem, ImportMemWl,
            Renderer, Texture,
        },
        SwapBuffersError,
    },
    desktop::{
        self,
        space::{
            constrain_space_element, ConstrainBehavior, ConstrainReference, SpaceElement,
            SurfaceTree,
        },
        utils::{surface_primary_scanout_output, update_surface_primary_scanout_output},
        PopupManager, Space,
    },
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::Output,
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        drm::{self, control::crtc},
        wayland_server::{
            backend::ObjectId,
            protocol::wl_surface::{self, WlSurface},
            Resource,
        },
    },
    utils::{
        Buffer, Clock, IsAlive, Logical, Monotonic, Physical, Point, Rectangle, Scale, Size,
        Transform,
    },
    wayland::{
        compositor::{self, with_states},
        fractional_scale::with_fractional_scale,
        input_method::{InputMethodHandle, InputMethodSeat},
    },
};

use crate::math::rectangle_to_rect;

use super::{
    backend::{
        udev::{
            self, post_repaint, take_presentation_feedback, SurfaceData, UdevOutputId, UdevRenderer,
        },
        Backend,
    },
    cursor::{PointerElement, PointerRenderElement, CLEAR_COLOR},
    shell::{FullscreenSurface, WindowElement, WindowRenderElement},
    surface::{get_component_locked, try_with_states_locked, DWaySurfaceData},
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
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        _data: &[u8],
        _size: smithay::utils::Size<i32, smithay::utils::Buffer>,
        _flipped: bool,
    ) -> Result<<Self as Renderer>::TextureId, <Self as Renderer>::Error> {
        todo!()
    }

    fn update_memory(
        &mut self,
        _texture: &<Self as Renderer>::TextureId,
        _data: &[u8],
        _region: Rectangle<i32, smithay::utils::Buffer>,
    ) -> Result<(), <Self as Renderer>::Error> {
        todo!()
    }
}
impl ImportMemWl for DummyRenderer {
    fn import_shm_buffer(
        &mut self,
        buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
        _surface: Option<&smithay::wayland::compositor::SurfaceData>,
        _damage: &[Rectangle<i32, smithay::utils::Buffer>],
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
        _display: &smithay::reexports::wayland_server::DisplayHandle,
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
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
        _surface: Option<&smithay::wayland::compositor::SurfaceData>,
        _damage: &[Rectangle<i32, smithay::utils::Buffer>],
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
        // let dmabuf=dmabuf.import_to(gbm, usage);
        use smithay::wayland::shm::with_buffer_contents;
        // let ret = with_buffer_contents(buffer, |slice, data| {
        //     let offset = data.offset as u32;
        //     let width = data.width as u32;
        //     let height = data.height as u32;
        //     let stride = data.stride as u32;
        //     let mut buffer = Vec::with_capacity((width * height * 4) as usize);
        //     for h in 0..height {
        //         buffer.extend_from_slice(
        //             &slice[(offset + h * stride) as usize
        //                 ..(offset + h * stride + 4 * width) as usize],
        //         );
        //     }
        //     (width, height, buffer)
        // });

        // match ret {
        //     Ok((width, height, data)) => ,
        //     Err(e) => Err(RenderError::Other(e.into())),
        // }
        Ok(DummyTexture {
            size: (1, 1).into(),
            data: vec![0, 0, 0, 0],
        })
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
        _color: [f32; 4],
        _at: &[Rectangle<i32, smithay::utils::Physical>],
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render_texture_from_to(
        &mut self,
        texture: &Self::TextureId,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, smithay::utils::Physical>,
        damage: &[Rectangle<i32, smithay::utils::Physical>],
        _src_transform: smithay::utils::Transform,
        _alpha: f32,
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
        _filter: smithay::backend::renderer::TextureFilter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn upscale_filter(
        &mut self,
        _filter: smithay::backend::renderer::TextureFilter,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn render(
        &mut self,
        _output_size: smithay::utils::Size<i32, smithay::utils::Physical>,
        _dst_transform: smithay::utils::Transform,
    ) -> Result<Self::Frame<'_>, Self::Error> {
        Ok(DummyFrame { render: self })
    }

    fn set_debug_flags(&mut self, flags: smithay::backend::renderer::DebugFlags) {
        todo!()
    }

    fn debug_flags(&self) -> smithay::backend::renderer::DebugFlags {
        todo!()
    }
}
pub fn render_surface(
    dway: &mut DWayState,
    surface: &WlSurface,
    geo: Rectangle<i32, Physical>,
    bbox: Rectangle<i32, Physical>,
) -> Fallible<RenderElementStates> {
    let _scale = Scale { x: 1, y: 1 };
    let mut render_state = RenderElementStates {
        states: Default::default(),
    };
    let render = &mut dway.render;
    let Some( uuid ) = try_with_states_locked(surface, |s: &mut DWaySurfaceData| s.uuid)else{
        warn!(dway.log,"surface {:?} has no uuid",surface.id());
        return Ok(render_state);
    };
    // with_states(surface, |states| {
    //     let Some( surface_state ) = states
    //         .data_map
    //         .get::<RendererSurfaceStateUserData>()
    //         .map(|d|d.borrow()) else{
    //         return Ok(false);
    //     };
    //     let mut surface_data = get_component_locked::<DWaySurfaceData>(states);
    //     let last_commit = render.last_commit_map.get(&surface.id()).cloned();
    //     let damages = surface_state.damage_since(last_commit);
    //     render
    //         .last_commit_map
    //         .insert(surface.id(), surface_state.current_commit());
    //     if !surface_data.need_rerender && damages.is_empty() {
    //         let size = dway.output.physical_properties().size;
    //         render_state.states.insert(
    //             Id::from_wayland_resource(surface),
    //             RenderElementState {
    //                 visible_area: (size.w * size.h) as usize,
    //                 presentation_state: RenderElementPresentationState::Rendering,
    //             },
    //         );
    //         return Fallible::Ok(false);
    //     }
    //     surface_data.need_rerender = false;
    //
    //     let texture = if let Some(buffer) = surface_state.wl_buffer {
    //         let buffer = render.import_buffer(buffer, Some(states), &damages);
    //         match buffer {
    //             Some(Ok(m)) => m,
    //             Some(Err(err)) => {
    //                 slog::error!(dway.log, "Error loading buffer: {}", err);
    //                 return Err(err.into());
    //             }
    //             None => {
    //                 slog::error!(dway.log, "Unknown buffer format for: {:?}", buffer);
    //                 return Fallible::Ok(false);
    //             }
    //         }
    //     } else {
    //         slog::warn!(dway.log, "no buffer on {:?}", surface.id());
    //         return Fallible::Ok(false);
    //     };
    //     render_state.states.insert(
    //         Id::from_wayland_resource(surface),
    //         RenderElementState {
    //             visible_area: (texture.size.w * texture.size.h) as usize,
    //             presentation_state: RenderElementPresentationState::Rendering,
    //         },
    //     );
    //
    //     dway.sender.send(WindowMessage {
    //         uuid,
    //         time: SystemTime::now(),
    //         data: WindowMessageKind::UpdateImage {
    //             image: ImageBuffer(
    //                 Vec2::new(texture.size.w as f32, texture.size.h as f32),
    //                 texture.data,
    //             ),
    //             geo: rectangle_to_rect(geo.to_f64()),
    //             bbox: rectangle_to_rect(bbox.to_f64()),
    //         },
    //     })?;
    //     Fallible::Ok(true)
    // })?;
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
        WindowElement::Wayland(_w) => {
            DWaySurfaceData::get_physical_geometry_bbox(element).unwrap_or_default()
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

smithay::backend::renderer::element::render_elements! {
    pub CustomRenderElements<R> where
        R: ImportAll + ImportMem;
    Pointer=PointerRenderElement<R>,
    Surface=WaylandSurfaceRenderElement<R>,
    Window=WindowRenderElement<R>,
    #[cfg(feature = "debug")]
    // Note: We would like to borrow this element instead, but that would introduce
    // a feature-dependent lifetime, which introduces a lot more feature bounds
    // as the whole type changes and we can't have an unused lifetime (for when "debug" is disabled)
    // in the declaration.
    Fps=FpsElement<<R as Renderer>::TextureId>,
}
smithay::backend::renderer::element::render_elements! {
    pub OutputRenderElements<'a, R> where
        R: ImportAll + ImportMem;
    Custom=&'a CustomRenderElements<R>,
    Preview=CropRenderElement<RelocateRenderElement<RescaleRenderElement<WindowRenderElement<R>>>>,
}

pub fn render_output<'a, R>(
    output: &Output,
    space: &'a Space<WindowElement>,
    custom_elements: &'a [CustomRenderElements<R>],
    renderer: &mut R,
    damage_tracked_renderer: &mut DamageTrackedRenderer,
    age: usize,
    show_window_preview: bool,
    log: &slog::Logger,
) -> Result<
    (Option<Vec<Rectangle<i32, Physical>>>, RenderElementStates),
    DamageTrackedRendererError<R>,
>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let output_scale = output.current_scale().fractional_scale().into();

    if let Some(window) = output
        .user_data()
        .get::<FullscreenSurface>()
        .and_then(|f| f.get())
    {
        if let DamageTrackedRendererMode::Auto(renderer_output) = damage_tracked_renderer.mode() {
            assert!(renderer_output == output);
        }

        let window_render_elements =
            AsRenderElements::<R>::render_elements(&window, renderer, (0, 0).into(), output_scale);

        let render_elements = custom_elements
            .iter()
            .chain(window_render_elements.iter())
            .collect::<Vec<_>>();

        damage_tracked_renderer.render_output(
            renderer,
            age,
            &render_elements,
            CLEAR_COLOR,
        )
    } else {
        let mut output_render_elements = custom_elements
            .iter()
            .map(OutputRenderElements::from)
            .collect::<Vec<_>>();

        if show_window_preview && space.elements_for_output(output).count() > 0 {
            let constrain_behavior = ConstrainBehavior {
                reference: ConstrainReference::BoundingBox,
                behavior: ConstrainScaleBehavior::Fit,
                align: ConstrainAlign::CENTER,
            };

            let preview_padding = 10;

            let elements_on_space = space.elements_for_output(output).count();
            let output_scale = output.current_scale().fractional_scale();
            let output_transform = output.current_transform();
            let output_size = output
                .current_mode()
                .map(|mode| {
                    output_transform
                        .transform_size(mode.size)
                        .to_f64()
                        .to_logical(output_scale)
                })
                .unwrap_or_default();

            let max_elements_per_row = 4;
            let elements_per_row = usize::min(elements_on_space, max_elements_per_row);
            let rows = f64::ceil(elements_on_space as f64 / elements_per_row as f64);

            let preview_size = Size::from((
                f64::round(output_size.w / elements_per_row as f64) as i32 - preview_padding * 2,
                f64::round(output_size.h / rows) as i32 - preview_padding * 2,
            ));

            output_render_elements.extend(space.elements_for_output(output).enumerate().flat_map(
                |(element_index, window)| {
                    let column = element_index % elements_per_row;
                    let row = element_index / elements_per_row;
                    let preview_location = Point::from((
                        preview_padding + (preview_padding + preview_size.w) * column as i32,
                        preview_padding + (preview_padding + preview_size.h) * row as i32,
                    ));
                    let constrain = Rectangle::from_loc_and_size(preview_location, preview_size);
                    constrain_space_element(
                        renderer,
                        window,
                        preview_location,
                        output_scale,
                        constrain,
                        constrain_behavior,
                    )
                },
            ));
        }

        desktop::space::render_output(
            output,
            renderer,
            age,
            [space],
            &output_render_elements,
            damage_tracked_renderer,
            CLEAR_COLOR,
        )
    }
}

pub fn render(dway: &mut DWayState, dev_id: DrmNode, crtc: Option<crtc::Handle>) {
    let now = dway.clock.now().try_into().unwrap();
    let udev = {
        match &mut dway.backend {
            // Backend::UDev(u) => u,
            Backend::Winit(_) => panic!(),
            Backend::Headless => panic!(),
        }
    };
    let device_backend = match udev.backends.get(&dev_id) {
        Some(backend) => backend,
        None => {
            error!(
                dway.log,
                "Trying to render on non-existent backend {}", dev_id
            );
            return;
        }
    };
    // setup two iterators on the stack, one over all surfaces for this backend, and
    // one containing only the one given as argument.
    // They make a trait-object to dynamically choose between the two
    let surfaces = device_backend.surfaces.borrow();
    let surfaces_iter = surfaces.iter();
    let option_iter = crtc
        .iter()
        .flat_map(|crtc| surfaces.get(crtc).map(|surface| (crtc, surface)));

    let to_render_iter: Vec<(&crtc::Handle, &Rc<RefCell<SurfaceData>>)> = if crtc.is_some() {
        option_iter.collect()
    } else {
        surfaces_iter.collect()
    };

    for (&crtc, surface) in to_render_iter {
        // TODO get scale from the rendersurface when supporting HiDPI
        let frame = udev.pointer_image.get_image(1 /*scale*/, now);
        let primary_gpu = udev.primary_gpu;
        let mut renderer = udev
            .gpus
            .renderer::<Gles2Renderbuffer>(&primary_gpu, &surface.borrow().render_node)
            .unwrap();
        let pointer_images = &mut udev.pointer_images;
        let pointer_image = pointer_images
            .iter()
            .find_map(|(image, texture)| {
                if image == &frame {
                    Some(texture.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let texture = TextureBuffer::from_memory(
                    &mut renderer,
                    &frame.pixels_rgba,
                    (frame.width as i32, frame.height as i32),
                    false,
                    1,
                    Transform::Normal,
                    None,
                )
                .expect("Failed to import cursor bitmap");
                pointer_images.push((frame, texture.clone()));
                texture
            });
        let device_id = { surface.borrow().device_id };

        let output = if let Some(output) = dway.space.outputs().find(|o| {
            o.user_data().get::<UdevOutputId>() == Some(&UdevOutputId { device_id, crtc })
        }) {
            output.clone()
        } else {
            // somehow we got called with an invalid output
            continue;
        };

        let result = {
            let surface: &mut SurfaceData = &mut surface.borrow_mut();
            let input_method = dway.seat.input_method().unwrap();
            let pointer_element: &mut PointerElement<MultiTexture> = &mut udev.pointer_element;
            let cursor_status: &mut CursorImageStatus = &mut dway.cursor_status.lock().unwrap();
            let output_geometry = (&dway.space).output_geometry(&output).unwrap();
            let scale = Scale::from((&output).current_scale().fractional_scale());

            let (dmabuf, age) = surface.surface.next_buffer().unwrap();
            (&mut renderer).bind(dmabuf).unwrap();

            let mut elements: Vec<CustomRenderElements<_>> = Vec::new();
            // draw input method surface if any
            let rectangle = input_method.coordinates();
            let position = Point::from((
                rectangle.loc.x + rectangle.size.w,
                rectangle.loc.y + rectangle.size.h,
            ));
            input_method.with_surface(|surface| {
                elements.extend(AsRenderElements::<UdevRenderer<'_>>::render_elements(
                    &SurfaceTree::from_surface(surface),
                    &mut renderer,
                    position.to_physical_precise_round(scale),
                    scale,
                ));
            });

            if output_geometry.to_f64().contains(dway.pointer_location) {
                let cursor_hotspot = if let CursorImageStatus::Surface(ref surface) = cursor_status
                {
                    compositor::with_states(surface, |states| {
                        states
                            .data_map
                            .get::<Mutex<CursorImageAttributes>>()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .hotspot
                    })
                } else {
                    (0, 0).into()
                };
                let cursor_pos =
                    dway.pointer_location - output_geometry.loc.to_f64() - cursor_hotspot.to_f64();
                let cursor_pos_scaled = cursor_pos.to_physical(scale).to_i32_round();

                // set cursor
                pointer_element.set_texture((&pointer_image).clone());

                // draw the cursor as relevant
                {
                    // reset the cursor if the surface is no longer alive
                    let mut reset = false;
                    if let CursorImageStatus::Surface(ref surface) = *cursor_status {
                        reset = !surface.alive();
                    }
                    if reset {
                        *cursor_status = CursorImageStatus::Default;
                    }

                    pointer_element.set_status(cursor_status.clone());
                }

                elements.extend(pointer_element.render_elements(
                    &mut renderer,
                    cursor_pos_scaled,
                    scale,
                ));

                // draw the dnd icon if applicable
                {
                    if let Some(wl_surface) = (&dway.dnd_icon).as_ref() {
                        if wl_surface.alive() {
                            elements.extend(AsRenderElements::<UdevRenderer<'_>>::render_elements(
                                &SurfaceTree::from_surface(wl_surface),
                                &mut renderer,
                                cursor_pos_scaled,
                                scale,
                            ));
                        }
                    }
                }
            }

            // and draw to our buffer
            let (rendered, states) = render_output(
                &output,
                &dway.space,
                &elements,
                &mut renderer,
                &mut surface.damage_tracked_renderer,
                age.into(),
                true,
                &dway.log,
            )
            .map(|(damage, states)| (damage.is_some(), states))
            .map_err(|err| match err {
                DamageTrackedRendererError::Rendering(err) => SwapBuffersError::from(err),
                _ => unreachable!(),
            })
            .unwrap();

            post_repaint(&output, &states, &dway.space, (&dway.clock).now());

            if rendered {
                let output_presentation_feedback =
                    take_presentation_feedback(&output, &dway.space, &states);
                surface
                    .surface
                    .queue_buffer(Some(output_presentation_feedback))
                    .map_err(Into::<SwapBuffersError>::into)
                    .unwrap();
            }

            Ok(rendered)
        };
        let reschedule = match &result {
            Ok(has_rendered) => !has_rendered,
            Err(err) => {
                warn!(dway.log, "Error during rendering: {:?}", err);
                match err {
                    SwapBuffersError::AlreadySwapped => false,
                    SwapBuffersError::TemporaryFailure(err) => !matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(&DrmError::DeviceInactive)
                            | Some(&DrmError::Access {
                                source: drm::SystemError::PermissionDenied,
                                ..
                            })
                    ),
                    SwapBuffersError::ContextLost(err) => {
                        panic!("Rendering loop lost: {}", err)
                    }
                }
            }
        };

        if reschedule {
            let output_refresh = match output.current_mode() {
                Some(mode) => mode.refresh,
                None => return,
            };
            // If reschedule is true we either hit a temporary failure or more likely rendering
            // did not cause any damage on the output. In this case we just re-schedule a repaint
            // after approx. one frame to re-test for damage.
            let reschedule_duration =
                Duration::from_millis((1_000_000f32 / output_refresh as f32) as u64);
            trace!(
                dway.log,
                "reschedule repaint timer with delay {:?} on {:?}",
                reschedule_duration,
                crtc,
            );
            let timer = Timer::from_duration(reschedule_duration);
            dway.handle
                .insert_source(timer, move |_, _, data| {
                    render(&mut data.state, dev_id, Some(crtc));
                    TimeoutAction::Drop
                })
                .expect("failed to schedule frame timer");
        }
    }
}
