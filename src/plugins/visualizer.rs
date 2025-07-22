use bevy::prelude::*;
use crate::states::app::AppState;
use crate::systems::spawning_visualizer::spawn_visualizer_simulation;
use crate::systems::{
    collision::detect_food_collision,
    movement::physics_simulation_system,
    spatial_grid::SpatialGrid,
    spawning::spawn_food,
    torus_spatial::TorusSpatialPlugin, 
};
use crate::plugins::compute::{ComputeEnabled, apply_compute_results};

pub struct VisualizerPlugin;

impl Plugin for VisualizerPlugin {
    fn build(&self, app: &mut App) {
        app
            // Systèmes d'entrée dans le mode visualisation
            .add_systems(
                OnEnter(AppState::Visualization),
                (
                    spawn_visualizer_simulation,
                    spawn_food,
                    setup_visualizer_spatial_params, // NOUVEAU
                ).chain(),
            )

            .add_systems(
                Update,
                (
                    physics_simulation_system, 
                    detect_food_collision,
                )
                    .chain()
                    .run_if(in_state(AppState::Visualization))
                    .run_if(compute_disabled),
            )

            // Système GPU (avec fallback spatial pour compatibilité)
            .add_systems(
                Update,
                (
                    apply_compute_results,
                    detect_food_collision,
                )
                    .chain()
                    .run_if(in_state(AppState::Visualization))
                    .run_if(compute_enabled),
            )

            // Nettoyage en sortant
            .add_systems(OnExit(AppState::Visualization), cleanup_visualization);
    }
}

/// NOUVEAU : Initialise les paramètres spatiaux pour le visualizer
fn setup_visualizer_spatial_params(
    mut torus_cache: ResMut<crate::systems::torus_spatial::TorusNeighborCache>,
    grid_params: Res<crate::resources::grid::GridParameters>,
    sim_params: Res<crate::resources::simulation::SimulationParameters>,
) {
    // Configurer le cache torus avec les paramètres de grille
    torus_cache.update_grid_bounds(
        grid_params.width,
        grid_params.height,
        grid_params.depth,
    );

    // Définir la distance de recherche maximale
    torus_cache.max_search_distance = sim_params.max_force_range;

    info!("🌐 Système spatial torus initialisé pour le visualizer avec portée {:.0}",
          sim_params.max_force_range);
}

fn compute_enabled(compute: Res<ComputeEnabled>) -> bool {
    compute.0
}

fn compute_disabled(compute: Res<ComputeEnabled>) -> bool {
    !compute.0
}

fn cleanup_visualization(
    mut commands: Commands,
    simulations: Query<Entity, With<crate::components::simulation::Simulation>>,
    food: Query<Entity, With<crate::components::food::Food>>,
    mut torus_cache: ResMut<crate::systems::torus_spatial::TorusNeighborCache>, // NOUVEAU
) {
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }
    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    // NOUVEAU : Nettoyer le cache spatial
    torus_cache.neighbors.clear();

    info!("Nettoyage de la visualisation terminé (y compris cache spatial)");
}