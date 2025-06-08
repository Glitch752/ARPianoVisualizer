use std::{fs, sync::Mutex};

use bevy::{app::{App, Plugin, Startup, Update}, core_pipeline::core_3d::Camera3d, ecs::{resource::Resource, schedule::IntoScheduleConfigs, system::{Commands, Query, Res, ResMut}}, math::{Mat3, Quat, Vec3}, transform::components::Transform};
use opencv::{boxed_ref::BoxedRef, calib3d, core::{AlgorithmHint, DataType, Mat, MatTraitConst, Point2f, Point3d, Scalar, Vector}, objdetect::{self, ArucoDetector, RefineParameters}, prelude::ArucoDetectorTraitConst};
use serde::Deserialize;
use crate::{video::WebcamFrame, VideoUpdateSystems};

pub struct ArUcoCameraPlugin;


#[derive(Resource)]
pub struct CameraIntrinsics {
    pub camera_matrix: Mat,
    pub dist_coeffs: Mat
}

#[derive(Resource)]
pub struct ArucoTrackingData {
    greyscale_image: Mat,

    ids: Vector<i32>,
    corners: Vector<Vector<Point2f>>,
    rejected_img_points: Vector<Vector<Point2f>>,

    latest_rotation: Mat,
    latest_translation: Mat
}

impl Default for ArucoTrackingData {
    fn default() -> Self {
        Self {
            greyscale_image: Mat::default(),
            ids: Vector::new(),
            corners: Vector::new(),
            rejected_img_points: Vector::new(),
            latest_rotation: Mat::from_slice(&[0.0, 0.0, 0.0]).expect("Failed to create default rotation vector").try_clone().expect("Failed to clone default rotation vector"),
            latest_translation: Mat::from_slice(&[0.0, 0.0, 0.0]).expect("Failed to create default translation vector").try_clone().expect("Failed to clone default translation vector")
        }
    }
}

#[derive(Resource)]
pub struct FiducialDetector(Mutex<ArucoDetector>);

struct FiducialPosition {
    id: i32,
    /** The offset from the center of the keyboard to the center of the fiducial in mm. Rightward is positive. */
    x_offset: f64
}

impl FiducialPosition {
    fn get_corners(&self) -> [Point3d; 4] {
        let half_size = FIDUCIAL_SIZE / 2.0;
        [
            // Top-left, top-right, bottom-right, bottom-left based on order of corners returned by OpenCV
            Point3d::new(self.x_offset - half_size, 0.0, -half_size),
            Point3d::new(self.x_offset + half_size, 0.0, -half_size),
            Point3d::new(self.x_offset + half_size, 0.0, half_size),
            Point3d::new(self.x_offset - half_size, 0.0, half_size)
        ]
    }
}

/** The size of the fiducial markers in mm. */
static FIDUCIAL_SIZE: f64 = 82.5;
static FIDUCIAL_POSITIONS: &[FiducialPosition] = &[
    FiducialPosition { id: 0, x_offset: -105.0 - 280.0 - FIDUCIAL_SIZE / 2.0 },
    FiducialPosition { id: 1, x_offset: -105.0 - FIDUCIAL_SIZE / 2.0 },
    FiducialPosition { id: 2, x_offset: 105.0 + FIDUCIAL_SIZE / 2.0 },
    FiducialPosition { id: 3, x_offset: 105.0 + 280.0 + FIDUCIAL_SIZE / 2.0 },
];

