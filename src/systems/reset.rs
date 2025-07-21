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

/// Structure pour stocker un génome avec son score et des statistiques
#[derive(Clone)]
struct ScoredGenome {
    genotype: Genotype,
    score: f32,
    generation: usize,
}

/// Statistiques de l'époque pour le logging
#[derive(Default)]
struct EpochStats {
    best_score: f32,
    worst_score: f32,
    average_score: f32,
    median_score: f32,
    std_deviation: f32,
    improvement: f32,
}

/// Réinitialise les positions et applique l'algorithme génétique amélioré
pub fn reset_for_new_epoch(
    mut commands: Commands,
    grid: Res<GridParameters>,
    sim_params: Res<SimulationParameters>,
    particle_config: Res<ParticleTypesConfig>,
    food_params: Res<FoodParameters>,
    mut simulations: Query<(&SimulationId, &mut Genotype, &mut Score, &Children), With<Simulation>>,
    mut particles: Query<(&mut Transform, &mut Velocity, &ParticleType), With<Particle>>,
    mut food_query: Query<(&mut Transform, &mut FoodRespawnTimer, &mut Visibility), (With<Food>, Without<Particle>)>,
    mut previous_best_score: Local<f32>,
) {
    // Si c'est l'époque 0, on ne fait rien car les entités viennent d'être créées
    if sim_params.current_epoch == 0 {
        return;
    }

    let mut rng = rand::rng();

    // === COLLECTE DES DONNÉES ET STATISTIQUES ===
    let mut scored_genomes: Vec<ScoredGenome> = simulations
        .iter()
        .map(|(_, genotype, score, _)| ScoredGenome {
            genotype: *genotype,
            score: score.get(),
            generation: sim_params.current_epoch,
        })
        .collect();

    // Calculer les statistiques avant le tri
    let stats = calculate_epoch_stats(&scored_genomes, *previous_best_score);

    // Trier par score décroissant
    scored_genomes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Mettre à jour le meilleur score précédent
    *previous_best_score = stats.best_score;

    // === LOGGING DÉTAILLÉ ===
    log_genetic_algorithm_stats(&stats, &sim_params, &scored_genomes);

    // === ALGORITHME GÉNÉTIQUE AMÉLIORÉ ===
    let elite_count = ((sim_params.simulation_count as f32 * sim_params.elite_ratio).ceil() as usize).max(1);
    let mut new_genomes = Vec::with_capacity(sim_params.simulation_count);

    // 1. CONSERVATION DES ÉLITES (inchangé)
    for i in 0..elite_count {
        new_genomes.push(scored_genomes[i].genotype);
    }

    // 2. GÉNÉRATION DE NOUVEAUX INDIVIDUS
    while new_genomes.len() < sim_params.simulation_count {
        let mut new_genotype;

        if rng.random::<f32>() < sim_params.crossover_rate && scored_genomes.len() >= 2 {
            // CROSSOVER AMÉLIORÉ avec sélection pondérée
            let parent1 = weighted_tournament_selection(&scored_genomes, &mut rng);
            let parent2 = weighted_tournament_selection(&scored_genomes, &mut rng);
            new_genotype = improved_crossover(&parent1, &parent2, &mut rng);
        } else {
            // REPRODUCTION ASEXUÉE avec sélection pondérée
            let parent = weighted_tournament_selection(&scored_genomes, &mut rng);
            new_genotype = parent;
        }

        // MUTATION ADAPTATIVE
        let adaptive_mutation_rate = calculate_adaptive_mutation_rate(
            &stats,
            sim_params.mutation_rate,
            sim_params.current_epoch
        );

        improved_mutation(&mut new_genotype, adaptive_mutation_rate, &mut rng);
        new_genomes.push(new_genotype);
    }

    // === RÉINITIALISATION DES SIMULATIONS ===
    reset_simulations_with_new_genomes(
        &mut commands,
        &grid,
        &sim_params,
        &particle_config,
        &food_params,
        new_genomes,
        &mut simulations,
        &mut particles,
        &mut food_query,
        &mut rng,
    );
}

/// Calcule les statistiques de l'époque
fn calculate_epoch_stats(scored_genomes: &[ScoredGenome], previous_best: f32) -> EpochStats {
    if scored_genomes.is_empty() {
        return EpochStats::default();
    }

    let scores: Vec<f32> = scored_genomes.iter().map(|g| g.score).collect();

    let best = scores.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let worst = scores.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let average = scores.iter().sum::<f32>() / scores.len() as f32;

    // Médiane
    let mut sorted_scores = scores.clone();
    sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if sorted_scores.len() % 2 == 0 {
        (sorted_scores[sorted_scores.len() / 2 - 1] + sorted_scores[sorted_scores.len() / 2]) / 2.0
    } else {
        sorted_scores[sorted_scores.len() / 2]
    };

    // Écart-type
    let variance = scores.iter()
        .map(|&x| (x - average).powi(2))
        .sum::<f32>() / scores.len() as f32;
    let std_deviation = variance.sqrt();

    let improvement = best - previous_best;

    EpochStats {
        best_score: best,
        worst_score: worst,
        average_score: average,
        median_score: median,
        std_deviation,
        improvement,
    }
}

