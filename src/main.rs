use bevy::color::{color_difference::EuclideanDistance, palettes::css};
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

#[derive(Resource)]
struct CanvasImage(Handle<Image>);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, window: Query<&Window>) {
    commands.spawn(Camera2d);
    println!("we reach here!");
    if let Ok(window) = window.get_single() {
        println!("we've got the window");
        let size = Extent3d {
            width: window.resolution.width().round() as u32,
            height: window.resolution.height().round() as u32,
            depth_or_array_layers: 1,
        };
        println!("we've got the size");
        let mut image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        println!("created image");
        let handle = images.add(image);

        commands.spawn(Sprite::from_image(handle.clone()));
        commands.insert_resource(CanvasImage(handle));
        println!("sys done");
    }
}
