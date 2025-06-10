use bevy::{app::{Plugin, Update}, ecs::{component::Component, entity::Entity, system::{Commands, Query}}};

#[derive(Component)]
pub struct DeleteAfterOneFrame;

fn testing_system(
    mut commands: Commands,
    query: Query<(&DeleteAfterOneFrame, Entity)>,
) {
    for (_, entity) in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub struct TestingPlugin;

impl Plugin for TestingPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Update, testing_system);
    }
}