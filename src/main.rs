use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    reflect::TypePath,
    render::{extract_resource::ExtractResource, render_resource::*},
    sprite::{Material2d, Material2dPlugin},
};

const SHADER_ASSET_PATH: &str = "shaders/gi_material.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins))
        .add_systems(Startup, setup)
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
