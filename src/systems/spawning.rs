use bevy::prelude::*;
use rand::Rng;

use crate::components::{
    particle::{ParticleBundle},
    food::{FoodBundle},
    simulation::{SimulationBundle},
    genotype::Genotype,
};
use crate::resources::{
    grid::GridParameters,
    particle_types::ParticleTypesConfig,
    food::FoodParameters,
    simulation::SimulationParameters,
};
use crate::globals::*;

/// Ressource pour stocker les positions de nourriture entre époques
#[derive(Resource, Clone)]
pub struct FoodPositions(pub Vec<Vec3>);

/// Spawn toutes les simulations avec leurs particules
/// Note: Les positions des particules sont régénérées à chaque époque
pub fn spawn_simulations_with_particles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
    particle_config: Res<ParticleTypesConfig>,
    simulation_params: Res<SimulationParameters>,
) {
    let mut rng = rand::thread_rng();

    // Créer un mesh partagé pour toutes les particules
    let particle_mesh = meshes.add(Sphere::new(PARTICLE_RADIUS)
        .mesh()
        .subdivisions(PARTICLE_SUBDIVISIONS));

    // Créer les matériaux pour chaque type
    let particle_materials: Vec<_> = particle_config.colors.iter()
        .map(|&color| materials.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * 0.2,
            ..default()
        }))
        .collect();

    // Pour chaque simulation
    for sim_id in 0..simulation_params.simulation_count {
        let genotype = Genotype::random(particle_config.type_count);

        // Préparer les enfants (particules) pour cette simulation
        let mut particle_children = Vec::new();

        // Pour chaque type de particule
        for particle_type in 0..particle_config.type_count {
            let particles_per_type = simulation_params.particle_count / particle_config.type_count;

            // Créer toutes les particules de ce type
            for _ in 0..particles_per_type {
                let position = random_position_in_grid(&grid, &mut rng);

                particle_children.push(ParticleBundle::new(
                    particle_type,
                    position,
                    particle_mesh.clone(),
                    particle_materials[particle_type].clone(),
                ));
            }
        }

        // Spawn la simulation avec toutes ses particules comme enfants
        commands.spawn((
            SimulationBundle::new(sim_id, genotype),
            children![particle_children],
        ));
    }
}

/// Spawn la nourriture en réutilisant les positions si elles existent
/// Note: Les positions de nourriture restent identiques entre toutes les époques
/// pour assurer l'équité entre les simulations
pub fn spawn_food(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
    food_params: Res<FoodParameters>,
    existing_positions: Option<Res<FoodPositions>>,
) {
    let mut rng = rand::thread_rng();

    // Mesh partagé pour toute la nourriture
    let food_mesh = meshes.add(Sphere::new(FOOD_RADIUS)
        .mesh()
        .subdivisions(PARTICLE_SUBDIVISIONS));

    // Matériau blanc pour la nourriture
    let food_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::WHITE * 0.5,
        ..default()
    });

    // Utiliser les positions existantes ou en générer de nouvelles
    let food_positions = if let Some(existing) = existing_positions {
        existing.0.clone()
    } else {
        // Première époque : générer et stocker les positions
        let new_positions: Vec<Vec3> = (0..food_params.food_count)
            .map(|_| random_position_in_grid(&grid, &mut rng))
            .collect();

        // Sauvegarder pour les époques suivantes
        commands.insert_resource(FoodPositions(new_positions.clone()));
        new_positions
    };

    // Spawn la nourriture
    for position in food_positions {
        let respawn_time = if food_params.respawn_enabled {
            Some(food_params.respawn_cooldown)
        } else {
            None
        };

        commands.spawn((
            FoodBundle::new(position, food_params.food_value, respawn_time),
            Mesh3d(food_mesh.clone()),
            MeshMaterial3d(food_material.clone()),
        ));
    }
}

/// Génère une position aléatoire dans la grille
fn random_position_in_grid(grid: &GridParameters, rng: &mut impl Rng) -> Vec3 {
    let half_width = grid.width / 2.0;
    let half_height = grid.height / 2.0;
    let half_depth = grid.depth / 2.0;

    Vec3::new(
        rng.gen_range(-half_width..half_width),
        rng.gen_range(-half_height..half_height),
        rng.gen_range(-half_depth..half_depth),
    )
}