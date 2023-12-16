#import bevy_ui::ui_vertex_output::UiVertexOutput
#import dway_ui::shapes::circleSDF

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    color: vec4<f32>,
    radius: f32,
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
	let d = circleSDF(in.uv, rect.radius);

    return vec4(rect.color.xyz, rect.color.w * max(min(0.5-d,1.0),0.0));
}
