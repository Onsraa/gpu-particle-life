use bevy::prelude::*;

use crate::states::simulation::SimulationState;
use crate::states::app::AppState;
use crate::systems::debug_particles::debug_particle_movement;
use crate::systems::{
    collision::detect_food_collision,
    debug::debug_scores,
    movement::{apply_movement, calculate_forces},
    spatial_grid::{SpatialGrid, update_spatial_grid},
    spawning::{spawn_food, spawn_simulations_with_particles, EntitiesSpawned},
    reset::reset_for_new_epoch,
};
use crate::plugins::compute::{ComputeEnabled, apply_compute_results};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app
            // État de la simulation
            .init_state::<SimulationState>()
            // Ressources
            .init_resource::<SpatialGrid>()
            .init_resource::<EntitiesSpawned>()

            // Transition vers l'état de simulation
            .add_systems(
                OnEnter(AppState::Simulation),
                |mut next_state: ResMut<NextState<SimulationState>>| {
                    next_state.set(SimulationState::Starting);
                },
            )

            // Systèmes de démarrage
            .add_systems(
                OnEnter(SimulationState::Starting),
                (
                    // Spawn initial (ne se fait qu'une fois)
                    spawn_simulations_with_particles,
                    spawn_food,
                    // Reset pour les époques suivantes
                    reset_for_new_epoch,
                ).chain(),
            )
            // Transition automatique vers Running
            .add_systems(
                Update,
                transition_to_running
                    .run_if(in_state(SimulationState::Starting))
                    .run_if(in_state(AppState::Simulation)),
            )
            // Systèmes de simulation CPU (si compute désactivé)
            .add_systems(
                Update,
                (
                    update_spatial_grid,
                    calculate_forces,
                    apply_movement,
                )
                    .chain()
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_disabled),
            )
            // Système de simulation GPU (si compute activé)
            .add_systems(
                Update,
                apply_compute_results
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_enabled),
            )
            // Systèmes communs
            .add_systems(
                Update,
                (
                    detect_food_collision,
                    check_epoch_end,
                    debug_scores,
                    debug_particle_movement,
                )
                    .chain()
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation)),
            )
            // Système de pause
            .add_systems(
                Update,
                handle_pause_input
                    .run_if(in_state(AppState::Simulation))
            )
            // Plus besoin de cleanup à chaque époque !
            // Seulement quand on quitte complètement la simulation
            .add_systems(OnExit(AppState::Simulation), cleanup_all);
    }
}

/// Condition pour vérifier si le compute est activé
fn compute_enabled(compute: Res<ComputeEnabled>) -> bool {
    compute.0
}

/// Condition pour vérifier si le compute est désactivé
fn compute_disabled(compute: Res<ComputeEnabled>) -> bool {
    !compute.0
}

/// Transition automatique de Starting vers Running
fn transition_to_running(mut next_state: ResMut<NextState<SimulationState>>) {
    next_state.set(SimulationState::Running);
}

/// Vérifie si l'époque est terminée
fn check_epoch_end(
    mut sim_params: ResMut<crate::resources::simulation::SimulationParameters>,
    mut next_state: ResMut<NextState<SimulationState>>,
    time: Res<Time>,
) {
    sim_params.tick(time.delta());

    if sim_params.is_epoch_finished() {
        info!("Époque {} terminée!", sim_params.current_epoch);

        // TODO: Passer à l'état de sélection génétique
        // Pour l'instant, on passe directement à la nouvelle époque
        sim_params.start_new_epoch();
        next_state.set(SimulationState::Starting);
    }
}

/// Gestion de la pause (touche Espace)
fn handle_pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<SimulationState>>,
    mut next_state: ResMut<NextState<SimulationState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        match state.get() {
            SimulationState::Running => {
                info!("Simulation en pause");
                next_state.set(SimulationState::Paused);
            }
            SimulationState::Paused => {
                info!("Reprise de la simulation");
                next_state.set(SimulationState::Running);
            }
            _ => {}
        }
    }
}

/// Nettoie tout quand on quitte la simulation complètement
fn cleanup_all(
    mut commands: Commands,
    simulations: Query<Entity, With<crate::components::simulation::Simulation>>,
    food: Query<Entity, With<crate::components::food::Food>>,
    cameras: Query<Entity, With<crate::systems::viewport_manager::ViewportCamera>>,
    mut entities_spawned: ResMut<EntitiesSpawned>,
) {
    // Supprimer toutes les simulations et leurs particules
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }

    // Supprimer toute la nourriture
    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    // Supprimer les caméras de viewport
    for entity in cameras.iter() {
        commands.entity(entity).despawn();
    }

    // Réinitialiser le flag
    entities_spawned.0 = false;

    info!("Nettoyage complet de la simulation");
}