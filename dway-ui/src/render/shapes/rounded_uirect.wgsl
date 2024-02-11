#import bevy_ui::ui_vertex_output::UiVertexOutput
#import dway_ui::shapes::boxSDF
#import dway_ui::shapes::mix_color

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    color: vec4<f32>,
    size: vec2<f32>,
    corner: f32,
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let local_pos = (in.uv - 0.5) * rect.size;
	let d = boxSDF(local_pos, rect.size, rect.corner);
    return mix_color(rect.color, d);
}

