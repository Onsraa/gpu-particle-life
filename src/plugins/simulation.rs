use bevy::prelude::*;

use crate::states::app::AppState;
use crate::states::simulation::SimulationState;
use crate::systems::debug_particles::debug_particle_movement;
use crate::systems::{
    collision::detect_food_collision,
    debug::debug_scores,
    movement::physics_simulation_system,
    population_save::{
        AvailablePopulations, PopulationSaveEvents, load_available_populations,
        process_save_requests, export_population_statistics,
    },
    reset::reset_for_new_epoch,
    spawning::{EntitiesSpawned, spawn_food, spawn_simulations_with_particles},
};
use crate::plugins::compute::ComputeEnabled;
use crate::components::{simulation::Simulation, genotype::Genotype, score::Score};
use crate::resources::simulation::SimulationParameters;

pub struct SimulationPlugin;

/// Ressource pour les métriques d'évolution en temps réel
#[derive(Resource, Default)]
pub struct EvolutionMetrics {
    pub epoch_start_time: Option<std::time::Instant>,
    pub generation_scores: Vec<f32>,
    pub coherence_history: Vec<f32>,
    pub diversity_history: Vec<f32>,
    pub best_score_history: Vec<f32>,
    pub improvement_rate: f32,
    pub stagnation_counter: usize,
    pub last_export_epoch: usize,
}

/// Ressource pour configurer l'auto-export des statistiques
#[derive(Resource)]
pub struct AutoExportConfig {
    pub enabled: bool,
    pub export_interval: usize, // Exporter toutes les N époques
    pub export_on_improvement: bool,
    pub min_improvement_threshold: f32,
}

impl Default for AutoExportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            export_interval: 10, // Exporter toutes les 10 époques
            export_on_improvement: true,
            min_improvement_threshold: 5.0, // Amélioration minimale de 5 points
        }
    }
}

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<SimulationState>()
            .init_resource::<EntitiesSpawned>()
            .init_resource::<PopulationSaveEvents>()
            .init_resource::<AvailablePopulations>()
            .init_resource::<EvolutionMetrics>()
            .init_resource::<AutoExportConfig>()
            .add_systems(Startup, load_available_populations)
            .add_systems(
                OnEnter(AppState::Simulation),
                |mut next_state: ResMut<NextState<SimulationState>>| {
                    next_state.set(SimulationState::Starting);
                },
            )
            .add_systems(
                OnEnter(SimulationState::Starting),
                (
                    spawn_simulations_with_particles,
                    spawn_food,
                    reset_for_new_epoch,
                    initialize_evolution_metrics,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                transition_to_running
                    .run_if(in_state(SimulationState::Starting))
                    .run_if(in_state(AppState::Simulation)),
            )
            // Système physique CPU seulement quand GPU désactivé
            .add_systems(
                Update,
                physics_simulation_system
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_disabled),
            )
            // Systèmes généraux avec métriques améliorées
            .add_systems(
                Update,
                (
                    detect_food_collision,
                    enhanced_epoch_management,
                    update_evolution_metrics,
                    debug_scores,
                    debug_particle_movement,
                    process_save_requests,
                    auto_export_statistics,
                )
                    .chain() // Utiliser chain() pour forcer l'ordre séquentiel
                    .run_if(in_state(SimulationState::Running))
                    .run_if(in_state(AppState::Simulation)),
            )
            .add_systems(
                Update,
                handle_pause_input.run_if(in_state(AppState::Simulation)),
            )
            .add_systems(OnExit(AppState::Simulation), cleanup_all_enhanced);
    }
}

fn compute_disabled(compute: Res<ComputeEnabled>) -> bool {
    !compute.0
}

fn transition_to_running(
    mut next_state: ResMut<NextState<SimulationState>>,
    compute_enabled: Res<ComputeEnabled>,
    mut evolution_metrics: ResMut<EvolutionMetrics>,
) {
    info!("🚀 Démarrage de la simulation - GPU compute: {}", compute_enabled.0);

    // Initialiser le timer d'époque
    evolution_metrics.epoch_start_time = Some(std::time::Instant::now());

    next_state.set(SimulationState::Running);
}

