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

fn out_of_bounds(uv: vec2<f32>) -> bool {
    return uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0;
}


fn raymarch(uv: vec2<f32>) -> vec4<f32> {
    var current = textureSample(screen_texture, texture_sampler, uv);
    if (current.a > 0.1) {
        return current;
    }

    let reciprocal_raycount = 1.0 / f32(settings.ray_count);
    let tau_raycount = TAU * reciprocal_raycount;

    let noise = rand(uv);
    var radiance = vec4<f32>(0.0);

    for (var i = 0u; i < settings.ray_count; i += 1u) {
        let angle = tau_raycount * (f32(i) + noise);
        let ray_direction = vec2<f32>(cos(angle), -sin(angle)) / settings.resolution;
        
        for (var step = 0u; step < settings.max_steps; step += 1u) {
            let sample_uv = uv + ray_direction * f32(step);

            if (out_of_bounds(sample_uv)) {
                break;
            }

            let sample_light = textureSample(screen_texture,texture_sampler, sample_uv);

            if (sample_light.a > 0.5) {
                radiance += sample_light;
                break;
            }
        }
    }

    return radiance * reciprocal_raycount;
}


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let final_color = raymarch(in.uv);
    // return final_color;
    return vec4<f32>(final_color.xyz, 1.0);
}
