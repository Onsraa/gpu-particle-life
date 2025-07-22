use crate::resources::camera::CameraSettings;
use crate::resources::grid::GridParameters; // AJOUT pour l'adaptation
use crate::systems::camera::orbit;
use crate::systems::viewport_manager::ViewportCamera;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
        app.add_systems(Startup, setup_default_camera);
        app.add_systems(Update, (manage_default_camera, update_default_camera_distance)); // AJOUT du système d'adaptation
    }
}

/// Marqueur pour la caméra par défaut
#[derive(Component)]
struct DefaultCamera;

/// NOUVEAU : Calcule la distance adaptative pour la caméra par défaut
fn calculate_default_camera_distance(grid: &GridParameters) -> f32 {
    // Calculer la diagonale 3D de la grille
    let diagonal_3d = (grid.width.powi(2) + grid.height.powi(2) + grid.depth.powi(2)).sqrt();

    // Distance pour voir confortablement toute la grille
    let distance = diagonal_3d * 0.85;

    distance.max(300.0) // Distance minimale de sécurité
}

/// Configure une caméra par défaut au démarrage - AMÉLIORÉ
fn setup_default_camera(
    mut commands: Commands,
    grid_params: Res<GridParameters>, // AJOUT pour l'adaptation immédiate
) {
    let camera_distance = calculate_default_camera_distance(&grid_params);

    let camera_position = Vec3::new(
        camera_distance * 0.7,
        camera_distance * 0.8,
        camera_distance * 0.7
    );

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(camera_position)
            .looking_at(Vec3::ZERO, Vec3::Y),
        DefaultCamera,
        RenderLayers::from_layers(&[0, 1]),
    ));

    info!("🎥 Caméra par défaut positionnée à distance: {:.0}", camera_distance);
}

/// NOUVEAU : Système pour adapter la distance de la caméra par défaut si la grille change
fn update_default_camera_distance(
    grid_params: Res<GridParameters>,
    mut default_cameras: Query<&mut Transform, With<DefaultCamera>>,
    mut camera_settings: ResMut<CameraSettings>,
) {
    // Ne s'exécute que si les paramètres de grille ont changé
    if !grid_params.is_changed() {
        return;
    }

    let new_distance = calculate_default_camera_distance(&grid_params);

    // Mettre à jour la distance d'orbite dans les paramètres
    camera_settings.orbit_distance = new_distance;

    // Mettre à jour la position de la caméra par défaut si elle existe
    for mut transform in default_cameras.iter_mut() {
        let new_position = Vec3::new(
            new_distance * 0.7,
            new_distance * 0.8,
            new_distance * 0.7
        );

        *transform = Transform::from_translation(new_position)
            .looking_at(Vec3::ZERO, Vec3::Y);
    }

    info!("🔄 Caméra par défaut adaptée à la nouvelle grille - Distance: {:.0}", new_distance);
}

/// Désactive la caméra par défaut quand des viewports sont créés
fn manage_default_camera(
    mut commands: Commands,
    default_camera: Query<Entity, With<DefaultCamera>>,
    viewport_cameras: Query<Entity, With<ViewportCamera>>,
    grid_params: Res<GridParameters>,
) {
    // S'il y a des caméras de viewport, supprimer la caméra par défaut
    if !viewport_cameras.is_empty() {
        for entity in default_camera.iter() {
            commands.entity(entity).despawn();
        }
    }
    // S'il n'y a plus de caméras de viewport et pas de caméra par défaut, en créer une
    else if viewport_cameras.is_empty() && default_camera.is_empty() {
        let camera_distance = calculate_default_camera_distance(&grid_params);

        let camera_position = Vec3::new(
            camera_distance * 0.7,
            camera_distance * 0.8,
            camera_distance * 0.7
        );

        commands.spawn((
            Camera3d::default(),
            Transform::from_translation(camera_position)
                .looking_at(Vec3::ZERO, Vec3::Y),
            DefaultCamera,
            RenderLayers::from_layers(&[0, 1]),
        ));

        info!("🎥 Caméra par défaut recréée avec distance adaptée: {:.0}", camera_distance);
    }
}