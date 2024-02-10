#import bevy_ui::ui_vertex_output::UiVertexOutput
#import dway_ui::shapes::boxSDF
#import dway_ui::shapes::mix_color

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    min_uv: vec2<f32>,
    size_uv: vec2<f32>,
    size: vec2<f32>,
    corner: f32,
}
@group(1) @binding(1) var image_texture: texture_2d<f32>;
@group(1) @binding(2) var image_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let local_pos = (in.uv - 0.5) * rect.size;
	let d = boxSDF(local_pos, rect.size, rect.corner);
	let uv = in.uv * rect.size_uv + rect.min_uv;
    let image_color = textureSample(image_texture, image_sampler, uv);
    return mix_color(image_color, d);
}

