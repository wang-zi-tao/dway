use std::ops::Range;

use bevy::{
    core::{Pod, Zeroable},
    ecs::system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        render_asset::RenderAssets,
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{BindGroup, BufferVec, CachedRenderPipelineId},
        renderer::{RenderAdapter, RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract, RenderApp,
    },
    utils::{FloatOrd, HashMap},
};
use dway_server::{
    components::{GlobalPhysicalRect, WlSurfaceWrapper},
    egl::import_wl_surface,
    surface::ImportedSurface,
};

pub struct DWayRender;
impl Plugin for DWayRender {
    fn build(&self, app: &mut App) {
        let render_app: &mut App = app.sub_app_mut(RenderApp);
        // app.add_plugin(ExtractComponentPlugin::<WlSurfaceWrapper>::extract_visible());

        render_app.init_resource::<DrawFunctions<WindowRenderPhase>>();
        // render_app.add_render_command::<WindowRenderPhase, DrawWindowNode>();
        render_app.add_system(prepare_window.in_schedule(ExtractSchedule));
    }
}

pub struct WindowRenderPhase {
    pub sort_key: FloatOrd,
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}
impl PhaseItem for WindowRenderPhase {
    type SortKey = FloatOrd;

    fn entity(&self) -> Entity {
        self.entity
    }

    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}
impl CachedRenderPipelinePhaseItem for WindowRenderPhase {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct ExtractedWindow {
    image: GpuImage,
    rect: GlobalPhysicalRect,
    z: usize,
}
#[derive(Resource)]
pub struct ExtractedWindows {
    pub windows: Vec<ExtractedWindow>,
}
pub fn extract_window(
    mut extracted_windows: ResMut<ExtractedWindows>,
    windows_query: Extract<
        Query<(
            &WlSurfaceWrapper,
            &GlobalPhysicalRect,
            &ImportedSurface,
            &ComputedVisibility,
        )>,
    >,
) {
    extracted_windows.windows.clear();
}
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct WindowVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}
#[derive(Resource)]
pub struct WindowsMeta {
    vertices: BufferVec<WindowVertex>,
    view_bind_group: Option<BindGroup>,
}
#[derive(Component)]
pub struct WindowBatch {
    pub range: Range<u32>,
    pub image: GpuImage,
    pub bind_group: BindGroup,
    pub z: f32,
}
pub fn prepare_window(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,

    mut extracted_windows: ResMut<ExtractedWindows>,
) {
    extracted_windows.windows.sort_by_key(|w| w.z);
    for extra in &extracted_windows.windows {}
}

#[derive(Resource, Default)]
pub struct WindowBindGroups {
    pub values: HashMap<GpuImage, BindGroup>,
}
pub type DrawWindow = (SetItemPipeline, DrawWindowNode<0>);
pub struct DrawWindowNode<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for DrawWindowNode<I> {
    type Param = SRes<WindowsMeta>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<WindowBatch>;
    #[inline]
    fn render<'w>(
        item: &P,
        view: bevy::ecs::query::ROQueryItem<'w, Self::ViewWorldQuery>,
        batch: bevy::ecs::query::ROQueryItem<'w, Self::ItemWorldQuery>,
        window_meta: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        pass.set_bind_group(I, &batch.bind_group, &[]);
        pass.set_vertex_buffer(
            0,
            window_meta
                .into_inner()
                .vertices
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}
