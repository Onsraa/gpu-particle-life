use bevy::prelude::*;
use crate::states::app::AppState;
use crate::systems::spawning_visualizer::spawn_visualizer_simulation;
use crate::systems::{
    collision::detect_food_collision,
    movement::physics_simulation_system, // NOUVEAU : système physique unifié
    spatial_grid::SpatialGrid,
    spawning::spawn_food,
};
use crate::plugins::compute::{ComputeEnabled, apply_compute_results};

pub struct VisualizerPlugin;

impl Plugin for VisualizerPlugin {
    fn build(&self, app: &mut App) {
        app
            // Systèmes d'entrée dans le mode visualisation
            .add_systems(
                OnEnter(AppState::Visualization),
                (spawn_visualizer_simulation, spawn_food).chain(),
            )

            // NOUVEAU : Utiliser le système physique unifié pour CPU
            .add_systems(
                Update,
                (
                    physics_simulation_system, // CHANGEMENT : système unifié
                    detect_food_collision,
                )
                    .chain()
                    .run_if(in_state(AppState::Visualization))
                    .run_if(compute_disabled),
            )

            // Système GPU (inchangé)
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
) {
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }
    for entity in food.iter() {
        commands.entity(entity).despawn();
    }
    info!("Nettoyage de la visualisation terminé");
}