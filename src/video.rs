use std::sync::Mutex;

use bevy::{app::{App, Plugin, Update}, ecs::{resource::Resource, schedule::IntoScheduleConfigs, system::{Res, ResMut}}};
use opencv::{core::{Mat, MatTraitConst}, videoio::{self, VideoCaptureTrait, VideoCaptureTraitConst}};

use crate::VideoCaptureSystems;

pub mod aruco_camera;

static MJPEG_STREAM_URL: &str = "http://192.168.68.116:8080/video";

#[derive(Resource)]
pub struct VideoCapture(pub Mutex<videoio::VideoCapture>);

#[derive(Resource, Default)]
pub struct WebcamFrame(pub Mat);

pub struct VideoCapturePlugin;

fn capture_background_image(
    mut webcam_frame: ResMut<WebcamFrame>,
    cam: Res<VideoCapture>
) {
    let frame = &mut webcam_frame.0;

    cam.0.lock().expect("Failed to lock video capture mutex").read(frame).expect("Failed to read frame from video capture");
    if frame.empty() {
        eprintln!("No frame captured from webcam");
        return;
    }
}

impl Plugin for VideoCapturePlugin {
    fn build(&self, app: &mut App) {
        let cam = videoio::VideoCapture::from_file(MJPEG_STREAM_URL, videoio::CAP_ANY)
            .expect("Failed to create video capture from MJPEG stream");
        // Temporary: Use the local camera for testing instead
        // let cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)
        //     .expect("Failed to create video capture from camera");
        let opened = videoio::VideoCapture::is_opened(&cam)
            .expect("Failed to check if video capture is opened");
        if !opened {
            panic!("Unable to open camera stream");
        }
        
        app
            .insert_resource(VideoCapture(Mutex::new(cam)))
            .insert_resource(WebcamFrame(Mat::default()))
            .add_systems(Update, capture_background_image.in_set(VideoCaptureSystems));
    }
}