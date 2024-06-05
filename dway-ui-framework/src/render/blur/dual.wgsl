#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var in_sampler: sampler;

struct KawaseSettings {
    radius: f32,
    layer: u32,
    stage: u32,
    size: vec2<f32>,
}
@group(0) @binding(2) var<uniform> settings: KawaseSettings;

struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return VertexOutput(clip_position, uv);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.position.xy / settings.size;

    if ( settings.stage == 0 ) {
        let uv = in.position.xy / settings.size;
        let f = ( 0.5 + settings.radius ) / settings.size;

        var color = 4.0 * textureSample(in_texture, in_sampler, uv);
        color += textureSample(in_texture, in_sampler, uv+vec2(1.0,1.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(1.0,-1.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(-1.0,1.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(-1.0,-1.0)*f);
        return color / 8.0;
    } else {
        let f = ( 0.5 + settings.radius ) / settings.size;

        var color = vec4(0.0);
        color += 2.0 * textureSample(in_texture, in_sampler, uv+vec2(1.0,1.0)*f);
        color += 2.0 * textureSample(in_texture, in_sampler, uv+vec2(1.0,-1.0)*f);
        color += 2.0 * textureSample(in_texture, in_sampler, uv+vec2(-1.0,1.0)*f);
        color += 2.0 * textureSample(in_texture, in_sampler, uv+vec2(-1.0,-1.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(0.0,2.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(0.0,-2.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(2.0,0.0)*f);
        color += textureSample(in_texture, in_sampler, uv+vec2(-2.0,0.0)*f);
        return color / 12.0;
    }
}


