use bevy::prelude::*;
use rand::Rng;

use crate::components::{
    particle::{Particle, ParticleType, Velocity},
    simulation::{Simulation, SimulationId},
    genotype::Genotype,
    score::Score,
    food::{Food, FoodRespawnTimer},
};
use crate::resources::{
    grid::GridParameters,
    simulation::SimulationParameters,
    particle_types::ParticleTypesConfig,
    food::FoodParameters,
};
use crate::systems::spawning::FoodPositions;

/// Réinitialise les positions et génomes pour une nouvelle époque
pub fn reset_for_new_epoch(
    mut commands: Commands,
    grid: Res<GridParameters>,
    sim_params: Res<SimulationParameters>,
    particle_config: Res<ParticleTypesConfig>,
    food_params: Res<FoodParameters>,
    mut simulations: Query<(&SimulationId, &mut Genotype, &mut Score, &Children), With<Simulation>>,
    mut particles: Query<(&mut Transform, &mut Velocity, &ParticleType), With<Particle>>,
    mut food_query: Query<(&mut Transform, &mut FoodRespawnTimer, &mut Visibility), (With<Food>, Without<Particle>)>,
) {
    // Si c'est l'époque 0, on ne fait rien car les entités viennent d'être créées
    if sim_params.current_epoch == 0 {
        return;
    }

    let mut rng = rand::rng();

    // Générer de nouvelles positions pour les particules
    // On crée une matrice de positions pour garantir que chaque simulation a les mêmes positions initiales
    let particles_per_type = sim_params.particle_count / particle_config.type_count;
    let mut particle_positions = Vec::new();

    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            particle_positions.push((particle_type, random_position_in_grid(&grid, &mut rng)));
        }
    }

    // Réinitialiser chaque simulation
    for (_, mut genotype, mut score, children) in simulations.iter_mut() {
        // TODO: Ici on appliquera l'algorithme génétique
        // Pour l'instant, on génère un nouveau génome aléatoire
        *genotype = Genotype::random(particle_config.type_count);

        // Réinitialiser le score
        *score = Score::default();

        // Réinitialiser les particules de cette simulation
        let mut particle_index = 0;
        for child in children.iter() {
            if let Ok((mut transform, mut velocity, particle_type)) = particles.get_mut(child) {
                // Utiliser la position correspondante de notre liste
                if particle_index < particle_positions.len() {
                    let (expected_type, position) = &particle_positions[particle_index];

                    // Vérifier que le type correspond (normalement toujours vrai)
                    if particle_type.0 == *expected_type {
                        transform.translation = *position;
                        velocity.0 = Vec3::ZERO;
                    }
                }
                particle_index += 1;
            }
        }
    }

    // Générer de nouvelles positions pour la nourriture
    let new_food_positions: Vec<Vec3> = (0..food_params.food_count)
        .map(|_| random_position_in_grid(&grid, &mut rng))
        .collect();

    // Mettre à jour la ressource des positions
    commands.insert_resource(FoodPositions(new_food_positions.clone()));

    // Réinitialiser la nourriture
    for (i, (mut transform, mut respawn_timer, mut visibility)) in food_query.iter_mut().enumerate() {
        if i < new_food_positions.len() {
            transform.translation = new_food_positions[i];

            // Réinitialiser le timer si nécessaire
            if let Some(ref mut timer) = respawn_timer.0 {
                timer.reset();
            }

            // Rendre visible
            *visibility = Visibility::Visible;
        }
    }

    info!("Réinitialisation pour l'époque {} terminée", sim_params.current_epoch);
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