/// Logging détaillé des statistiques génétiques
fn log_genetic_algorithm_stats(
    stats: &EpochStats,
    sim_params: &SimulationParameters,
    genomes: &[ScoredGenome],
) {
    info!("=== ALGORITHME GÉNÉTIQUE - ÉPOQUE {} ===", sim_params.current_epoch);
    info!("📊 Statistiques des scores:");
    info!("   • Meilleur: {:.2}", stats.best_score);
    info!("   • Pire: {:.2}", stats.worst_score);
    info!("   • Moyenne: {:.2}", stats.average_score);
    info!("   • Médiane: {:.2}", stats.median_score);
    info!("   • Écart-type: {:.2}", stats.std_deviation);

    if stats.improvement > 0.0 {
        info!("📈 Amélioration: +{:.2} ({}%)",
            stats.improvement,
            (stats.improvement / (stats.best_score - stats.improvement) * 100.0).max(0.0));
    } else if stats.improvement < 0.0 {
        info!("📉 Régression: {:.2}", stats.improvement);
    } else {
        info!("➡️ Stagnation (pas d'amélioration)");
    }

    let elite_count = ((sim_params.simulation_count as f32 * sim_params.elite_ratio).ceil() as usize).max(1);
    info!("🏆 Élites conservées: {} / {}", elite_count, sim_params.simulation_count);

    // Distribution des scores par quartiles
    let mut sorted_scores: Vec<f32> = genomes.iter().map(|g| g.score).collect();
    sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if sorted_scores.len() >= 4 {
        let q1_idx = sorted_scores.len() / 4;
        let q3_idx = 3 * sorted_scores.len() / 4;
        info!("📈 Quartiles: Q1={:.1}, Q3={:.1}",
            sorted_scores[q1_idx],
            sorted_scores[q3_idx.min(sorted_scores.len() - 1)]);
    }
}

/// Sélection par tournoi pondéré (favorise les meilleurs scores)
fn weighted_tournament_selection(population: &[ScoredGenome], rng: &mut impl Rng) -> Genotype {
    const TOURNAMENT_SIZE: usize = 3;

    // Sélection pondérée : plus de chance de sélectionner les meilleurs
    let weights: Vec<f32> = population.iter()
        .enumerate()
        .map(|(i, _)| 1.0 / (1.0 + i as f32 * 0.1)) // Poids décroissant selon le rang
        .collect();

    let mut tournament_indices = Vec::new();
    for _ in 0..TOURNAMENT_SIZE.min(population.len()) {
        let total_weight: f32 = weights.iter().sum();
        let mut random = rng.random::<f32>() * total_weight;

        for (i, &weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                tournament_indices.push(i);
                break;
            }
        }
    }

    // Retourner le meilleur du tournoi
    tournament_indices.into_iter()
        .map(|i| &population[i])
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
        .map(|g| g.genotype)
        .unwrap_or(population[0].genotype)
}

/// Crossover amélioré avec zones de préservation
fn improved_crossover(parent1: &Genotype, parent2: &Genotype, rng: &mut impl Rng) -> Genotype {
    let mut new_genome = 0u64;
    let mut new_food_genome = 0u16;

    // Crossover par blocs pour préserver les interactions liées
    let interactions = parent1.type_count * parent1.type_count;
    let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8);

    // Crossover par interaction plutôt que par bit
    for interaction in 0..interactions {
        let bit_start = interaction * bits_per_interaction;
        if bit_start + bits_per_interaction <= 64 {
            let mask = ((1u64 << bits_per_interaction) - 1) << bit_start;

            if rng.random_bool(0.5) {
                new_genome |= parent1.genome & mask;
            } else {
                new_genome |= parent2.genome & mask;
            }
        }
    }

    // Crossover pour le génome de nourriture (par type)
    let bits_per_type = (16 / parent1.type_count.max(1)).max(3).min(8);
    for type_idx in 0..parent1.type_count {
        let bit_start = type_idx * bits_per_type;
        if bit_start + bits_per_type <= 16 {
            let mask = ((1u16 << bits_per_type) - 1) << bit_start;

            if rng.random_bool(0.5) {
                new_food_genome |= parent1.food_force_genome & mask;
            } else {
                new_food_genome |= parent2.food_force_genome & mask;
            }
        }
    }

    Genotype::new(new_genome, parent1.type_count, new_food_genome)
}

