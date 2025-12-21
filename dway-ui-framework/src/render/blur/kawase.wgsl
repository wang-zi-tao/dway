#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import dway_ui_framework::shader::framework::visualition_2d_uv 

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

struct Vertex {
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>( vec2<f32>(vertex.uv.x, 1.0 - vertex.uv.y) * 2.0 - vec2<f32>(1.0, 1.0), 1.0, 1.0 );
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let f = ( 0.5 + f32(settings.layer) ) * settings.radius / settings.size;

    var color = vec4(0.0);
    color += textureSample(in_texture, in_sampler, uv+vec2(1.0,1.0)*f);
    color += textureSample(in_texture, in_sampler, uv+vec2(1.0,-1.0)*f);
    color += textureSample(in_texture, in_sampler, uv+vec2(-1.0,1.0)*f);
    color += textureSample(in_texture, in_sampler, uv+vec2(-1.0,-1.0)*f);
    return vec4(1.0, 0.0, 0.0, 1.0);
}

