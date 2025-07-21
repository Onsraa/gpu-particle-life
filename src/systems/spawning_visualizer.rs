use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use rand::Rng;

use crate::components::{
    food::{Food, FoodRespawnTimer, FoodValue},
    particle::{Particle, ParticleType},
    simulation::{Simulation, SimulationId},
    score::Score,
};
use crate::globals::*;
use crate::resources::{
    food::FoodParameters, grid::GridParameters, particle_types::ParticleTypesConfig,
    simulation::SimulationParameters,
};
use crate::ui::visualizer::VisualizerGenome;

/// Spawn une seule simulation avec le génome spécifique du visualiseur
pub fn spawn_visualizer_simulation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
    particle_config: Res<ParticleTypesConfig>,
    simulation_params: Res<SimulationParameters>,
    visualizer_genome: Res<VisualizerGenome>,
    existing_simulations: Query<Entity, With<Simulation>>,
) {
    // Ne spawn qu'une fois
    if !existing_simulations.is_empty() {
        return;
    }

    let mut rng = rand::rng();

    // Mesh et matériaux pour les particules
    let particle_mesh = meshes.add(
        Sphere::new(PARTICLE_RADIUS)
            .mesh()
            .ico(PARTICLE_SUBDIVISIONS)
            .unwrap(),
    );

    let particle_materials: Vec<_> = (0..particle_config.type_count)
        .map(|i| {
            let (base_color, emissive) = particle_config.get_color_for_type(i);
            materials.add(StandardMaterial {
                base_color,
                emissive,
                unlit: true,
                ..default()
            })
        })
        .collect();

    // Calculer les positions initiales
    let particles_per_type = (simulation_params.particle_count + particle_config.type_count - 1) / particle_config.type_count;
    let mut initial_positions = Vec::new();

    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            initial_positions.push((particle_type, random_position_in_grid(&grid, &mut rng)));
        }
    }

    // Spawn la simulation unique avec le génome du visualiseur
    commands
        .spawn((
            Simulation,
            SimulationId(0), // Toujours ID 0 pour le visualiseur
            visualizer_genome.0, // Utiliser le génome sauvegardé
            Score::default(),
            RenderLayers::layer(1),
        ))
        .with_children(|parent| {
            for (particle_type, position) in &initial_positions {
                parent.spawn((
                    Particle,
                    ParticleType(*particle_type),
                    Transform::from_translation(*position),
                    Mesh3d(particle_mesh.clone()),
                    MeshMaterial3d(particle_materials[*particle_type].clone()),
                    RenderLayers::layer(1),
                ));
            }
        });

    info!("Simulation de visualisation créée avec le génome sauvegardé");
}

fn random_position_in_grid(grid: &GridParameters, rng: &mut impl Rng) -> Vec3 {
    let half_width = grid.width / 2.0;
    let half_height = grid.height / 2.0;
    let half_depth = grid.depth / 2.0;

    Vec3::new(
        rng.random_range(-half_width..half_width),
        rng.random_range(-half_height..half_height),
        rng.random_range(-half_depth..half_depth),
    )
}