/// Mutation adaptative basée sur la diversité de la population
fn calculate_adaptive_mutation_rate(
    stats: &EpochStats,
    base_rate: f32,
    epoch: usize
) -> f32 {
    let diversity_factor = if stats.std_deviation < 5.0 {
        // Peu de diversité -> augmenter les mutations
        2.0
    } else if stats.std_deviation > 20.0 {
        // Beaucoup de diversité -> réduire les mutations
        0.5
    } else {
        1.0
    };

    let stagnation_factor = if stats.improvement <= 0.0 {
        // Stagnation -> augmenter les mutations
        1.5
    } else {
        1.0
    };

    let early_exploration = if epoch < 10 {
        // Exploration initiale plus importante
        1.5
    } else {
        1.0
    };

    (base_rate * diversity_factor * stagnation_factor * early_exploration).min(0.5)
}

/// Mutation améliorée avec plusieurs stratégies
fn improved_mutation(genotype: &mut Genotype, mutation_rate: f32, rng: &mut impl Rng) {
    let interactions = genotype.type_count * genotype.type_count;
    let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8);

    // Mutation des forces particule-particule
    for interaction in 0..interactions {
        if rng.random::<f32>() < mutation_rate {
            let bit_start = interaction * bits_per_interaction;
            if bit_start + bits_per_interaction <= 64 {

                // 3 types de mutations possibles :
                match rng.random_range(0..3) {
                    0 => {
                        // Mutation légère : inverser 1 bit
                        let bit_offset = rng.random_range(0..bits_per_interaction);
                        let bit_position = bit_start + bit_offset;
                        genotype.genome ^= 1u64 << bit_position;
                    },
                    1 => {
                        // Mutation moyenne : inverser 2-3 bits
                        let bits_to_flip = rng.random_range(2..=3.min(bits_per_interaction));
                        for _ in 0..bits_to_flip {
                            let bit_offset = rng.random_range(0..bits_per_interaction);
                            let bit_position = bit_start + bit_offset;
                            genotype.genome ^= 1u64 << bit_position;
                        }
                    },
                    2 => {
                        // Mutation forte : remplacer toute l'interaction
                        let mask = ((1u64 << bits_per_interaction) - 1) << bit_start;
                        let new_value = (rng.random::<u64>() & ((1u64 << bits_per_interaction) - 1)) << bit_start;
                        genotype.genome = (genotype.genome & !mask) | new_value;
                    },
                    _ => unreachable!(),
                }
            }
        }
    }

    // Mutation des forces de nourriture (avec taux réduit)
    if rng.random::<f32>() < mutation_rate * 0.3 {
        let bits_per_type = (16 / genotype.type_count.max(1)).max(3).min(8);
        let type_to_mutate = rng.random_range(0..genotype.type_count);
        let bit_start = type_to_mutate * bits_per_type;

        if bit_start + bits_per_type <= 16 {
            let bit_offset = rng.random_range(0..bits_per_type);
            let bit_position = bit_start + bit_offset;
            genotype.food_force_genome ^= 1u16 << bit_position;
        }
    }
}

/// Réinitialise les simulations avec les nouveaux génomes
fn reset_simulations_with_new_genomes(
    commands: &mut Commands,
    grid: &GridParameters,
    sim_params: &SimulationParameters,
    particle_config: &ParticleTypesConfig,
    food_params: &FoodParameters,
    new_genomes: Vec<Genotype>,
    simulations: &mut Query<(&SimulationId, &mut Genotype, &mut Score, &Children), With<Simulation>>,
    particles: &mut Query<(&mut Transform, &mut Velocity, &ParticleType), With<Particle>>,
    food_query: &mut Query<(&mut Transform, &mut FoodRespawnTimer, &mut Visibility), (With<Food>, Without<Particle>)>,
    rng: &mut impl Rng,
) {
    // Générer de nouvelles positions pour les particules
    let particles_per_type = (sim_params.particle_count + particle_config.type_count - 1) / particle_config.type_count;
    let mut particle_positions = Vec::new();

    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            particle_positions.push((particle_type, random_position_in_grid(grid, rng)));
        }
    }

    // Réinitialiser chaque simulation avec son nouveau génome
    let mut sim_index = 0;
    for (_, mut genotype, mut score, children) in simulations.iter_mut() {
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
                if particle_index < particle_positions.len() {
                    let (expected_type, position) = &particle_positions[particle_index];
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

    // Réinitialiser la nourriture
    let new_food_positions: Vec<Vec3> = (0..food_params.food_count)
        .map(|_| random_position_in_grid(grid, rng))
        .collect();

    commands.insert_resource(FoodPositions(new_food_positions.clone()));

    for (i, (mut transform, mut respawn_timer, mut visibility)) in food_query.iter_mut().enumerate() {
        if i < new_food_positions.len() {
            transform.translation = new_food_positions[i];
            if let Some(ref mut timer) = respawn_timer.0 {
                timer.reset();
            }
            *visibility = Visibility::Visible;
        }
    }

    info!("✅ Réinitialisation pour l'époque {} terminée avec {} génomes",
        sim_params.current_epoch, new_genomes.len());
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