fn initialize_evolution_metrics(
    mut evolution_metrics: ResMut<EvolutionMetrics>,
    sim_params: Res<SimulationParameters>,
) {
    // Réinitialiser les métriques pour une nouvelle simulation
    evolution_metrics.epoch_start_time = Some(std::time::Instant::now());
    evolution_metrics.generation_scores.clear();
    evolution_metrics.coherence_history.clear();
    evolution_metrics.diversity_history.clear();
    evolution_metrics.best_score_history.clear();
    evolution_metrics.improvement_rate = 0.0;
    evolution_metrics.stagnation_counter = 0;
    evolution_metrics.last_export_epoch = 0;

    info!("📊 Métriques d'évolution initialisées pour {} simulations",
        sim_params.simulation_count);
}

/// Gestion avancée des fins d'époque avec métriques enrichies
fn enhanced_epoch_management(
    mut sim_params: ResMut<SimulationParameters>,
    mut next_state: ResMut<NextState<SimulationState>>,
    mut evolution_metrics: ResMut<EvolutionMetrics>,
    time: Res<Time>,
    simulations: Query<(&Genotype, &Score), With<Simulation>>,
) {
    sim_params.tick(time.delta());

    if sim_params.is_epoch_finished() {
        // Calculer les métriques de fin d'époque
        let current_metrics = calculate_epoch_metrics(&simulations);

        // Mettre à jour l'historique
        evolution_metrics.generation_scores.push(current_metrics.average_score);
        evolution_metrics.coherence_history.push(current_metrics.average_coherence);
        evolution_metrics.diversity_history.push(current_metrics.diversity_index);
        evolution_metrics.best_score_history.push(current_metrics.best_score);

        // Calculer le taux d'amélioration
        if evolution_metrics.best_score_history.len() >= 2 {
            let recent_scores = &evolution_metrics.best_score_history;
            let current_best = recent_scores[recent_scores.len() - 1];
            let previous_best = recent_scores[recent_scores.len() - 2];
            evolution_metrics.improvement_rate = current_best - previous_best;

            // Détecter la stagnation
            if evolution_metrics.improvement_rate <= 0.1 {
                evolution_metrics.stagnation_counter += 1;
            } else {
                evolution_metrics.stagnation_counter = 0;
            }
        }

        // Calculer la durée de l'époque
        let epoch_duration = evolution_metrics.epoch_start_time
            .map(|start| start.elapsed().as_secs_f32())
            .unwrap_or(0.0);

        info!("⏱️  Époque {} terminée en {:.1}s!", sim_params.current_epoch, epoch_duration);
        log_enhanced_epoch_summary(&current_metrics, &evolution_metrics, &sim_params);

        // Prédictions pour la prochaine époque
        log_evolution_predictions(&evolution_metrics);

        // Démarrer la nouvelle époque
        sim_params.start_new_epoch();
        evolution_metrics.epoch_start_time = Some(std::time::Instant::now());
        next_state.set(SimulationState::Starting);
    }
}

#[derive(Debug)]
struct SimulationEpochMetrics {
    best_score: f32,
    average_score: f32,
    average_coherence: f32,
    diversity_index: f32,
    high_coherence_count: usize,
    total_simulations: usize,
}

fn calculate_epoch_metrics(simulations: &Query<(&Genotype, &Score), With<Simulation>>) -> SimulationEpochMetrics {
    if simulations.is_empty() {
        return SimulationEpochMetrics {
            best_score: 0.0,
            average_score: 0.0,
            average_coherence: 0.0,
            diversity_index: 0.0,
            high_coherence_count: 0,
            total_simulations: 0,
        };
    }

    let genomes: Vec<(&Genotype, &Score)> = simulations.iter().collect();
    let scores: Vec<f32> = genomes.iter().map(|(_, score)| score.get()).collect();
    let coherences: Vec<f32> = genomes.iter().map(|(genome, _)| genome.strategy_coherence).collect();

    let best_score = scores.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
    let average_score = scores.iter().sum::<f32>() / scores.len() as f32;
    let average_coherence = coherences.iter().sum::<f32>() / coherences.len() as f32;
    let high_coherence_count = coherences.iter().filter(|&&c| c > 0.7).count();

    // Calculer la diversité génétique
    let diversity_index = calculate_population_genetic_diversity(&genomes);

    SimulationEpochMetrics {
        best_score,
        average_score,
        average_coherence,
        diversity_index,
        high_coherence_count,
        total_simulations: genomes.len(),
    }
}

