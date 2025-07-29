use crate::plugins::simulation::compute::ComputeEnabled;
use crate::states::app::AppState;
use crate::states::simulation::SimulationState;
use crate::systems::lifecycle::*;
use crate::systems::persistence::population_save::*;
use crate::systems::rendering::viewport_manager::ViewportCamera;
use crate::systems::simulation::collision::detect_food_collision;
use crate::systems::simulation::physics::*;
use crate::systems::simulation::reset::*;
use crate::systems::simulation::spawning::*;
use bevy::prelude::*;
use crate::components::entities::food::Food;
use crate::components::entities::simulation::Simulation;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SimulationState>()
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
    info!(
        "Transitioning to Running state, GPU compute: {}",
        compute_enabled.0
    );
    next_state.set(SimulationState::Running);
}

fn cleanup_all(
    mut commands: Commands,
    simulations: Query<Entity, With<Simulation>>,
    food: Query<Entity, With<Food>>,
    cameras: Query<Entity, With<ViewportCamera>>,
    mut entities_spawned: ResMut<EntitiesSpawned>,
) {
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }

    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    for entity in cameras.iter() {
        commands.entity(entity).despawn();
    }

    entities_spawned.0 = false;

    info!("Nettoyage complet de la simulation");
}
