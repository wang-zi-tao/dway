#define_import_path dway_ui::shapes

fn boxSDF(uv: vec2<f32>, size: vec2<f32>, cornerRadius: f32) -> f32 {
   let pos = size * (uv - vec2( 0.5 ));
   let pos2 = abs(pos) - size * 0.5 + cornerRadius;
   return length(max(pos2, vec2<f32>(0.0))) + min(max(pos2.x, pos2.y), 0.0) - cornerRadius;
}

fn circleSDF(uv: vec2<f32>, radius: f32) -> f32 {
   let pos = radius * (uv - vec2(0.5));
   return length(pos) - radius * 0.5;
}

const COLOR_NONE: vec4<f32> = vec4<f32>(0.0);
