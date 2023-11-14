#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::view::View

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    size: vec2<f32>,
    corner: f32,
}
@group(1) @binding(1) var<uniform> view: View;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
	let color = vec4(0.0);
    color += textureSample(image_texture, image_sampler, in.position + vec2(-0.5, 0.5));
    color += textureSample(image_texture, image_sampler, in.position + vec2(-0.5, -0.5));
    color += textureSample(image_texture, image_sampler, in.position + vec2(0.5, 0.5));
    color += textureSample(image_texture, image_sampler, in.position + vec2(0.5, -0.5));
    return color * 0.25;
}

