use bevy::{
    pbr::prepare_prepass_view_bind_group,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        graph,
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::texture_storage_2d, *},
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
    },
};

use std::borrow::Cow;

const SHADER_ASSET_PATH: &str = "shaders/gi_material.wgsl";

const SIZE: (u32, u32) = (1280, 720);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CascadePlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, switch_textures)
        .run();
}

#[derive(Resource, Clone, ExtractResource)]
struct LightingImages {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, window: Query<&Window>) {
    commands.spawn(Camera2d);
    println!("we reach here!");
    if let Ok(window) = window.get_single() {
        let mut image = Image::new_fill(
            Extent3d {
                width: window.width().round() as u32,
                height: window.height().round() as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;

        let image0 = images.add(image.clone());
        let image1 = images.add(image);

        commands.spawn(Sprite {
            image: image0.clone(),
            custom_size: Some(window.size()),
            ..Default::default()
        });

        commands.insert_resource(LightingImages {
            texture_a: image0,
            texture_b: image1,
        });
    }
}

fn switch_textures(images: Res<LightingImages>, mut sprite: Single<&mut Sprite>) {
    if sprite.image == images.texture_a {
        sprite.image = images.texture_b.clone_weak();
    } else {
        sprite.image = images.texture_a.clone_weak();
    }
}

struct CascadePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CascadeLabel;

impl Plugin for CascadePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<LightingImages>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(CascadeLabel, CascadeNode::default());
        render_graph.add_node_edge(CascadeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<CascadePipeline>();
    }
}

#[derive(Resource)]
struct LightingImageBindGroups([BindGroup; 2]);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<CascadePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    lighting_images: Res<LightingImages>,
    render_device: Res<RenderDevice>,
) {
    let view_a = gpu_images.get(&lighting_images.texture_a).unwrap();
    let view_b = gpu_images.get(&lighting_images.texture_b).unwrap();
    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_a.texture_view, &view_b.texture_view)),
    );

    let bind_group_1 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_a.texture_view, &view_b.texture_view)),
    );

    commands.insert_resource(LightingImageBindGroups([bind_group_0, bind_group_1]));
}

#[derive(Resource)]
struct CascadePipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for CascadePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "LightingImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::ReadOnly),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
            zero_initialize_workgroup_memory: false,
        });

        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
            zero_initialize_workgroup_memory: false,
        });

        CascadePipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum CascadeState {
    Loading,
    Init,
    Update(usize),
}

struct CascadeNode {
    state: CascadeState,
}

impl Default for CascadeNode {
    fn default() -> Self {
        Self {
            state: CascadeState::Loading,
        }
    }
}

impl render_graph::Node for CascadeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<CascadePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match self.state {
            CascadeState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline) {
                    CachedPipelineState::Ok(_) => {
                        self.state = CascadeState::Init;
                    }
                    CachedPipelineState::Err(_) => {
                        panic!("the pipeline is fucked")
                    }
                    _ => {}
                }
            }
            CascadeState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = CascadeState::Update(1);
                }
            }
            CascadeState::Update(0) => {
                self.state = CascadeState::Update(1);
            }
            CascadeState::Update(1) => {
                self.state = CascadeState::Update(0);
            }
            CascadeState::Update(_) => unreachable!(),
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = &world.resource::<LightingImageBindGroups>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<CascadePipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        match self.state {
            CascadeState::Loading => {}
            CascadeState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            CascadeState::Update(index) => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[index], &[]);
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }
        Ok(())
    }
}
