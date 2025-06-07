use bevy::{
    app::{App, Startup, Update}, asset::{Assets, Handle, RenderAssetUsages}, color::Color, core_pipeline::core_3d::Camera3d, ecs::system::{Commands, Res, ResMut}, image::Image, math::{primitives::Plane3d, Quat, Vec3}, pbr::{MeshMaterial3d, StandardMaterial}, prelude::{PluginGroup, Resource}, render::{
        mesh::{Mesh, Mesh3d, Meshable},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::ImagePlugin
    }, transform::components::Transform, DefaultPlugins
};
use opencv::{
    core::AlgorithmHint, imgproc, prelude::*, videoio
};
use std::sync::Mutex;

static MJPEG_STREAM_URL: &str = "http://192.168.68.115:8080/video";

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Create an initial empty image
    let size = Extent3d {
        width: 640,
        height: 480,
        depth_or_array_layers: 1,
    };
    let image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD
    );
    let image_handle = images.add(image);

    // Create a plane mesh
    let mesh = meshes.add(Plane3d::default().mesh().size(5.0, 5.0));

    // Create a material with the video texture
    let material = materials.add(StandardMaterial {
        // base_color: Color::srgb(1.0, 0.0, 0.0), // Placeholder color
        base_color_texture: Some(image_handle.clone()),
        unlit: true,
        ..Default::default()
    });
    
    commands.insert_resource(VideoTextureHandle(image_handle));

    // Spawn the background entity
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)).with_rotation(
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), // Rotate the plane to face the camera
        ), // Position the plane behind the camera
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Resource)]
struct VideoCapture(Mutex<videoio::VideoCapture>);

#[derive(Resource, Default)]
struct WebcamFrame(Mat);

#[derive(Resource)]
struct VideoTextureHandle(Handle<Image>);

fn update_background_texture(
    mut images: ResMut<Assets<Image>>,
    cam: Res<VideoCapture>,
    video_texture_handle: Res<VideoTextureHandle>,
    mut webcam_frame: ResMut<WebcamFrame>,
) {
    // Retrieve the latest frame from the webcam
    let frame = &mut webcam_frame.0;

    cam.0.lock().expect("Failed to lock video capture mutex").read(frame).expect("Failed to read frame from video capture");
    if frame.empty() {
        return;
    }

    let mut rgba = Mat::default();
    if imgproc::cvt_color(frame, &mut rgba, imgproc::COLOR_BGR2RGBA, 0, AlgorithmHint::ALGO_HINT_DEFAULT).is_err() {
        return;
    }

    // Get image dimensions
    let (width, height) = (rgba.cols() as u32, rgba.rows() as u32);

    // Get the image data
    let data = match rgba.data_bytes() {
        Ok(data) => data.to_vec(),
        Err(_) => return,
    };

    // Update the existing image asset
    if let Some(image) = images.get_mut(&video_texture_handle.0) {
        image.resize(Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });
        image.data = Some(data);
    }
}

fn main() -> opencv::Result<()> {
    let cam = videoio::VideoCapture::from_file(MJPEG_STREAM_URL, videoio::CAP_ANY)?; 
    // Temporary: Use a local video file for testing instead
    // let cam = videoio::VideoCapture::from_file("assets/video.mp4", videoio::CAP_ANY)?;
    let opened = videoio::VideoCapture::is_opened(&cam)?;
    if !opened {
        panic!("Unable to open camera stream");
    }

    App::new()
        .insert_resource(VideoCapture(Mutex::new(cam)))
        .insert_resource(WebcamFrame(Mat::default()))
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, update_background_texture)
        .run();

    Ok(())
}