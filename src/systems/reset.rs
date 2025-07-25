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
    coherence: f32,
    fitness_trend: f32,
}

#[derive(Default)]
pub struct EpochStats {
    best_score: f32,
    worst_score: f32,
    average_score: f32,
    median_score: f32,
    std_deviation: f32,
    improvement: f32,
    average_coherence: f32,
    diversity_index: f32,
}

#[derive(Default)]
struct GeneticConfig {
    elite_ratio: f32,           // 30% au lieu de 10%
    mutation_rate: f32,         // Taux de base
    crossover_rate: f32,        // 25% au lieu de 70%
    coherence_threshold: f32,   // Seuil minimum de cohérence
    diversity_pressure: f32,    // Pression pour maintenir la diversité
}

impl GeneticConfig {
    fn optimized() -> Self {
        Self {
            elite_ratio: 0.3,           // Plus d'élitisme
            mutation_rate: 0.15,        // Mutation modérée
            crossover_rate: 0.25,       // Moins de crossover
            coherence_threshold: 0.3,   // Rejeter les génomes trop incohérents
            diversity_pressure: 0.1,    // Favoriser la diversité
        }
    }
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
    mut previous_stats: Local<Option<EpochStats>>,
) {
    if sim_params.current_epoch == 0 {
        return;
    }

    let mut rng = rand::rng();
    let genetic_config = GeneticConfig::optimized();

    // Collecter et évaluer les génomes avec leurs métriques étendues
    let mut scored_genomes = collect_and_evaluate_genomes(&simulations, sim_params.current_epoch);

    let current_stats = calculate_epoch_stats(&scored_genomes, previous_stats.as_ref());
    log_advanced_genetic_stats(&current_stats, &sim_params, &scored_genomes);

    // Trier par score combiné (performance + cohérence + diversité)
    scored_genomes.sort_by(|a, b| {
        let score_a = calculate_combined_fitness(a, &current_stats, &genetic_config);
        let score_b = calculate_combined_fitness(b, &current_stats, &genetic_config);
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Générer la nouvelle population avec l'algorithme amélioré
    let new_genomes = generate_improved_population(
        &scored_genomes,
        sim_params.simulation_count,
        &genetic_config,
        &current_stats,
        &mut rng
    );

    // Appliquer les nouveaux génomes
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

    *previous_stats = Some(current_stats);
}

fn collect_and_evaluate_genomes(
    simulations: &Query<(&SimulationId, &mut Genotype, &mut Score, &Children), With<Simulation>>,
    current_epoch: usize,
) -> Vec<ScoredGenome> {
    simulations
        .iter()
        .map(|(_, genotype, score, _)| {
            let mut genotype_copy = genotype.clone();
            genotype_copy.update_fitness_history(score.get());

            ScoredGenome {
                coherence: genotype_copy.strategy_coherence,
                fitness_trend: genotype_copy.get_fitness_trend(),
                genotype: genotype_copy,
                score: score.get(),
                generation: current_epoch,
            }
        })
        .collect()
}

fn calculate_combined_fitness(
    genome: &ScoredGenome,
    stats: &EpochStats,
    config: &GeneticConfig
) -> f32 {
    let normalized_score = if stats.best_score > stats.worst_score {
        (genome.score - stats.worst_score) / (stats.best_score - stats.worst_score)
    } else {
        0.5
    };

    let coherence_bonus = if genome.coherence > config.coherence_threshold {
        (genome.coherence - config.coherence_threshold) / (1.0 - config.coherence_threshold)
    } else {
        0.0
    };

    let trend_bonus = genome.fitness_trend.max(0.0) / 10.0; // Normaliser la tendance

    // Score final pondéré
    normalized_score * 0.6 + coherence_bonus * 0.3 + trend_bonus * 0.1
}

fn calculate_epoch_stats(scored_genomes: &[ScoredGenome], previous: Option<&EpochStats>) -> EpochStats {
    if scored_genomes.is_empty() {
        return EpochStats::default();
    }

    let scores: Vec<f32> = scored_genomes.iter().map(|g| g.score).collect();
    let coherences: Vec<f32> = scored_genomes.iter().map(|g| g.coherence).collect();

    let best = scores.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let worst = scores.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let average = scores.iter().sum::<f32>() / scores.len() as f32;
    let average_coherence = coherences.iter().sum::<f32>() / coherences.len() as f32;

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

    let improvement = if let Some(prev) = previous {
        best - prev.best_score
    } else {
        0.0
    };

    // Calculer un index de diversité génétique
    let diversity_index = calculate_genetic_diversity(scored_genomes);

    EpochStats {
        best_score: best,
        worst_score: worst,
        average_score: average,
        median_score: median,
        std_deviation,
        improvement,
        average_coherence,
        diversity_index,
    }
}

fn calculate_genetic_diversity(genomes: &[ScoredGenome]) -> f32 {
    if genomes.len() < 2 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let mut comparisons = 0;

    // Calculer la distance génétique moyenne entre tous les génomes
    for i in 0..genomes.len() {
        for j in i+1..genomes.len() {
            let distance = calculate_genetic_distance(&genomes[i].genotype, &genomes[j].genotype);
            total_distance += distance;
            comparisons += 1;
        }
    }

    if comparisons > 0 {
        total_distance / comparisons as f32
    } else {
        0.0
    }
}

fn calculate_genetic_distance(genome1: &Genotype, genome2: &Genotype) -> f32 {
    let mut distance = 0.0;

    // Distance dans la matrice des forces
    for i in 0..genome1.force_matrix.len() {
        let diff = genome1.force_matrix[i] - genome2.force_matrix[i];
        distance += diff * diff;
    }

    // Distance dans les forces de nourriture
    for i in 0..genome1.food_forces.len() {
        let diff = genome1.food_forces[i] - genome2.food_forces[i];
        distance += diff * diff;
    }

    distance.sqrt()
}

fn generate_improved_population(
    scored_genomes: &[ScoredGenome],
    target_size: usize,
    config: &GeneticConfig,
    stats: &EpochStats,
    rng: &mut impl Rng,
) -> Vec<Genotype> {
    let mut new_population = Vec::with_capacity(target_size);

    // 1. ÉLITISME ÉTENDU - Conserver les meilleurs avec leurs variations
    let elite_count = ((target_size as f32 * config.elite_ratio).ceil() as usize).max(1);

    info!("🏆 Conservation de {} élites sur {} individus", elite_count, target_size);

    for i in 0..elite_count.min(scored_genomes.len()) {
        let mut elite = scored_genomes[i].genotype.clone();

        // Appliquer une très légère mutation aux élites pour éviter la stagnation
        let light_mutation_rate = config.mutation_rate * 0.1;
        elite.mutate(light_mutation_rate, rng);

        new_population.push(elite);
    }

    // 2. REPRODUCTION SÉLECTIVE avec validation
    while new_population.len() < target_size {
        let mut offspring = if rng.random::<f32>() < config.crossover_rate && scored_genomes.len() >= 2 {
            // Crossover avec sélection basée sur la performance ET la cohérence
            let parent1 = &enhanced_tournament_selection(scored_genomes, config, rng);
            let parent2 = &enhanced_tournament_selection(scored_genomes, config, rng);

            let mut child = parent1.crossover(parent2, rng);

            // Validation post-crossover
            let max_attempts = 3;
            for _ in 0..max_attempts {
                if child.strategy_coherence >= config.coherence_threshold {
                    break;
                }
                // Réessayer le crossover si incohérent
                child = parent1.crossover(parent2, rng);
            }

            child
        } else {
            // Reproduction asexuée avec mutation
            let parent = enhanced_tournament_selection(scored_genomes, config, rng);
            parent.clone()
        };

        // Mutation adaptative
        let adaptive_mutation_rate = calculate_adaptive_mutation_rate(
            stats,
            config.mutation_rate,
            offspring.strategy_coherence
        );

        offspring.mutate(adaptive_mutation_rate, rng);

        // Validation finale avant ajout
        if offspring.strategy_coherence >= config.coherence_threshold || new_population.len() >= target_size - 2 {
            new_population.push(offspring);
        }
        // Si validation échoue, créer un individu aléatoire cohérent
        else if new_population.len() < target_size - 1 {
            let random_genome = Genotype::random(scored_genomes[0].genotype.type_count);
            new_population.push(random_genome);
        }
    }

    // 3. INJECTION DE DIVERSITÉ si nécessaire
    if stats.diversity_index < 0.5 && new_population.len() > 2 {
        let diversity_injection = (target_size as f32 * 0.1) as usize;
        info!("🌱 Injection de {} individus pour maintenir la diversité", diversity_injection);

        for _ in 0..diversity_injection {
            if new_population.len() > diversity_injection {
                let random_genome = Genotype::random(scored_genomes[0].genotype.type_count);
                let replace_idx = rng.random_range(elite_count..new_population.len());
                new_population[replace_idx] = random_genome;
            }
        }
    }

    new_population.truncate(target_size);
    new_population
}

fn enhanced_tournament_selection(
    population: &[ScoredGenome],
    config: &GeneticConfig,
    rng: &mut impl Rng,
) -> Genotype {
    const TOURNAMENT_SIZE: usize = 4; // Tournoi plus grand

    let mut tournament: Vec<&ScoredGenome> = Vec::new();

    // Sélection pondérée pour le tournoi
    for _ in 0..TOURNAMENT_SIZE.min(population.len()) {
        // Favoriser les individus avec haut score ET haute cohérence
        let weights: Vec<f32> = population.iter()
            .enumerate()
            .map(|(i, genome)| {
                let rank_weight = 1.0 / (1.0 + i as f32 * 0.1);
                let coherence_weight = (genome.coherence - config.coherence_threshold).max(0.0) + 0.1;
                rank_weight * coherence_weight
            })
            .collect();

        let total_weight: f32 = weights.iter().sum();
        let mut random = rng.random::<f32>() * total_weight;

        for (i, &weight) in weights.iter().enumerate() {
            random -= weight;
            if random <= 0.0 {
                tournament.push(&population[i]);
                break;
            }
        }
    }

    // Choisir le meilleur du tournoi selon le score combiné
    tournament.into_iter()
        .max_by(|a, b| {
            let stats = EpochStats::default(); // Stats simplifiées pour la sélection
            let score_a = calculate_combined_fitness(a, &stats, config);
            let score_b = calculate_combined_fitness(b, &stats, config);
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|g| g.genotype.clone())
        .unwrap_or(population[0].genotype.clone())
}

fn calculate_adaptive_mutation_rate(
    stats: &EpochStats,
    base_rate: f32,
    genome_coherence: f32,
) -> f32 {
    // Facteur de diversité
    let diversity_factor = if stats.diversity_index < 0.3 {
        2.0 // Augmenter la mutation si faible diversité
    } else if stats.diversity_index > 0.8 {
        0.7 // Réduire si trop de diversité
    } else {
        1.0
    };

    // Facteur de stagnation
    let stagnation_factor = if stats.improvement <= 0.0 {
        1.5 // Plus de mutation si stagnation
    } else if stats.improvement > 10.0 {
        0.8 // Moins de mutation si bonne progression
    } else {
        1.0
    };

    // Facteur de cohérence individuelle
    let coherence_factor = if genome_coherence > 0.8 {
        0.5 // Mutations très douces pour les génomes cohérents
    } else if genome_coherence < 0.4 {
        1.8 // Plus de mutation pour améliorer la cohérence
    } else {
        1.0
    };

    (base_rate * diversity_factor * stagnation_factor * coherence_factor).clamp(0.01, 0.5)
}

fn log_advanced_genetic_stats(
    stats: &EpochStats,
    sim_params: &SimulationParameters,
    genomes: &[ScoredGenome],
) {
    info!("=== ALGORITHME GÉNÉTIQUE AVANCÉ - ÉPOQUE {} ===", sim_params.current_epoch);

    info!("📊 Statistiques de performance:");
    info!("   • Meilleur: {:.2}", stats.best_score);
    info!("   • Moyenne: {:.2} (±{:.2})", stats.average_score, stats.std_deviation);
    info!("   • Médiane: {:.2}", stats.median_score);

    if stats.improvement > 0.0 {
        info!("📈 Amélioration: +{:.2} ({:.1}%)",
            stats.improvement,
            (stats.improvement / (stats.best_score - stats.improvement) * 100.0).max(0.0));
    } else if stats.improvement < 0.0 {
        info!("📉 Régression: {:.2}", stats.improvement);
    } else {
        info!("➡️ Stagnation");
    }

    info!("🧬 Métriques génétiques:");
    info!("   • Cohérence moyenne: {:.3}", stats.average_coherence);
    info!("   • Index de diversité: {:.3}", stats.diversity_index);

    let high_coherence_count = genomes.iter().filter(|g| g.coherence > 0.7).count();
    info!("   • Génomes très cohérents: {}/{}", high_coherence_count, genomes.len());

    // Analyser les tendances
    let improving_genomes = genomes.iter().filter(|g| g.fitness_trend > 0.0).count();
    info!("   • Génomes en progression: {}/{}", improving_genomes, genomes.len());

    // Prédiction de performance
    let genetic_config = GeneticConfig::optimized();
    let predicted_improvement = if stats.diversity_index > 0.5 && stats.average_coherence > 0.6 {
        "Forte"
    } else if stats.diversity_index > 0.3 || stats.average_coherence > 0.4 {
        "Modérée"
    } else {
        "Faible"
    };

    info!("🔮 Potentiel d'amélioration prédit: {}", predicted_improvement);
    info!("⚙️ Configuration: {:.0}% élites, {:.0}% crossover, mutation adaptative",
        genetic_config.elite_ratio * 100.0, genetic_config.crossover_rate * 100.0);
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

    // Générer nouvelles positions pour réinitialiser l'environnement
    for particle_type in 0..particle_config.type_count {
        for _ in 0..particles_per_type {
            particle_positions.push((particle_type, random_position_in_grid(grid, rng)));
        }
    }

    // Appliquer les nouveaux génomes aux simulations
    let mut sim_index = 0;
    for (sim_id, mut genotype, mut score, children) in simulations.iter_mut() {
        if sim_index < new_genomes.len() {
            *genotype = new_genomes[sim_index].clone();
            info!("Simulation {} - Cohérence: {:.3}", sim_id.0, genotype.strategy_coherence);
        }

        *score = Score::default();

        // Repositionner les particules
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

    // Repositionner la nourriture
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

    let avg_coherence = new_genomes.iter().map(|g| g.strategy_coherence).sum::<f32>() / new_genomes.len() as f32;
    info!("✅ Époque {} initialisée - {} génomes (cohérence moyenne: {:.3})",
        sim_params.current_epoch, new_genomes.len(), avg_coherence);
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