#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
@group(0) @binding(0) var screen_texture: texture_2d<f32>;

@group(0) @binding(1) var texture_sampler: sampler;

struct RaymarchSettings {
    resolution: vec2<f32>,
    ray_count: u32,
    max_steps: u32,
}

@group(0) @binding(2) var<uniform> settings: RaymarchSettings;

const PI: f32 = 3.14159265;
const TAU: f32 = 2.0 * PI;

fn rand(in: vec2<f32>) -> f32 {
    let magic_vec = vec2<f32>(12.9898f, 78.233f);
    return fract(sin(dot(in, magic_vec) * 43758.5453));
}


fn raymarch(uv: vec2<f32>) -> @location(0) vec4<f32> {
    var current = textureSample(screen_texture, texture_sampler, uv);
    if (current[3] > 0.1) {
        return current;
    }
}
