use std::sync::Mutex;

use bevy::ecs::resource::Resource;
use opencv::{core::Mat, videoio};


#[derive(Resource)]
pub struct VideoCapture(pub Mutex<videoio::VideoCapture>);

#[derive(Resource, Default)]
pub struct WebcamFrame(pub Mat);

#[derive(Resource, Default)]
pub struct ConvertedWebcamFrame(pub Mat);
