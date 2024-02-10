#import dway_ui::shapes::boxSDF
#import dway_ui::shapes::mix_color
#import dway_ui::shapes::mix_inner_color
#import dway_ui::shapes::sigmoid
#import dway_ui::shapes::sdf_visualition
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::view::View

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<uniform> rect: Settings;
struct Settings {
    color: vec4<f32>,
    size: vec2<f32>,
    corner: f32,

    shadow_color: vec4<f32>,
    shadow_offset: vec2<f32>,
    shadow_margin: vec2<f32>,
    shadow_radius: f32,
}

struct ShadowVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    // @location(2) @interpolate(flat) shadow_size: vec2<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) border_widths: vec4<f32>,
) -> ShadowVertexOutput {
    var out: ShadowVertexOutput;
    let shadow_pos = rect.shadow_offset + (vertex_uv - vec2(0.5)) * 4.0 * rect.shadow_margin * 8.0;
    let pos = vertex_position + vec3(shadow_pos, 0.0);
    out.position = view.view_proj * vec4<f32>(pos, 1.0);
    out.uv = vertex_uv + shadow_pos / rect.size;
    out.border_widths = border_widths;
    return out;
}

@fragment
fn fragment(in: ShadowVertexOutput) -> @location(0) vec4<f32> {
    let local_pos = (in.uv - 0.5) * rect.size;
    let d = boxSDF(local_pos, rect.size, rect.corner);
    // if 0.0 < d && d < 1.0 {
    //     return vec4(0.0, 0.0, 0.0, 1.0);
    // }
    if d < 1.0 {
        return mix_color(rect.color, d);
    }

    let shadow_size = rect.size + rect.shadow_margin * 2.0;
    let shadow_d = boxSDF(local_pos - rect.shadow_offset, shadow_size, rect.shadow_radius + rect.corner);
    let shadow_alpha = 1.42 * (1.0 - sigmoid(shadow_d / rect.shadow_radius));
    return vec4(rect.shadow_color.rgb, shadow_alpha * rect.shadow_color.a);
}

