#import bevy_ui::ui_vertex_output::UiVertexOutput
#import dway_ui::shapes::circleSDF
#import dway_ui::shapes::mix_color

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    color: vec4<f32>,
    radius: f32,
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let local_pos = (in.uv - 0.5) * rect.radius;
	let d = circleSDF(local_pos, rect.radius);
    return mix_color(rect.color, d);
}