fn calculate_population_genetic_diversity(genomes: &[(&Genotype, &Score)]) -> f32 {
    if genomes.len() < 2 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let mut comparisons = 0;

    for i in 0..genomes.len() {
        for j in i+1..genomes.len() {
            let genome1 = genomes[i].0;
            let genome2 = genomes[j].0;

            let distance = calculate_genetic_distance(genome1, genome2);
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
    for i in 0..genome1.force_matrix.len().min(genome2.force_matrix.len()) {
        let diff = genome1.force_matrix[i] - genome2.force_matrix[i];
        distance += diff * diff;
    }

    // Distance dans les forces de nourriture
    for i in 0..genome1.food_forces.len().min(genome2.food_forces.len()) {
        let diff = genome1.food_forces[i] - genome2.food_forces[i];
        distance += diff * diff;
    }

    distance.sqrt()
}

fn log_enhanced_epoch_summary(
    metrics: &SimulationEpochMetrics,
    evolution: &EvolutionMetrics,
    sim_params: &SimulationParameters,
) {
    info!("=== 🧬 RÉSUMÉ D'ÉPOQUE {} ===", sim_params.current_epoch);

    // Métriques de performance
    info!("📊 Performance:");
    info!("   • Meilleur score: {:.2}", metrics.best_score);
    info!("   • Score moyen: {:.2}", metrics.average_score);

    // Métriques génétiques
    info!("🧬 Génétique:");
    info!("   • Cohérence moyenne: {:.3}", metrics.average_coherence);
    info!("   • Diversité: {:.3}", metrics.diversity_index);
    info!("   • Génomes cohérents: {}/{}", metrics.high_coherence_count, metrics.total_simulations);

    // Tendances d'évolution
    if evolution.best_score_history.len() >= 2 {
        info!("📈 Évolution:");
        info!("   • Amélioration: {:.2}", evolution.improvement_rate);
        if evolution.stagnation_counter > 0 {
            info!("   • ⚠️  Stagnation: {} époque(s)", evolution.stagnation_counter);
        }

        // Tendance sur les 5 dernières époques
        if evolution.best_score_history.len() >= 5 {
            let recent_trend = calculate_recent_trend(&evolution.best_score_history, 5);
            info!("   • Tendance (5 époques): {:.2}/époque", recent_trend);
        }
    }

    // Configuration génétique actuelle
    info!("⚙️  Configuration:");
    info!("   • {:.0}% élites, {:.0}% mutation, {:.0}% crossover",
        sim_params.elite_ratio * 100.0,
        sim_params.mutation_rate * 100.0,
        sim_params.crossover_rate * 100.0);
}

fn calculate_recent_trend(history: &[f32], window_size: usize) -> f32 {
    if history.len() < window_size {
        return 0.0;
    }

    let recent_data = &history[history.len() - window_size..];
    let first = recent_data[0];
    let last = recent_data[recent_data.len() - 1];

    (last - first) / (window_size - 1) as f32
}

fn log_evolution_predictions(evolution: &EvolutionMetrics) {
    if evolution.best_score_history.len() < 3 {
        return;
    }

    info!("🔮 Prédictions:");

    // Prédire le potentiel d'amélioration
    let stability = calculate_stability(&evolution.best_score_history);
    let diversity_trend = if evolution.diversity_history.len() >= 2 {
        let recent = evolution.diversity_history[evolution.diversity_history.len() - 1];
        let previous = evolution.diversity_history[evolution.diversity_history.len() - 2];
        recent - previous
    } else {
        0.0
    };

    let potential = if stability > 0.9 && diversity_trend > 0.0 {
        "Très élevé"
    } else if stability > 0.7 || diversity_trend > 0.0 {
        "Modéré"
    } else if evolution.stagnation_counter > 3 {
        "Faible - Stagnation détectée"
    } else {
        "Incertain"
    };

    info!("   • Potentiel d'amélioration: {}", potential);

    // Recommandations automatiques
    if evolution.stagnation_counter > 2 {
        info!("   • 💡 Recommandation: Augmenter la mutation ou réduire l'élitisme");
    }

    if evolution.diversity_history.last().unwrap_or(&0.0) < &0.3 {
        info!("   • 💡 Recommandation: Injection de diversité nécessaire");
    }

    if evolution.coherence_history.last().unwrap_or(&0.0) > &0.8 {
        info!("   • ✨ Excellente cohérence - Stratégies stables trouvées");
    }
}

fn calculate_stability(history: &[f32]) -> f32 {
    if history.len() < 3 {
        return 0.0;
    }

    let recent = &history[history.len().saturating_sub(5)..];
    let mean = recent.iter().sum::<f32>() / recent.len() as f32;
    let variance = recent.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f32>() / recent.len() as f32;

    // Stabilité = 1 - variance normalisée
    (1.0 - (variance.sqrt() / mean.max(1.0))).max(0.0)
}

fn update_evolution_metrics(
    mut evolution_metrics: ResMut<EvolutionMetrics>,
    simulations: Query<(&Genotype, &Score), With<Simulation>>,
    sim_params: Res<SimulationParameters>,
) {
    // Mise à jour en temps réel pendant l'époque
    if sim_params.epoch_timer.fraction() > 0.9 {
        // Près de la fin d'époque, calculer les métriques préliminaires
        let current_metrics = calculate_epoch_metrics(&simulations);

        // Détecter les améliorations significatives en cours d'époque
        if let Some(&last_best) = evolution_metrics.best_score_history.last() {
            if current_metrics.best_score > last_best + 10.0 {
                info!("🚀 Amélioration majeure détectée: {:.1} → {:.1}",
                    last_best, current_metrics.best_score);
            }
        }
    }
}

fn auto_export_statistics(
    evolution_metrics: Res<EvolutionMetrics>,
    mut export_config: ResMut<AutoExportConfig>,
    sim_params: Res<SimulationParameters>,
    available_populations: Res<AvailablePopulations>,
) {
    if !export_config.enabled {
        return;
    }

    let should_export =
        // Export périodique
        (sim_params.current_epoch > 0 &&
            sim_params.current_epoch % export_config.export_interval == 0 &&
            sim_params.current_epoch != evolution_metrics.last_export_epoch) ||

            // Export sur amélioration significative
            (export_config.export_on_improvement &&
                evolution_metrics.improvement_rate > export_config.min_improvement_threshold);

    if should_export {
        match export_population_statistics(&available_populations.populations) {
            Ok(()) => {
                info!("📊 Statistiques exportées automatiquement (époque {})", sim_params.current_epoch);
                // Note: Nous ne pouvons pas modifier evolution_metrics ici car il est en Res
                // Il faudrait le passer en ResMut, mais cela créerait un conflit avec auto_export_statistics
            }
            Err(e) => {
                warn!("❌ Échec de l'export automatique: {}", e);
            }
        }
    }
}

fn handle_pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<SimulationState>>,
    mut next_state: ResMut<NextState<SimulationState>>,
    evolution_metrics: Res<EvolutionMetrics>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        match state.get() {
            SimulationState::Running => {
                info!("⏸️  Simulation en pause");
                if let Some(start_time) = evolution_metrics.epoch_start_time {
                    let elapsed = start_time.elapsed().as_secs_f32();
                    info!("   • Époque en cours: {:.1}s écoulées", elapsed);
                }
                next_state.set(SimulationState::Paused);
            }
            SimulationState::Paused => {
                info!("▶️  Reprise de la simulation");
                next_state.set(SimulationState::Running);
            }
            _ => {}
        }
    }

    // Export manuel avec 'E'
    if keyboard.just_pressed(KeyCode::KeyE) {
        info!("📊 Export manuel des statistiques...");
        // Note: L'export serait géré par un événement ou un système séparé
    }

    // Affichage des métriques avec 'M'
    if keyboard.just_pressed(KeyCode::KeyM) {
        log_real_time_metrics(&evolution_metrics);
    }
}

