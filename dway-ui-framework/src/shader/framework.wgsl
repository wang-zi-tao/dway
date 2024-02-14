#define_import_path dway_ui_framework::shader::framework

fn rectSDF(pos: vec2<f32>, size: vec2<f32>) -> f32 {
   let pos2 = abs(pos) - size * 0.5;
   return length(max(pos2, vec2(0.0))) + min(max(pos2.x, pos2.y), 0.0);
}

fn boxSDF(pos: vec2<f32>, size: vec2<f32>, cornerRadius: f32) -> f32 {
   let pos2 = abs(pos) - size * 0.5 + cornerRadius;
   return length(max(pos2, vec2(0.0))) + min(max(pos2.x, pos2.y), 0.0) - cornerRadius;
}

fn circleSDF(pos: vec2<f32>, radius: f32) -> f32 {
   return length(pos) - radius;
}

fn mix_color(color: vec4<f32>, value: f32) -> vec4<f32> {
    let alpha = max(min(1.0 - value, 1.0), 0.0);
    // let alpha = smoothstep(-1.0, 1.0, value);
    return vec4(color.rgb, alpha * color.a);
}

fn mix_inner_color(color: vec4<f32>, value: f32) -> vec4<f32> {
    let alpha = max(min(- value, 1.0), 0.0);
    return vec4(color.xyz, alpha * color.w);
}

fn sigmoid(t: f32) -> f32 {
    return 1.0 / (1.0 + exp(-t));
}

fn sdf_visualition(v: f32) -> vec4<f32> {
    let s = sin(v*3.14*2.0 ) * 0.5 + 0.5;
    return vec4(s,sigmoid(v),0.0,1.0);
}

fn mix_alpha(bg: vec4<f32>, fg: vec4<f32>) -> vec4<f32> {
    return vec4(bg.rgb * (1.0-fg.a) + fg.rgb*fg.a,bg.a + fg.a - bg.a*fg.a);
}

fn sdf_rotation(pos: vec2<f32>, rotation: f32) -> vec2<f32> {
    let s = sin(-rotation);
    let c = cos(-rotation);
    return vec2(c*pos.x-s*pos.y, s*pos.x + c*pos.y);
}
