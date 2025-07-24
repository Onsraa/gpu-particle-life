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

#[derive(Clone)]
struct ScoredGenome {
    genotype: Genotype,
    score: f32,
    generation: usize,
}

#[derive(Default)]
struct EpochStats {
    best_score: f32,
    worst_score: f32,
    average_score: f32,
    median_score: f32,
    std_deviation: f32,
    improvement: f32,
}

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
    if sim_params.current_epoch == 0 {
        return;
    }

    let mut rng = rand::rng();

    let mut scored_genomes: Vec<ScoredGenome> = simulations
        .iter()
        .map(|(_, genotype, score, _)| ScoredGenome {
            genotype: genotype.clone(),
            score: score.get(),
            generation: sim_params.current_epoch,
        })
        .collect();

    let stats = calculate_epoch_stats(&scored_genomes, *previous_best_score);
    scored_genomes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    *previous_best_score = stats.best_score;

    log_genetic_algorithm_stats(&stats, &sim_params, &scored_genomes);

    let elite_count = ((sim_params.simulation_count as f32 * sim_params.elite_ratio).ceil() as usize).max(1);
    let mut new_genomes = Vec::with_capacity(sim_params.simulation_count);

    // Conservation des élites
    for i in 0..elite_count {
        new_genomes.push(scored_genomes[i].genotype.clone());
    }

    // Génération de nouveaux individus
    while new_genomes.len() < sim_params.simulation_count {
        let mut new_genotype;

        if rng.random::<f32>() < sim_params.crossover_rate && scored_genomes.len() >= 2 {
            let parent1 = &weighted_tournament_selection(&scored_genomes, &mut rng);
            let parent2 = &weighted_tournament_selection(&scored_genomes, &mut rng);
            new_genotype = improved_crossover(parent1, parent2, &mut rng);
        } else {
            let parent = weighted_tournament_selection(&scored_genomes, &mut rng);
            new_genotype = parent;
        }

        let adaptive_mutation_rate = calculate_adaptive_mutation_rate(
            &stats,
            sim_params.mutation_rate,
            sim_params.current_epoch
        );

        new_genotype.mutate(adaptive_mutation_rate, &mut rng);
        new_genomes.push(new_genotype);
    }

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

fn calculate_epoch_stats(scored_genomes: &[ScoredGenome], previous_best: f32) -> EpochStats {
    if scored_genomes.is_empty() {
        return EpochStats::default();
    }

    let scores: Vec<f32> = scored_genomes.iter().map(|g| g.score).collect();

    let best = scores.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let worst = scores.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let average = scores.iter().sum::<f32>() / scores.len() as f32;

    let mut sorted_scores = scores.clone();
    sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if sorted_scores.len() % 2 == 0 {
        (sorted_scores[sorted_scores.len() / 2 - 1] + sorted_scores[sorted_scores.len() / 2]) / 2.0
    } else {
        sorted_scores[sorted_scores.len() / 2]
    };

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

fn weighted_tournament_selection(population: &[ScoredGenome], rng: &mut impl Rng) -> Genotype {
    const TOURNAMENT_SIZE: usize = 3;

    let weights: Vec<f32> = population.iter()
        .enumerate()
        .map(|(i, _)| 1.0 / (1.0 + i as f32 * 0.1))
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

    tournament_indices.into_iter()
        .map(|i| &population[i])
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
        .map(|g| g.genotype.clone())
        .unwrap_or(population[0].genotype.clone())
}

fn improved_crossover(parent1: &Genotype, parent2: &Genotype, rng: &mut impl Rng) -> Genotype {
    let mut new_genotype = Genotype::new(parent1.type_count);

    // Crossover des forces particule-particule
    for i in 0..parent1.force_matrix.len() {
        if rng.random_bool(0.5) {
            new_genotype.force_matrix[i] = parent1.force_matrix[i];
        } else {
            new_genotype.force_matrix[i] = parent2.force_matrix[i];
        }
    }

    // Crossover des forces de nourriture
    for i in 0..parent1.food_forces.len() {
        if rng.random_bool(0.5) {
            new_genotype.food_forces[i] = parent1.food_forces[i];
        } else {
            new_genotype.food_forces[i] = parent2.food_forces[i];
        }
    }

    new_genotype
}

fn calculate_adaptive_mutation_rate(
    stats: &EpochStats,
    base_rate: f32,
    epoch: usize
) -> f32 {
    let diversity_factor = if stats.std_deviation < 5.0 {
        2.0
    } else if stats.std_deviation > 20.0 {
        0.5
    } else {
        1.0
    };

    let stagnation_factor = if stats.improvement <= 0.0 {
        1.5
    } else {
        1.0
    };

    let early_exploration = if epoch < 10 {
        1.5
    } else {
        1.0
    };

    (base_rate * diversity_factor * stagnation_factor * early_exploration).min(0.5)
}

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
    let particles_per_type = (sim_params.particle_count + particle_config.type_count - 1) / particle_config.type_count;
    let mut particle_positions = Vec::new();

    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            particle_positions.push((particle_type, random_position_in_grid(grid, rng)));
        }
    }

    let mut sim_index = 0;
    for (_, mut genotype, mut score, children) in simulations.iter_mut() {
        if sim_index < new_genomes.len() {
            *genotype = new_genomes[sim_index].clone();
        }

        *score = Score::default();

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