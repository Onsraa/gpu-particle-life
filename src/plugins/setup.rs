use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use crate::resources::{
    grid::GridParameters,
    simulation::SimulationParameters,
    particle_types::ParticleTypesConfig,
    food::FoodParameters,
};
use crate::resources::boundary::BoundaryMode;
use crate::states::app::AppState;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
        app.init_resource::<GridParameters>();
        app.init_resource::<ParticleTypesConfig>();
        app.init_resource::<SimulationParameters>();
        app.init_resource::<FoodParameters>();
        app.init_resource::<BoundaryMode>();

        app
            .add_systems(
                OnEnter(AppState::Simulation),
                setup_grid_visualization
            )
            .add_systems(
                OnExit(AppState::Simulation),
                cleanup_grid_visualization
            );
    }
}

fn setup_grid_visualization(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
) {
    // Créer un cube wireframe pour visualiser les limites
    let grid_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.3, 0.3, 0.1),
        emissive: LinearRgba::rgb(0.3, 0.3, 0.3),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(grid.width, grid.height, grid.depth))),
        MeshMaterial3d(grid_material),
        Transform::from_translation(Vec3::ZERO),
        RenderLayers::layer(0),
        GridVisualization,
    ));
}

/// Marqueur pour la visualisation de la grille
#[derive(Component)]
struct GridVisualization;

fn cleanup_grid_visualization(
    mut commands: Commands,
    grid_viz: Query<Entity, With<GridVisualization>>,
) {
    for entity in grid_viz.iter() {
        commands.entity(entity).despawn();
    }
}