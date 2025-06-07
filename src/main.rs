use opencv::{
    prelude::*,
    videoio,
    highgui,
};

static MJPEG_STREAM_URL: &str = "http://192.168.68.115:8080/video";

fn main() -> opencv::Result<()> {
    let mut cam = videoio::VideoCapture::from_file(MJPEG_STREAM_URL, videoio::CAP_ANY)?; 
    let opened = videoio::VideoCapture::is_opened(&cam)?;
    if !opened {
        panic!("Unable to open camera stream");
    }

    let window = "Stream";
    highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;

    let mut frame = Mat::default();
    loop {
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }
        highgui::imshow(window, &frame)?;
        if highgui::wait_key(10)? > 0 {
            break;
        }
    }

    Ok(())
}