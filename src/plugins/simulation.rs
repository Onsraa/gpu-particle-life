use bevy::prelude::*;

use crate::states::simulation::SimulationState;
use crate::states::app::AppState;
use crate::systems::debug_particles::debug_particle_movement;
use crate::systems::{
    collision::detect_food_collision,
    debug::debug_scores,
    movement::physics_simulation_system,
    spatial_grid::{SpatialGrid, update_spatial_grid}, 
    spawning::{spawn_food, spawn_simulations_with_particles, EntitiesSpawned},
    reset::reset_for_new_epoch,
    population_save::{PopulationSaveEvents, AvailablePopulations, process_save_requests, load_available_populations},
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
            .init_resource::<PopulationSaveEvents>()
            .init_resource::<AvailablePopulations>()

            // Charger les populations au démarrage
            .add_systems(Startup, load_available_populations)

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
                    spawn_simulations_with_particles,
                    spawn_food,
                    reset_for_new_epoch,
                    setup_spatial_system_params, 
                ).chain(),
            )

            .add_systems(
                Update,
                transition_to_running
                    .run_if(in_state(SimulationState::Starting))
                    .run_if(in_state(AppState::Simulation)),
            )

            // NOUVEAU : Système physique avec support torus pour CPU
            .add_systems(
                Update,
                physics_simulation_system
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_disabled),
            )

            // MODIFICATION : Système GPU avec fallback spatial pour compatibilité
            .add_systems(
                Update,
                (
                    update_spatial_grid.before(apply_compute_results), // NOUVEAU : Mise à jour avant GPU
                    apply_compute_results,
                )
                    .chain()
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_enabled),
            )

            // Systèmes généraux (inchangés)
            .add_systems(
                Update,
                (
                    detect_food_collision,
                    check_epoch_end,
                    debug_scores,
                    debug_particle_movement,
                    process_save_requests,
                )
                    .chain()
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation)),
            )

            .add_systems(
                Update,
                handle_pause_input
                    .run_if(in_state(AppState::Simulation))
            )

            .add_systems(OnExit(AppState::Simulation), cleanup_all);
    }
}

/// NOUVEAU : Initialise les paramètres du système spatial
fn setup_spatial_system_params(
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

    info!("🌐 Système spatial torus initialisé avec portée {:.0}",
          sim_params.max_force_range);
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
fn transition_to_running(
    mut next_state: ResMut<NextState<SimulationState>>,
    compute_enabled: Res<ComputeEnabled>,
) {
    info!("Transitioning to Running state, GPU compute: {}, Torus spatial: Activé", compute_enabled.0);
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
    mut torus_cache: ResMut<crate::systems::torus_spatial::TorusNeighborCache>, // NOUVEAU
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

    // NOUVEAU : Nettoyer le cache spatial
    torus_cache.neighbors.clear();

    // Réinitialiser le flag
    entities_spawned.0 = false;

    info!("Nettoyage complet de la simulation (y compris cache spatial)");
}