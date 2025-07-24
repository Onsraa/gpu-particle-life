use bevy::prelude::*;

use crate::states::app::AppState;
use crate::states::simulation::SimulationState;
use crate::systems::debug_particles::debug_particle_movement;
use crate::systems::{
    collision::detect_food_collision,
    debug::debug_scores,
    movement::physics_simulation_system,
    population_save::{
        AvailablePopulations, PopulationSaveEvents, load_available_populations,
        process_save_requests,
    },
    reset::reset_for_new_epoch,
    spawning::{EntitiesSpawned, spawn_food, spawn_simulations_with_particles},
};
use crate::plugins::compute::ComputeEnabled;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<SimulationState>()
            .init_resource::<EntitiesSpawned>()
            .init_resource::<PopulationSaveEvents>()
            .init_resource::<AvailablePopulations>()
            .add_systems(Startup, load_available_populations)
            .add_systems(
                OnEnter(AppState::Simulation),
                |mut next_state: ResMut<NextState<SimulationState>>| {
                    next_state.set(SimulationState::Starting);
                },
            )
            .add_systems(
                OnEnter(SimulationState::Starting),
                (
                    spawn_simulations_with_particles,
                    spawn_food,
                    reset_for_new_epoch,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                transition_to_running
                    .run_if(in_state(SimulationState::Starting))
                    .run_if(in_state(AppState::Simulation)),
            )
            // Système physique CPU seulement quand GPU désactivé
            .add_systems(
                Update,
                physics_simulation_system
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_disabled),
            )
            // Systèmes généraux
            .add_systems(
                Update,
                (
                    detect_food_collision,
                    check_epoch_end,
                    debug_scores,
                    debug_particle_movement,
                    process_save_requests,
                )
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation)),
            )
            .add_systems(
                Update,
                handle_pause_input.run_if(in_state(AppState::Simulation)),
            )
            .add_systems(OnExit(AppState::Simulation), cleanup_all);
    }
}

fn compute_disabled(compute: Res<ComputeEnabled>) -> bool {
    !compute.0
}

fn transition_to_running(
    mut next_state: ResMut<NextState<SimulationState>>,
    compute_enabled: Res<ComputeEnabled>,
) {
    info!("Transitioning to Running state, GPU compute: {}", compute_enabled.0);
    next_state.set(SimulationState::Running);
}

fn check_epoch_end(
    mut sim_params: ResMut<crate::resources::simulation::SimulationParameters>,
    mut next_state: ResMut<NextState<SimulationState>>,
    time: Res<Time>,
) {
    sim_params.tick(time.delta());

    if sim_params.is_epoch_finished() {
        info!("Époque {} terminée!", sim_params.current_epoch);
        sim_params.start_new_epoch();
        next_state.set(SimulationState::Starting);
    }
}

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