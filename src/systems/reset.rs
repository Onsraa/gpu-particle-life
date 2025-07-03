use bevy::prelude::*;
use rand::prelude::IndexedRandom;
use rand::Rng;
use rand::seq::SliceRandom;

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

/// Structure pour stocker un génome avec son score pour le tri
#[derive(Clone)]
struct ScoredGenome {
    genotype: Genotype,
    score: f32,
}

/// Réinitialise les positions et applique l'algorithme génétique pour une nouvelle époque
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

    // === ALGORITHME GÉNÉTIQUE ===
    // Collecter tous les génomes avec leurs scores
    let mut scored_genomes: Vec<ScoredGenome> = simulations
        .iter()
        .map(|(_, genotype, score, _)| ScoredGenome {
            genotype: *genotype,
            score: score.get(),
        })
        .collect();

    // Trier par score décroissant
    scored_genomes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Calculer le nombre d'élites à garder
    let elite_count = ((sim_params.simulation_count as f32 * sim_params.elite_ratio).ceil() as usize).max(1);

    info!("=== Algorithme Génétique - Époque {} ===", sim_params.current_epoch);
    info!("Meilleur score: {:.1}", scored_genomes.first().map(|g| g.score).unwrap_or(0.0));
    info!("Score moyen: {:.1}", scored_genomes.iter().map(|g| g.score).sum::<f32>() / scored_genomes.len() as f32);
    info!("Élites conservées: {}", elite_count);

    // Créer la nouvelle population
    let mut new_genomes = Vec::with_capacity(sim_params.simulation_count);

    // 1. Conserver les élites
    for i in 0..elite_count {
        new_genomes.push(scored_genomes[i].genotype);
    }

    // 2. Générer le reste de la population
    while new_genomes.len() < sim_params.simulation_count {
        let mut new_genotype;

        if rng.random::<f32>() < sim_params.crossover_rate && scored_genomes.len() >= 2 {
            // Crossover - Sélection par tournoi
            let parent1 = tournament_selection(&scored_genomes, &mut rng);
            let parent2 = tournament_selection(&scored_genomes, &mut rng);
            new_genotype = parent1.crossover(&parent2, &mut rng);
        } else {
            // Reproduction asexuée - Cloner un parent sélectionné
            let parent = tournament_selection(&scored_genomes, &mut rng);
            new_genotype = parent;
        }

        // Appliquer la mutation
        new_genotype.mutate(sim_params.mutation_rate, &mut rng);

        new_genomes.push(new_genotype);
    }

    // === RÉINITIALISATION DES SIMULATIONS ===
    // Générer de nouvelles positions pour les particules
    let particles_per_type = (sim_params.particle_count + particle_config.type_count - 1) / particle_config.type_count;
    let mut particle_positions = Vec::new();

    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            particle_positions.push((particle_type, random_position_in_grid(&grid, &mut rng)));
        }
    }

    // Réinitialiser chaque simulation avec son nouveau génome
    let mut sim_index = 0;
    for (sim_id, mut genotype, mut score, children) in simulations.iter_mut() {
        // Appliquer le nouveau génome
        if sim_index < new_genomes.len() {
            *genotype = new_genomes[sim_index];
        }

        // Réinitialiser le score
        *score = Score::default();

        // Réinitialiser les particules de cette simulation
        let mut particle_index = 0;
        for child in children.iter() {
            if let Ok((mut transform, mut velocity, particle_type)) = particles.get_mut(child) {
                // Utiliser la position correspondante de notre liste
                if particle_index < particle_positions.len() {
                    let (expected_type, position) = &particle_positions[particle_index];

                    // Vérifier que le type correspond
                    if particle_type.0 == *expected_type {
                        transform.translation = *position;
                        velocity.0 = Vec3::ZERO;
                    }
                }
                particle_index += 1;
            }
        }

        sim_index += 1;
    }

    // === RÉINITIALISATION DE LA NOURRITURE ===
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

/// Sélection par tournoi pour l'algorithme génétique
fn tournament_selection(population: &[ScoredGenome], rng: &mut impl Rng) -> Genotype {
    const TOURNAMENT_SIZE: usize = 3;

    let mut tournament: Vec<&ScoredGenome> = population
        .choose_multiple(rng, TOURNAMENT_SIZE.min(population.len()))
        .collect();

    tournament.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    tournament.first().unwrap().genotype
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