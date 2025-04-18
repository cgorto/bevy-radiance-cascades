use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    reflect::TypePath,
    render::render_resource::*,
    sprite::{Material2d, Material2dPlugin},
};

const SHADER_ASSET_PATH: &str = "shaders/gi_material.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Material2dPlugin::<GIMaterial>::default()))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct GIMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

impl Material2d for GIMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GIMaterial>>,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window>,
) {
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
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING;

        let handle: Handle<Image> = images.add(image);
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::from_size(window.resolution.size()))),
            MeshMaterial2d(materials.add(GIMaterial {
                color: LinearRgba::BLUE,
                color_texture: Some(handle),
            })),
        ));
    }
}