fn setup(
    mut commands: Commands
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 80.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn track_aruco_targets(
    fiducial_detector: Res<FiducialDetector>,
    mut webcam_frame: ResMut<WebcamFrame>,
    mut tracking_data: ResMut<ArucoTrackingData>,
    camera_intrinsics: Res<CameraIntrinsics>,
    mut camera_query: Query<(
        &mut Camera3d,
        &mut Transform
    )>
) {
    let frame = &mut webcam_frame.0;

    let (greyscale, corners, ids, rejected_img_points, latest_rotation, latest_translation) = {
        let data = tracking_data.as_mut();
        (&mut data.greyscale_image, &mut data.corners, &mut data.ids, &mut data.rejected_img_points, &mut data.latest_rotation, &mut data.latest_translation)
    };

    if frame.empty() {
        eprintln!("No frame captured from webcam");
        return;
    }

    // Convert the frame to greyscale
    opencv::imgproc::cvt_color(frame, greyscale, opencv::imgproc::COLOR_BGR2GRAY, 0, AlgorithmHint::ALGO_HINT_DEFAULT).expect("Failed to convert frame to greyscale");

    // Detect ArUco markers in the greyscale frame
    fiducial_detector.0.lock()
        .expect("Failed to lock fiducial detector mutex")
        .detect_markers(greyscale, corners, ids, rejected_img_points)
        .expect("Failed to detect ArUco markers");

    // TEMPORARY
    objdetect::draw_detected_markers(frame, corners, ids, Scalar::new(0.0, 255.0, 0.0, 255.0))
        .expect("Failed to draw detected markers on frame");

    if ids.len() == 0 {
        eprintln!("No ArUco markers detected");
        return;
    }

    let flat_corners: Vector<Point2f> = corners.iter().flatten().collect();

    // Generate only the fiducial corners for the found fiducials
    let fiducial_corners: Vector<Point3d> = ids.iter()
        .filter_map(|id| {
            // It's not a big deal that this is O(n^2) since there are only a few fiducials
            FIDUCIAL_POSITIONS.iter().find(|fiducial| fiducial.id == id)
                .map(|fiducial| fiducial.get_corners())
        })
        .flatten()
        .map(|point| Point3d::new(point.x, point.y, point.z))
        .collect();

    if fiducial_corners.len() != flat_corners.len() {
        eprintln!("Number of fiducial corners ({}) does not match number of detected corners ({})", fiducial_corners.len(), corners.len());
        return;
    }

    // Use SolvePnP to determine the pose of the camera relative to the known markers
    if !calib3d::solve_pnp(
        &fiducial_corners,
        &flat_corners,
        &camera_intrinsics.camera_matrix,
        &camera_intrinsics.dist_coeffs,
        latest_rotation,
        latest_translation,
        false,
        calib3d::SOLVEPNP_IPPE // Requires coplanar markers, which is the case for our fiducials
        // calib3d::SOLVEPNP_ITERATIVE // Can sometimes be better
    ).expect("Failed to solve PnP for ArUco markers") {
        eprintln!("Failed to solve PnP for ArUco markers");
        return;
    }
    
    // Update the camera transform based on the latest rotation and translation
    for (_camera, mut transform) in camera_query.iter_mut() {
        let translation = Vec3::from_slice(&mat3x1_to_arr::<f64>(latest_translation.col(0).expect("No translation for camera")));

        let rotation_vector = mat3x1_to_arr::<f64>(latest_rotation.col(0).expect("No rotation for camera"));
        let mut rotation_matrix = Mat::default();
        calib3d::rodrigues_def(&Mat::from_slice(&rotation_vector).expect("Failed to create rotation matrix from vector"), &mut rotation_matrix)
            .expect("Failed to convert rotation vector to rotation matrix");
        let rotation_matrix: Mat3 = Mat3::from_cols_array_2d(&[
            mat3x1_to_arr::<f32>(rotation_matrix.col(0).expect("No rotation matrix column 0")),
            mat3x1_to_arr::<f32>(rotation_matrix.col(1).expect("No rotation matrix column 1")),
            mat3x1_to_arr::<f32>(rotation_matrix.col(2).expect("No rotation matrix column 2"))
        ]);

        // Update the camera transform based on the inverse of the rotation and translation
        let rotation_matrix_inverse = rotation_matrix.transpose();
        let rotation_inverse = Quat::from_mat3(&rotation_matrix_inverse);
        // Invert translation: -R.T * tvec
        let translation_inverse = -rotation_matrix_inverse.clone() * translation;

        transform.translation = translation_inverse;
        transform.rotation = rotation_inverse;

        println!("Camera transform updated: translation = {:?}, rotation = {:?}", transform.translation, transform.rotation);
    }
}

trait OCVIntoF32 {
    fn into_f32(self) -> f32;
}
impl OCVIntoF32 for f64 {
    fn into_f32(self) -> f32 {
        self as f32
    }
}
impl OCVIntoF32 for f32 {
    fn into_f32(self) -> f32 {
        self
    }
}

fn mat3x1_to_arr<OriginalType: OCVIntoF32>(arr: BoxedRef<'_, Mat>) -> [f32; 3]
    where OriginalType: DataType {
    if arr.rows() != 3 || arr.cols() != 1 {
        panic!("Expected a 3x1 matrix for translation or rotation");
    }
    let mut result = [0.0; 3];
    for i in 0..3 {
        let v = *arr.at_2d::<OriginalType>(i as i32, 0).expect("Failed to get value from matrix");
        result[i] = v.into_f32();
    }
    result
}

#[derive(Deserialize)]
#[allow(unused)]
struct CalibrationData {
    camera: String,
    platform: String,
    avg_reprojection_error: f64,
    camera_matrix: Vec<Vec<f64>>,
    distortion_coefficients: Vec<f64>,
    distortion_model: String,
    img_size: Vec<u32>,
    calibration_time: String
}

impl Plugin for ArUcoCameraPlugin {
    fn build(&self, app: &mut App) {
        // Load assets/calibration.json
        let file_data = fs::read_to_string("assets/calibration.json")
            .expect("Failed to read calibration file");
        let calibration_data: CalibrationData = serde_json::from_str(&file_data)
            .expect("Failed to parse calibration data");

        // Create the camera intrinsics from the calibration data
        let camera_matrix = Mat::from_slice_2d(&calibration_data.camera_matrix)
            .expect("Failed to create camera matrix from calibration data");
        let dist_coeffs = Mat::from_slice(&calibration_data.distortion_coefficients)
            .expect("Failed to create distortion coefficients from calibration data")
            .try_clone()
            .expect("Failed to clone distortion coefficients");

        let camera_intrinsics = CameraIntrinsics {
            camera_matrix,
            dist_coeffs
        };
        
        app
            .insert_resource(FiducialDetector(Mutex::new(
                ArucoDetector::new(
                    &objdetect::get_predefined_dictionary(objdetect::PredefinedDictionaryType::DICT_APRILTAG_25h9).expect("Failed to get predefined dictionary"),
                    &objdetect::DetectorParameters::default().expect("Failed to create detector parameters"),
                    RefineParameters::new(10.0, 3.0, true).expect("Failed to create refine parameters")
                ).expect("Failed to create ArUco detector")
            )))
            .insert_resource(camera_intrinsics)
            .insert_resource(ArucoTrackingData::default())
            .add_systems(Startup, setup)
            .add_systems(Update, track_aruco_targets.in_set(VideoUpdateSystems));
    }
}