fn log_real_time_metrics(evolution_metrics: &EvolutionMetrics) {
    info!("=== 📊 MÉTRIQUES TEMPS RÉEL ===");

    if let Some(start_time) = evolution_metrics.epoch_start_time {
        let elapsed = start_time.elapsed().as_secs_f32();
        info!("⏱️  Époque actuelle: {:.1}s", elapsed);
    }

    if !evolution_metrics.best_score_history.is_empty() {
        info!("📈 Historique des meilleurs scores:");
        let recent_scores = evolution_metrics.best_score_history.iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>();

        for (i, &score) in recent_scores.iter().enumerate() {
            let epoch_num = evolution_metrics.best_score_history.len() - i;
            info!("   • Époque {}: {:.1}", epoch_num, score);
        }
    }

    if evolution_metrics.stagnation_counter > 0 {
        info!("⚠️  Stagnation: {} époque(s)", evolution_metrics.stagnation_counter);
    }

    info!("🎯 Amélioration récente: {:.2}", evolution_metrics.improvement_rate);
}

fn cleanup_all_enhanced(
    mut commands: Commands,
    simulations: Query<Entity, With<crate::components::simulation::Simulation>>,
    food: Query<Entity, With<crate::components::food::Food>>,
    cameras: Query<Entity, With<crate::systems::viewport_manager::ViewportCamera>>,
    mut entities_spawned: ResMut<EntitiesSpawned>,
    mut evolution_metrics: ResMut<EvolutionMetrics>,
    available_populations: Res<AvailablePopulations>,
    export_config: Res<AutoExportConfig>,
) {
    // Supprimer toutes les entités de simulation
    for entity in simulations.iter() {
        commands.entity(entity).despawn();
    }

    for entity in food.iter() {
        commands.entity(entity).despawn();
    }

    for entity in cameras.iter() {
        commands.entity(entity).despawn();
    }

    // Export final des statistiques si configuré
    if export_config.enabled && !available_populations.populations.is_empty() {
        match export_population_statistics(&available_populations.populations) {
            Ok(()) => info!("📊 Export final des statistiques effectué"),
            Err(e) => warn!("❌ Échec de l'export final: {}", e),
        }
    }

    // Afficher un résumé final de l'évolution
    if !evolution_metrics.best_score_history.is_empty() {
        info!("=== 🏁 RÉSUMÉ FINAL DE L'ÉVOLUTION ===");

        let initial_score = evolution_metrics.best_score_history[0];
        let final_score = evolution_metrics.best_score_history.last().copied().unwrap_or(0.0);
        let total_improvement = final_score - initial_score;
        let epochs_count = evolution_metrics.best_score_history.len();

        info!("📊 Performance globale:");
        info!("   • Score initial: {:.1}", initial_score);
        info!("   • Score final: {:.1}", final_score);
        info!("   • Amélioration totale: {:.1} (+{:.1}%)",
            total_improvement,
            (total_improvement / initial_score.max(1.0)) * 100.0);
        info!("   • Nombre d'époques: {}", epochs_count);

        if epochs_count > 1 {
            let avg_improvement = total_improvement / (epochs_count - 1) as f32;
            info!("   • Amélioration moyenne/époque: {:.2}", avg_improvement);
        }

        // Analyse de la qualité de l'évolution
        let evolution_quality = if total_improvement > 50.0 {
            "Excellente"
        } else if total_improvement > 20.0 {
            "Bonne"
        } else if total_improvement > 5.0 {
            "Modérée"
        } else {
            "Limitée"
        };

        info!("🎯 Qualité de l'évolution: {}", evolution_quality);

        if evolution_metrics.stagnation_counter > 0 {
            info!("⚠️  Simulation terminée en stagnation ({} époques)", evolution_metrics.stagnation_counter);
        }
    }

    // Réinitialiser les états
    entities_spawned.0 = false;
    *evolution_metrics = EvolutionMetrics::default();

    info!("🧹 Nettoyage complet de la simulation terminé");
}