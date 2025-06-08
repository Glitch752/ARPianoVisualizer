use bevy::{
    app::{App, Startup}, color::Color, core_pipeline::core_3d::Camera3d, ecs::system::Commands, math::Vec3, prelude::PluginGroup, render::{
        camera::ClearColor, texture::ImagePlugin
    }, transform::components::Transform, DefaultPlugins
};
use opencv::videoio::{self, VideoCaptureTraitConst};
use std::sync::Mutex;

use crate::video::VideoCapture;

mod video;
mod background;

static MJPEG_STREAM_URL: &str = "http://192.168.68.115:8080/video";

fn setup(
    mut commands: Commands
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn main() -> opencv::Result<()> {
    // let cam = videoio::VideoCapture::from_file(MJPEG_STREAM_URL, videoio::CAP_ANY)?; 
    // Temporary: Use the local camera for testing instead
    let cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?;
    let opened = videoio::VideoCapture::is_opened(&cam)?;
    if !opened {
        panic!("Unable to open camera stream");
    }

    App::new()
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(VideoCapture(Mutex::new(cam)))
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(background::CameraBackground)
        .add_systems(Startup, setup)
        .run();

    Ok(())
}