
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
@group(0) @binding(0) var screen_texture: texture_2d<f32>;

@group(0) @binding(1) var texture_sampler: sampler;

struct PostProcessSettings {
    resolution: vec2<f32>,
    radius_squared: f32,
    drawing: u32,
    fro: vec2<f32>,
    to: vec2<f32>,
    color: vec3<f32>,
}

@group(0) @binding(2) var<uniform> settings: PostProcessSettings;


fn sdf_line_squared(p: vec2<f32>, fro: vec2<f32>, to: vec2<f32>) -> f32 {
    let start = p - fro;
    let line = to - fro;
    let len_squared = dot(line, line);
    let t = clamp(dot(start, line) / len_squared, 0.0, 1.0);
    let diff = start - line * t;
    return dot(diff,diff);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var current = textureSample(screen_texture, texture_sampler, in.uv);
    if (settings.drawing != 0u) {
        let coord = in.uv * settings.resolution;
        if (sdf_line_squared(coord, settings.fro, settings.to) <= settings.radius_squared){
            current = vec4<f32>(settings.color.rgb, 1.0);
        }
    }
    //return vec4<f32>(0.0,0.0,1.0,1.0);
    return current;
}
