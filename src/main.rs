use bevy::{
    app::{App, Startup, Update}, asset::Assets, color::{palettes::css::SILVER, Color}, ecs::{schedule::{IntoScheduleConfigs, SystemSet}, system::{Commands, ResMut}}, math::primitives::Plane3d, pbr::{MeshMaterial3d, StandardMaterial}, prelude::PluginGroup, render::{mesh::{Mesh, Mesh3d, Meshable}, texture::ImagePlugin}, transform::components::Transform, DefaultPlugins
};

/// Systems that capture video frames from the camera.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct VideoCaptureSystems;

/// Systems that will always run after the video frame is captured.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct VideoUpdateSystems;

/// The systems that draw the video frames to the screen.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct VideoDrawSystems;

mod video;
mod background;
pub mod testing;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
    
) {
    // Insert a small ground plane
    // commands.spawn((
    //     Mesh3d(meshes.add(Plane3d::default().mesh().size(500.0, 500.0))),
    //     MeshMaterial3d(materials.add(Color::from(SILVER))),
    //     Transform::from_xyz(0.0, -500.0, 0.0)
    // ));
}

fn main() -> opencv::Result<()> {

    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins((background::CameraBackground, video::VideoCapturePlugin, video::aruco_camera::ArUcoCameraPlugin, testing::TestingPlugin))
        .add_systems(Startup, setup)
        .configure_sets(Update, (
            VideoCaptureSystems,
            VideoUpdateSystems.after(VideoCaptureSystems),
            VideoDrawSystems.after(VideoUpdateSystems)
        ))
        .run();

    Ok(())
}