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
use crate::plugins::compute::{ComputeEnabled, ComputeSystemSet, apply_compute_results_system};

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
                    setup_visualizer_spatial_params,
                ).chain(),
            )

            // Système CPU
            .add_systems(
                Update,
                (
                    physics_simulation_system
                        .in_set(ComputeSystemSet::Execute),
                    detect_food_collision
                        .after(ComputeSystemSet::Execute),
                )
                    .run_if(in_state(AppState::Visualization))
                    .run_if(compute_disabled),
            )

            // MODIFICATION : Système GPU avec set spécifique pour éviter les conflits
            .add_systems(
                Update,
                (
                    apply_compute_results_system
                        .in_set(ComputeSystemSet::ApplyResults)
                        .after(ComputeSystemSet::Execute),
                    detect_food_collision
                        .after(ComputeSystemSet::ApplyResults),
                )
                    .run_if(in_state(AppState::Visualization))
                    .run_if(compute_enabled),
            )

            // Nettoyage en sortant
            .add_systems(OnExit(AppState::Visualization), cleanup_visualization);
    }
}

// Reste du code inchangé...

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
    mut torus_cache: ResMut<crate::systems::torus_spatial::TorusNeighborCache>,
) {
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }
    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    // Nettoyer le cache spatial
    torus_cache.neighbors.clear();

    info!("Nettoyage de la visualisation terminé (y compris cache spatial)");
}