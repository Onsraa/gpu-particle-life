use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use rand::Rng;

use crate::components::{
    food::{Food, FoodRespawnTimer, FoodValue},
    genotype::Genotype,
    particle::{Particle, ParticleType},
    simulation::{Simulation, SimulationId},
    score::Score,
};
use crate::globals::*;
use crate::resources::{
    food::FoodParameters, grid::GridParameters, particle_types::ParticleTypesConfig,
    simulation::SimulationParameters,
};

/// Ressource pour stocker les positions de nourriture entre époques
#[derive(Resource, Clone)]
pub struct FoodPositions(pub Vec<Vec3>);

/// Marqueur pour indiquer que les entités ont déjà été créées
#[derive(Resource, Default)]
pub struct EntitiesSpawned(pub bool);

/// Spawn toutes les simulations avec leurs particules (première fois uniquement)
pub fn spawn_simulations_with_particles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
    particle_config: Res<ParticleTypesConfig>,
    simulation_params: Res<SimulationParameters>,
    mut entities_spawned: ResMut<EntitiesSpawned>,
    // Query pour vérifier si des entités existent déjà
    existing_simulations: Query<Entity, With<Simulation>>,
) {
    // Si les entités ont déjà été créées, on ne fait rien
    if entities_spawned.0 || !existing_simulations.is_empty() {
        return;
    }

    let mut rng = rand::rng();

    // Créer un mesh partagé pour toutes les particules
    let particle_mesh = meshes.add(
        Sphere::new(PARTICLE_RADIUS)
            .mesh()
            .ico(PARTICLE_SUBDIVISIONS)
            .unwrap(),
    );

    // Créer les matériaux pour chaque type avec émissive
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

    // Générer les positions initiales pour toutes les particules
    // Ces positions seront les mêmes pour toutes les simulations
    let particles_per_type = simulation_params.particle_count / particle_config.type_count;
    let mut initial_positions = Vec::new();

    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            initial_positions.push((particle_type, random_position_in_grid(&grid, &mut rng)));
        }
    }

    // Pour chaque simulation
    for sim_id in 0..simulation_params.simulation_count {
        let genotype = Genotype::random(particle_config.type_count);

        // Spawn la simulation avec son RenderLayer
        commands
            .spawn((
                Simulation,
                SimulationId(sim_id),
                genotype,
                Score::default(),
                // Assigner le RenderLayer à la simulation (layer sim_id + 1)
                RenderLayers::layer(sim_id + 1),
            ))
            .with_children(|parent| {
                // Spawn toutes les particules comme enfants avec les positions communes
                for (particle_type, position) in &initial_positions {
                    parent.spawn((
                        Particle,
                        ParticleType(*particle_type),
                        Transform::from_translation(*position),
                        Mesh3d(particle_mesh.clone()),
                        MeshMaterial3d(particle_materials[*particle_type].clone()),
                        // Les particules héritent automatiquement du RenderLayer du parent
                        RenderLayers::layer(sim_id + 1),
                    ));
                }
            });
    }

    // Marquer que les entités ont été créées
    entities_spawned.0 = true;
    info!("Création initiale des {} simulations avec {} particules chacune", 
          simulation_params.simulation_count, 
          simulation_params.particle_count);
}

/// Spawn la nourriture (première fois uniquement)
pub fn spawn_food(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
    food_params: Res<FoodParameters>,
    existing_food: Query<Entity, With<Food>>,
) {
    // Si la nourriture existe déjà, on ne fait rien
    if !existing_food.is_empty() {
        return;
    }

    let mut rng = rand::rng();

    // Mesh partagé pour toute la nourriture
    let food_mesh = meshes.add(
        Sphere::new(FOOD_RADIUS)
            .mesh()
            .ico(PARTICLE_SUBDIVISIONS)
            .unwrap(),
    );

    // Matériau blanc émissif pour la nourriture
    let food_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::WHITE,
        unlit: true,
        ..default()
    });

    // Générer les positions initiales
    let food_positions: Vec<Vec3> = (0..food_params.food_count)
        .map(|_| random_position_in_grid(&grid, &mut rng))
        .collect();

    // Sauvegarder les positions
    commands.insert_resource(FoodPositions(food_positions.clone()));

    // Spawn la nourriture
    for position in food_positions {
        let respawn_timer = if food_params.respawn_enabled {
            Some(Timer::from_seconds(
                food_params.respawn_cooldown,
                TimerMode::Once,
            ))
        } else {
            None
        };

        commands.spawn((
            Food,
            FoodValue(food_params.food_value),
            FoodRespawnTimer(respawn_timer),
            Transform::from_translation(position),
            Mesh3d(food_mesh.clone()),
            MeshMaterial3d(food_material.clone()),
            RenderLayers::layer(0),
        ));
    }

    info!("Création initiale de {} nourritures", food_params.food_count);
}

/// Génère une position aléatoire dans la grille
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