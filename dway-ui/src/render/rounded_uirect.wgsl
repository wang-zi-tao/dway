#import bevy_ui::ui_vertex_output::UiVertexOutput
#import dway_ui::shapes::boxSDF

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    color: vec4<f32>,
    size: vec2<f32>,
    corner: f32,
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
	let d = boxSDF(in.uv, rect.size, rect.corner);

    return rect.color * max(min(1.0-d,1.0),0.0);
}

