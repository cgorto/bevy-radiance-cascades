use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        RenderApp,
        extract_component::{
            ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
        },
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
    },
};

use bevy_egui::{EguiContextPass, EguiContextSettings, EguiContexts, EguiPlugin, egui};

const CANVAS_SHADER_ASSET_PATH: &str = "shaders/canvas.wgsl";
const RAYMARCH_SHADER_ASSET_PATH: &str = "shaders/raymarching.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CascadePlugin))
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update_settings)
        .add_systems(Update, ping_pong_canvas)
        .add_systems(EguiContextPass, side_panel_stroke_control)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, window: Query<&Window>) {
    commands.spawn((
        Camera2d,
        PostProcessSettings::default(),
        //initializing the raymarch uniforms with values here for testing
        RaymarchSettings {
            resolution: Vec2::default(),
            ray_count: 16,
            max_steps: 128,
        },
    ));
    println!("we reach here!");
    if let Ok(window) = window.single() {
        //Initialize an empty image for input to our shaders, we are just making it the same size as the screen
        let mut image = Image::new_fill(
            Extent3d {
                width: window.width().round() as u32,
                height: window.height().round() as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        //I think having all of these flags is overkill, we should just need RenderAttachment and TextureBinding
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT;

        let a = images.add(image.clone());
        let a_raymarch = images.add(image.clone());
        let b_raymarch = images.add(image.clone());
        let b = images.add(image);

        commands.spawn(Sprite {
            image: a.clone(),
            custom_size: Some(window.size()),
            ..Default::default()
        });
        //Initializing our two ping pong resources for rendering.
        //We need two since we want the lighting not to feed back in to what we've drawn.
        commands.insert_resource(RaymarchImages {
            a: a_raymarch,
            b: b_raymarch,
            ping: false,
        });

        //Here we're initializing our ping pong rendering
        commands.insert_resource(CanvasImages {
            front: a,
            back: b,
            target_front: false,
        });
    }
}

#[derive(Resource, Clone, ExtractResource)]
struct CanvasImages {
    front: Handle<Image>,
    back: Handle<Image>,
    target_front: bool,
}

#[derive(Resource, Clone, ExtractResource)]
struct RaymarchImages {
    a: Handle<Image>,
    b: Handle<Image>,
    ping: bool,
}

fn update_settings(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window>,
    mut settings: Query<(&mut PostProcessSettings, &mut RaymarchSettings)>,
) {
    //This system is run every frame. Eventually I'll add a GUI that allows you to toggle shader settings. This is fine for now.
    if let Ok(window) = window.single() {
        for (mut canvas_setting, mut raymarch_setting) in &mut settings {
            if let Some(cursor_pos) = window.cursor_position() {
                canvas_setting.resolution = window.resolution.size();
                raymarch_setting.resolution = window.resolution.size();
                //First frame of drawing
                if mouse.just_pressed(MouseButton::Left) {
                    canvas_setting.drawing = 1;
                    canvas_setting.from = cursor_pos;
                    canvas_setting.to = cursor_pos;
                } else if mouse.pressed(MouseButton::Left) {
                    canvas_setting.drawing = 1;
                    canvas_setting.from = canvas_setting.to;
                    canvas_setting.to = cursor_pos;
                } else {
                    canvas_setting.drawing = 0;
                }
            }
            //Brush size and color for the shader uniforms
            canvas_setting.radius_squared = 100.0;
            canvas_setting.color = Vec3::new(0.0, 0.0, 1.0);
            //Raymarching uniforms here
            raymarch_setting.max_steps = 256;
            raymarch_setting.ray_count = 16;
        }
    }
}
fn side_panel_stroke_control(
    mut contexts: EguiContexts,
    mut settings: Query<(&mut PostProcessSettings, &mut RaymarchSettings)>,
) {
    if let Ok((mut canvas_settings, mut raymarch_settings)) = settings.single_mut() {
        let ctx = contexts.ctx_mut();
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.label("Stroke Color:");
            ui.color_edit_button_rgb(canvas_settings.color.as_mut());

            ui.separator();

            ui.label("Stroke Radius:");

            let mut radius = canvas_settings.radius_squared.sqrt();
            let slider_response = ui.add(egui::Slider::new(&mut radius, 1.0..=50.0).text("Radius"));

            if slider_response.changed() {
                canvas_settings.radius_squared = radius * radius;
            }
        });
    }
}

fn ping_pong_canvas(
    mut canvas_images: ResMut<CanvasImages>,
    mut raymarch_images: ResMut<RaymarchImages>,
    mut sprite: Single<&mut Sprite>,
) {
    let image = if raymarch_images.ping {
        &raymarch_images.a
    } else {
        &raymarch_images.b
    };
    sprite.image = image.clone();
    canvas_images.target_front = !canvas_images.target_front;
    raymarch_images.ping = !raymarch_images.ping;
}

struct CascadePlugin;

