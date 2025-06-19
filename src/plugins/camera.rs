use crate::resources::camera::CameraSettings;
use crate::systems::camera::orbit;
use crate::systems::viewport_manager::ViewportCamera;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
        app.add_systems(Startup, setup_default_camera);
        app.add_systems(Update, manage_default_camera);
    }
}

/// Marqueur pour la caméra par défaut
#[derive(Component)]
struct DefaultCamera;

/// Configure une caméra par défaut au démarrage
fn setup_default_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(500.0, 500.0, 500.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        DefaultCamera,
        RenderLayers::from_layers(&[0, 1]),
    ));
}

/// Désactive la caméra par défaut quand des viewports sont créés
fn manage_default_camera(
    mut commands: Commands,
    default_camera: Query<Entity, With<DefaultCamera>>,
    viewport_cameras: Query<Entity, With<ViewportCamera>>,
) {
    // S'il y a des caméras de viewport, supprimer la caméra par défaut
    if !viewport_cameras.is_empty() {
        for entity in default_camera.iter() {
            commands.entity(entity).despawn();
        }
    }
    // S'il n'y a plus de caméras de viewport et pas de caméra par défaut, en créer une
    else if viewport_cameras.is_empty() && default_camera.is_empty() {
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(500.0, 500.0, 500.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            DefaultCamera,
            RenderLayers::from_layers(&[0, 1]),
        ));
    }
}