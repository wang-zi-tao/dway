#import bevy_ui::ui_vertex_output::UiVertexOutput

fn boxSDF(pos: vec2<f32>, size: vec2<f32>, cornerRadius: f32) -> f32 {
   let pos2 = abs(pos) - size * 0.5 + cornerRadius;
   return length(max(pos2, vec2(0.0))) + min(max(pos2.x, pos2.y), 0.0) - cornerRadius;
}

fn mix_color(color: vec4<f32>, value: f32) -> vec4<f32> {
    let alpha = max(min(1.0 - value, 1.0), 0.0);
    return vec4(color.xyz, alpha * color.w);
}

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

