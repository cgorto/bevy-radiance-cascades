use bevy::color::{color_difference::EuclideanDistance, palettes::css};
use bevy::prelude::*;
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

    if let Ok(window) = window.get_single() {
        let size = Extent3d {
            width: window.resolution.width().round() as u32,
            height: window.resolution.height().round() as u32,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &(css::BEIGE.to_u8_array()),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        let handle = images.add(image);

        commands.spawn(Sprite::from_image(handle.clone()));
        commands.insert_resource(CanvasImage(handle));
    }
}