impl Plugin for CascadePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<PostProcessSettings>::default(),
            UniformComponentPlugin::<PostProcessSettings>::default(),
            ExtractComponentPlugin::<RaymarchSettings>::default(),
            UniformComponentPlugin::<RaymarchSettings>::default(),
            ExtractResourcePlugin::<CanvasImages>::default(),
            ExtractResourcePlugin::<RaymarchImages>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        //We add our custom render graph nodes here.
        // Most of this is boilerplate from the custom post process effect example, so we *probably* have more stuff than we need
        // I don't think we need most of Core2d, but since we're rendering a sprite I'm keeping it all just in case.
        render_app
            .add_render_graph_node::<CanvasNode>(Core2d, CanvasPassLabel)
            .add_render_graph_node::<RaymarchNode>(Core2d, RaymarchLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::PostProcessing,
                    CanvasPassLabel,
                    RaymarchLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<CanvasPipeline>();
        render_app.init_resource::<RaymarchPipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CanvasPassLabel;

#[derive(Default)]
struct CanvasNode;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RaymarchLabel;

#[derive(Default)]
struct RaymarchNode;

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, AsBindGroup)]
struct PostProcessSettings {
    resolution: Vec2,
    radius_squared: f32,
    drawing: u32,
    from: Vec2,
    to: Vec2,
    color: Vec3,
}

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, AsBindGroup)]
struct RaymarchSettings {
    resolution: Vec2,
    ray_count: u32,
    max_steps: u32,
}

impl Node for CanvasNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let post_process_pipeline = world.resource::<CanvasPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<PostProcessSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };
        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let canvas_images = world.resource::<CanvasImages>();

        let front_gpu = gpu_images.get(&canvas_images.front).unwrap();
        let back_gpu = gpu_images.get(&canvas_images.back).unwrap();

        let (src_view, dst_view) = if canvas_images.target_front {
            (&back_gpu.texture_view, &front_gpu.texture_view)
        } else {
            (&front_gpu.texture_view, &back_gpu.texture_view)
        };

        let bind_group = render_context.render_device().create_bind_group(
            "post_process_bind_group",
            &post_process_pipeline.layout,
            &BindGroupEntries::sequential((
                src_view,
                &post_process_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("post_process_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: dst_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

impl Node for RaymarchNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let raymarch_pipeline = world.resource::<RaymarchPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(raymarch_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<RaymarchSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };
        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let canvas_images = world.resource::<CanvasImages>();
        let raymarch_images = world.resource::<RaymarchImages>();
        // The source view should be the same texture that we just wrote to in the canvas pass
        let src_view = if canvas_images.target_front {
            &gpu_images.get(&canvas_images.front).unwrap().texture_view
        } else {
            &gpu_images.get(&canvas_images.back).unwrap().texture_view
        };
        // Here is where we begin to incorporate the second ping pong. Any subsequent passes should use this
        // We need two ping pongs to keep what we've drawn separate from the output of the GI passes
        // The GI passes need the undrawn area to have a low alpha, but drawing the lighting will necessarily give the pixels a higher alpha.
        // There's most certainly a better what to approach this but this is what I've done.
        let dst_view = if raymarch_images.ping {
            &gpu_images.get(&raymarch_images.a).unwrap().texture_view
        } else {
            &gpu_images.get(&raymarch_images.b).unwrap().texture_view
        };

        let bind_group = render_context.render_device().create_bind_group(
            "raymarch_bind_group",
            &raymarch_pipeline.layout,
            &BindGroupEntries::sequential((
                src_view,
                &raymarch_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("raymarch_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: dst_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct CanvasPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for CanvasPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "post_process_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::Filtering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<PostProcessSettings>(false),
                ),
            ),
        );

        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        // Get the shader handle
        let shader = world.load_asset(CANVAS_SHADER_ASSET_PATH);

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("post_process_pipeline".into()),
                layout: vec![layout.clone()],
                // This will setup a fullscreen triangle for the vertex state
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader,
                    shader_defs: vec![],
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                // All of the following properties are not important for this effect so just use the default values.
                // This struct doesn't have the Default trait implemented because not all fields can have a default value.
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
                zero_initialize_workgroup_memory: false,
            });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

#[derive(Resource)]
struct RaymarchPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for RaymarchPipeline {
    fn from_world(world: &mut World) -> Self {
        println!("surely happing here?");
        let render_device = world.resource::<RenderDevice>();

        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "raymarch_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::Filtering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<RaymarchSettings>(false),
                ),
            ),
        );

        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        // Get the shader handle
        let shader = world.load_asset(RAYMARCH_SHADER_ASSET_PATH);

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("raymarch_pipeline".into()),
                layout: vec![layout.clone()],
                // This will setup a fullscreen triangle for the vertex state
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader,
                    shader_defs: vec![],
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                // All of the following properties are not important for this effect so just use the default values.
                // This struct doesn't have the Default trait implemented because not all fields can have a default value.
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
                zero_initialize_workgroup_memory: false,
            });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}
