use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::{FULLSCREEN_SHADER_HANDLE, fullscreen_shader_vertex_state},
        post_process::{self, PostProcessingPipeline},
    },
    ecs::query::QueryItem,
    log::LogPlugin,
    prelude::*,
    render::{
        Extract, Render, RenderApp,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{
            Node, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode,
            ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::ViewTarget,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/gi_material.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CascadePlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, update_settings)
        .add_systems(Update, ping_pong_canvas)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, window: Query<&Window>) {
    commands.spawn((Camera2d, PostProcessSettings::default()));
    println!("we reach here!");
    if let Ok(window) = window.get_single() {
        let mut image = Image::new_fill(
            Extent3d {
                width: window.width().round() as u32,
                height: window.height().round() as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 0],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT;

        let a = images.add(image.clone());
        let b = images.add(image);

        commands.spawn(Sprite {
            image: a.clone(),
            custom_size: Some(window.size()),
            ..Default::default()
        });

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

fn update_settings(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window>,
    mut settings: Query<&mut PostProcessSettings>,
) {
    if let Ok(window) = window.get_single() {
        for mut setting in &mut settings {
            if let Some(cursor_pos) = window.cursor_position() {
                setting.resolution = window.resolution.size();
                if mouse.just_pressed(MouseButton::Left) {
                    setting.drawing = 1;
                    setting.from = cursor_pos;
                    setting.to = cursor_pos;
                } else if mouse.pressed(MouseButton::Left) {
                    setting.drawing = 1;
                    setting.from = setting.to;
                    setting.to = cursor_pos;
                } else {
                    setting.drawing = 0;
                }
            }
            setting.radius_squared = 100.0;
            setting.color = Vec3::new(0.0, 0.0, 1.0);
        }
    }
}

fn ping_pong_canvas(mut canvas_images: ResMut<CanvasImages>, mut sprite: Single<&mut Sprite>) {
    let image = if canvas_images.target_front {
        &canvas_images.front
    } else {
        &canvas_images.back
    };
    sprite.image = image.clone();
    canvas_images.target_front = !canvas_images.target_front;
}

struct CascadePlugin;

impl Plugin for CascadePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<PostProcessSettings>::default(),
            UniformComponentPlugin::<PostProcessSettings>::default(),
            ExtractResourcePlugin::<CanvasImages>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<PostProcessNode>(Core2d, PostProcessLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::PostProcessing,
                    PostProcessLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<PostProcessPipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct PostProcessLabel;

#[derive(Default)]
struct PostProcessNode;

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, AsBindGroup)]
struct PostProcessSettings {
    resolution: Vec2,
    radius_squared: f32,
    drawing: u32,
    from: Vec2,
    to: Vec2,
    color: Vec3,
}

impl Node for PostProcessNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let post_process_pipeline = world.resource::<PostProcessPipeline>();
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

#[derive(Resource)]
struct PostProcessPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for PostProcessPipeline {
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
        let shader = world.load_asset(SHADER_ASSET_PATH);

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
