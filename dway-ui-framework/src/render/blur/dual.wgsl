#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct KawaseSettings {
    radius: f32,
    layer: u32,
    stage: u32,
    size: vec2<f32>,
}
@group(0) @binding(2) var<uniform> settings: KawaseSettings;

struct VertexOutput {
    [[builtin(position)]]
    position: vec4<f32>;
    [[location(0)]]
    uv: vec2<f32>;
    p0: vec2<f32>,
    p1: vec2<f32>,
    p2: vec2<f32>,
    p3: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    let p0 = uv + vec2(1.0,1.0) * settings.radius / position.size;
    let p1 = uv + vec2(1.0,-1.0) * settings.radius / position.size;
    let p2 = uv + vec2(-1.0,1.0) * settings.radius / position.size;
    let p3 = uv + vec2(-1.0,-1.0) * settings.radius / position.size;

    return VertexOutput(clip_position, uv, p0, p1, p2, p3);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let offset_strength = settings.intensity;

    var color = 0.5 * textureSample(screen_texture, texture_sampler, in.uv);
    color += 0.125 * textureSample(screen_texture, texture_sampler, in.p0);
    color += 0.125 * textureSample(screen_texture, texture_sampler, in.p1);
    color += 0.125 * textureSample(screen_texture, texture_sampler, in.p2);
    color += 0.125 * textureSample(screen_texture, texture_sampler, in.p3);
    return color;
}


