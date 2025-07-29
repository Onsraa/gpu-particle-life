use crate::plugins::simulation::compute::ComputeEnabled;
use crate::resources::config::simulation::SimulationParameters;
use crate::resources::world::boundary::BoundaryMode;
use crate::resources::world::grid::GridParameters;
use crate::states::app::AppState;
use crate::systems::simulation::collision::detect_food_collision;
use crate::systems::simulation::physics::physics_simulation_system;
use crate::systems::simulation::spawning::spawn_food;
use crate::systems::simulation::visualizer_spawning::spawn_visualizer_simulation;
use bevy::prelude::*;
use crate::components::entities::food::Food;
use crate::components::entities::particle::{Particle, ParticleType, Velocity};
use crate::components::entities::simulation::{Simulation, SimulationId};
use crate::components::genetics::genotype::Genotype;

pub struct VisualizerPlugin;

impl Plugin for VisualizerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::Visualization),
            (spawn_visualizer_simulation, spawn_food).chain(),
        )
        // Système CPU uniquement
        .add_systems(
            Update,
            (
                visualizer_physics_system,
                detect_food_collision.after(visualizer_physics_system),
            )
                .run_if(in_state(AppState::Visualization))
                .run_if(compute_disabled),
        )
        // Système GPU (si activé)
        .add_systems(
            Update,
            detect_food_collision
                .run_if(in_state(AppState::Visualization))
                .run_if(compute_enabled),
        )
        .add_systems(OnExit(AppState::Visualization), cleanup_visualization);
    }
}

fn compute_enabled(compute: Res<ComputeEnabled>) -> bool {
    compute.0
}

fn compute_disabled(compute: Res<ComputeEnabled>) -> bool {
    !compute.0
}

/// Wrapper pour le système physique du visualizer (évite les conflits de noms)
fn visualizer_physics_system(
    sim_params: Res<SimulationParameters>,
    grid: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
    mut particles: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &ParticleType,
            &ChildOf,
        ),
        With<Particle>,
    >,
    food_query: Query<(&Transform, &ViewVisibility), (With<Food>, Without<Particle>)>,
) {
    physics_simulation_system(
        sim_params,
        grid,
        boundary_mode,
        simulations,
        particles,
        food_query,
    );
}

fn cleanup_visualization(
    mut commands: Commands,
    simulations: Query<Entity, With<Simulation>>,
    food: Query<Entity, With<Food>>,
) {
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }
    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    info!("Nettoyage de la visualisation terminé");
}
