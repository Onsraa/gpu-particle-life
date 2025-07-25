use bevy::prelude::*;
use crate::states::app::AppState;
use crate::systems::{
    collision::detect_food_collision,
    movement::physics_simulation_system,
    spawning::spawn_food,
    spawning_visualizer::spawn_visualizer_simulation,
};
use crate::plugins::compute::ComputeEnabled;
use crate::components::{
    genotype::Genotype,
    particle::{Particle, ParticleType, Velocity},
    simulation::{Simulation, SimulationId},
    food::Food,
};
use crate::resources::boundary::BoundaryMode;
use crate::resources::{grid::GridParameters, simulation::SimulationParameters};

pub struct VisualizerPlugin;

impl Plugin for VisualizerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnEnter(AppState::Visualization),
                (
                    spawn_visualizer_simulation,
                    spawn_food,
                ).chain(),
            )
            // Système CPU uniquement - avec wrapper pour éviter les conflits
            .add_systems(
                Update,
                (
                    visualizer_physics_system,
                    visualizer_food_collision_cpu,
                )
                    .chain()
                    .run_if(in_state(AppState::Visualization))
                    .run_if(compute_disabled),
            )
            // Système GPU (si activé) - avec wrapper pour éviter les conflits
            .add_systems(
                Update,
                visualizer_food_collision_gpu
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
        (Entity, &mut Transform, &mut Velocity, &ParticleType, &ChildOf),
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

/// Wrapper CPU pour detect_food_collision - évite les conflits avec SimulationPlugin
fn visualizer_food_collision_cpu(
    commands: Commands,
    time: Res<Time>,
    particles: Query<(&Transform, &ChildOf), With<Particle>>,
    food_query: Query<
        (
            Entity,
            &Transform,
            &crate::components::food::FoodValue,
            &mut crate::components::food::FoodRespawnTimer,
            &ViewVisibility,
        ),
        With<Food>,
    >,
    simulations: Query<&mut crate::components::score::Score, With<Simulation>>,
) {
    detect_food_collision(commands, time, particles, food_query, simulations);
}

/// Wrapper GPU pour detect_food_collision - évite les conflits avec SimulationPlugin
fn visualizer_food_collision_gpu(
    commands: Commands,
    time: Res<Time>,
    particles: Query<(&Transform, &ChildOf), With<Particle>>,
    food_query: Query<
        (
            Entity,
            &Transform,
            &crate::components::food::FoodValue,
            &mut crate::components::food::FoodRespawnTimer,
            &ViewVisibility,
        ),
        With<Food>,
    >,
    simulations: Query<&mut crate::components::score::Score, With<Simulation>>,
) {
    detect_food_collision(commands, time, particles, food_query, simulations);
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

    info!("🧹 Nettoyage de la visualisation terminé");
}