use bevy::prelude::*;

use crate::states::simulation::SimulationState;
use crate::systems::{
    spawning::{spawn_simulations_with_particles, spawn_food},
    movement::{calculate_forces, apply_movement},
    collision::detect_food_collision,
    debug::debug_scores,
    spatial_grid::{SpatialGrid, update_spatial_grid},
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app
            // État de la simulation
            .init_state::<SimulationState>()

            // Ressource pour la grille spatiale
            .init_resource::<SpatialGrid>()

            // Systèmes de démarrage (une seule fois au début de l'époque)
            .add_systems(OnEnter(SimulationState::Starting), (
                spawn_simulations_with_particles,
                spawn_food,
            ).chain())

            // Transition automatique vers Running
            .add_systems(Update,
                         transition_to_running.run_if(in_state(SimulationState::Starting))
            )

            // Systèmes de simulation
            .add_systems(Update, (
                update_spatial_grid, // IMPORTANT: doit être avant calculate_forces
                calculate_forces,
                apply_movement,
                detect_food_collision,
                check_epoch_end,
                debug_scores,
            ).chain().run_if(in_state(SimulationState::Running)))

            // Système de pause
            .add_systems(Update,
                         handle_pause_input
            )

            // Nettoyage en sortie d'époque
            .add_systems(OnExit(SimulationState::Running),
                         cleanup_epoch
            );
    }
}

/// Transition automatique de Starting vers Running
fn transition_to_running(
    mut next_state: ResMut<NextState<SimulationState>>,
) {
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
            },
            SimulationState::Paused => {
                info!("Reprise de la simulation");
                next_state.set(SimulationState::Running);
            },
            _ => {}
        }
    }
}

/// Nettoie toutes les entités de la simulation précédente
fn cleanup_epoch(
    mut commands: Commands,
    simulations: Query<Entity, With<crate::components::simulation::Simulation>>,
    food: Query<Entity, With<crate::components::food::Food>>,
) {
    // Supprimer toutes les simulations (et leurs enfants automatiquement)
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }

    // Supprimer toute la nourriture
    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    info!("Nettoyage de l'époque terminé");
}