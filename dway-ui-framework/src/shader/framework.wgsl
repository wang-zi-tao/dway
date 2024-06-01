#define_import_path dway_ui_framework::shader::framework

const PI:f32 = 3.141592653589793;

fn kawase_blur_image(texture: texture_2d<f32>, sampler_: sampler, pos: vec2<f32>, size: vec2<f32>, radius: f32) -> vec4<f32> {
    var color = vec4(0.0);
    color += textureSample(texture, sampler_, (pos + vec2( 1.0, 1.0) * radius) / size + vec2(0.5));
    color += textureSample(texture, sampler_, (pos + vec2( 1.0,-1.0) * radius) / size + vec2(0.5));
    color += textureSample(texture, sampler_, (pos + vec2(-1.0, 1.0) * radius) / size + vec2(0.5));
    color += textureSample(texture, sampler_, (pos + vec2(-1.0,-1.0) * radius) / size + vec2(0.5));
    return color * 0.25;
}

fn kawase_blur_image2(texture: texture_2d<f32>, sampler_: sampler, pos: vec2<f32>, size: vec2<f32>, radius: f32) -> vec4<f32> {
    var color = vec4(0.0);
    color += kawase_blur_image(texture, sampler_, pos + vec2( 1.0, 1.0) * radius * 3.0, size, radius);
    color += kawase_blur_image(texture, sampler_, pos + vec2( 1.0,-1.0) * radius * 3.0, size, radius);
    color += kawase_blur_image(texture, sampler_, pos + vec2(-1.0, 1.0) * radius * 3.0, size, radius);
    color += kawase_blur_image(texture, sampler_, pos + vec2(-1.0,-1.0) * radius * 3.0, size, radius);
    return color * 0.25;
}

fn gaussian_blur_image5_h(texture: texture_2d<f32>, sampler_: sampler, pos: vec2<f32>, size: vec2<f32>, radius: f32) -> vec4<f32> {
    var color = vec4(0.0);
    color += 1.0 / 16.0 * textureSample(texture, sampler_, (pos + vec2(-2.0, 0.0)) / size + vec2(0.5));
    color += 1.0 / 16.0 * textureSample(texture, sampler_, (pos + vec2( 2.0, 0.0)) / size + vec2(0.5));
    color += 4.0 / 16.0 * textureSample(texture, sampler_, (pos + vec2(-1.0, 0.0)) / size + vec2(0.5));
    color += 4.0 / 16.0* textureSample(texture, sampler_, (pos + vec2( 1.0, 0.0)) / size + vec2(0.5));
    color += 6.0 / 16.0* textureSample(texture, sampler_, pos / size + vec2(0.5));
    return color;
}

fn gaussian_blur_image5(texture: texture_2d<f32>, sampler_: sampler, pos: vec2<f32>, size: vec2<f32>, radius: f32) -> vec4<f32> {
    var color = vec4(0.0);
    color += 1.0 / 16.0 * gaussian_blur_image5_h(texture, sampler_, (pos + vec2(0.0, -2.0)), size, radius);
    color += 1.0 / 16.0 * gaussian_blur_image5_h(texture, sampler_, (pos + vec2(0.0, -2.0)), size, radius);
    color += 4.0 / 16.0 * gaussian_blur_image5_h(texture, sampler_, (pos + vec2(0.0, -1.0)), size, radius);
    color += 4.0 / 16.0* gaussian_blur_image5_h(texture, sampler_, (pos + vec2(0.0, 1.0)), size, radius);
    color += 6.0 / 16.0* gaussian_blur_image5_h(texture, sampler_, pos, size, radius);
    return color;
}

fn rect_sdf(pos: vec2<f32>, size: vec2<f32>) -> f32 {
   let pos2 = abs(pos) - size * 0.5;
   return length(max(pos2, vec2(0.0))) + min(max(pos2.x, pos2.y), 0.0);
}

fn rect_sdf_gradient(pos: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    let w = abs(pos) - size*0.5;
    let s = vec2(select(-1.0,1.0,pos.x<0.0),select(-1.0,1.0,pos.y<0.0));
    let g = max(w.x,w.y);
    let q = max(w, vec2(0.0));
    let l = length(q);
    return s*(select(q/l, (select( vec2(1.0,0.0), vec2(0.0,1.0),w.x<w.y)), g>0.0));
}

fn rounded_rect_sdf(pos: vec2<f32>, size: vec2<f32>, cornerRadius: f32) -> f32 {
   let pos2 = abs(pos) - size * 0.5 + cornerRadius;
   return length(max(pos2, vec2(0.0))) + min(max(pos2.x, pos2.y), 0.0) - cornerRadius;
}

fn rounded_rect_sdf_gradient(pos: vec2<f32>, size: vec2<f32>, cornerRadius: f32) -> vec2<f32> {
    let w = abs(pos) - size*0.5 + cornerRadius;
    let s = vec2(select(-1.0,1.0,pos.x<0.0),select(-1.0,1.0,pos.y<0.0));
    let g = max(w.x,w.y);
    let q: vec2<f32> = max(w, vec2(0.0));
    let l = length(q);
    return s*(select(q/l, (select( vec2(1.0,0.0), vec2(0.0,1.0),w.x<w.y)), g>0.0));
}

fn circle_sdf(pos: vec2<f32>, radius: f32) -> f32 {
   return length(pos) - radius;
}

fn circle_sdf_gradient(pos: vec2<f32>, radius: f32) -> vec2<f32> {
   return pos;
}

fn arc_sdf(pos: vec2<f32>, angles: vec2<f32>, width: f32, radius: f32) -> f32 {
    let angle = 0.5*abs(angles.y - angles.x);
    var pos2 = sdf_rotation(pos, -0.5*(angles.x+angles.y));
    pos2.x = abs(pos2.x);
    let sc = vec2(sin(angle),cos(angle));
    if sc.y*pos2.x > sc.x*pos2.y {
        return length(pos2-sc*radius) - width;
    } else {
        return abs(length(pos2)-radius) - width;
    }
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

fn hsl2rgb(c: vec3<f32>) -> vec3<f32> {
    let rgb = clamp( abs((c.x*6.0+vec3(0.0,4.0,2.0))%6.0-3.0)-1.0, vec3(0.0), vec3(1.0) );
    return c.z + c.y * (rgb-0.5)*(1.0-abs(2.0*c.z-1.0));
}

fn color_wheel(pos: vec2<f32>) -> vec4<f32> {
    var angle = 0.0;
    if abs(pos.y) > abs(pos.x) {
        angle = tanh(-pos.x / pos.y) + 0.5 * PI;
    } else {
        angle = tanh(pos.y / pos.x);
    }
    if pos.x + pos.y < 0.0 {
        angle = angle + PI;
    }
    let rgb = hsl2rgb(vec3( angle / ( 2.0*PI ), 1.0, 0.5 ));
    return vec4(rgb, 1.0);
}
