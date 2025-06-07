
//! Illustrates bloom post-processing using HDR and emissive materials.

use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode},
        tonemapping::Tonemapping,
    },
    math::ops,
    prelude::*,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (update_bloom_settings, bounce_spheres))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Tonemapping::TonyMcMapface, // 1. Using a tonemapper that desaturates to white is recommended
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Bloom::NATURAL, // 2. Enable bloom for the camera
    ));
}