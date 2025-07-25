use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::components::{
    genotype::Genotype,
    simulation::{Simulation, SimulationId},
    score::Score,
};
use crate::resources::{
    boundary::BoundaryMode,
    food::FoodParameters,
    grid::GridParameters,
    particle_types::ParticleTypesConfig,
    simulation::SimulationParameters,
};

/// Structure pour sauvegarder une population complète avec métriques génétiques avancées
#[derive(Serialize, Deserialize, Clone)]
pub struct SavedPopulation {
    pub name: String,
    pub timestamp: String,
    pub genotype: SavedGenotype,
    pub score: f32,
    pub simulation_params: SavedSimulationParams,
    pub grid_params: SavedGridParams,
    pub food_params: SavedFoodParams,
    pub particle_types_config: SavedParticleTypesConfig,
    pub boundary_mode: SavedBoundaryMode,
    pub description: Option<String>,

    // NOUVELLES MÉTRIQUES GÉNÉTIQUES
    pub genetic_metrics: SavedGeneticMetrics,
    pub evolution_context: SavedEvolutionContext,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedGenotype {
    pub force_matrix: Vec<f32>,
    pub food_forces: Vec<f32>,
    pub type_count: usize,

    // NOUVELLES DONNÉES GÉNÉTIQUES
    pub fitness_history: Vec<f32>,
    pub strategy_coherence: f32,
    pub genetic_diversity_score: Option<f32>, // Par rapport à la population d'origine
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedGeneticMetrics {
    pub coherence_score: f32,
    pub fitness_trend: f32,
    pub generation: usize,
    pub rank_in_population: usize,
    pub population_size: usize,
    pub diversity_contribution: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedEvolutionContext {
    pub epoch_number: usize,
    pub population_diversity: f32,
    pub average_population_score: f32,
    pub best_population_score: f32,
    pub improvement_trend: f32,
    pub genetic_algorithm_config: SavedGeneticConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedGeneticConfig {
    pub elite_ratio: f32,
    pub mutation_rate: f32,
    pub crossover_rate: f32,
    pub coherence_threshold: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedSimulationParams {
    pub particle_count: usize,
    pub particle_types: usize,
    pub max_force_range: f32,
    pub velocity_half_life: f32,
    pub epoch_duration: f32,

    // PARAMÈTRES GÉNÉTIQUES SAUVEGARDÉS
    pub elite_ratio: f32,
    pub mutation_rate: f32,
    pub crossover_rate: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedGridParams {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedFoodParams {
    pub food_count: usize,
    pub respawn_enabled: bool,
    pub respawn_cooldown: f32,
    pub food_value: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedParticleTypesConfig {
    pub type_count: usize,
    pub colors: Vec<(f32, f32, f32, f32)>, // RGBA values
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum SavedBoundaryMode {
    Bounce,
    Teleport,
}

/// Statistiques de la population sauvegardée pour analyse
#[derive(Serialize, Deserialize, Clone)]
pub struct PopulationAnalysis {
    pub total_interactions: usize,
    pub strong_attractions: usize,
    pub strong_repulsions: usize,
    pub neutral_interactions: usize,
    pub food_attraction_ratio: f32,
    pub complexity_score: f32,
    pub predicted_behaviors: Vec<String>,
}

#[derive(Resource, Default)]
pub struct PopulationSaveEvents {
    pub save_requests: Vec<PopulationSaveRequest>,
}

#[derive(Clone)]
pub struct PopulationSaveRequest {
    pub simulation_id: usize,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Resource, Default)]
pub struct AvailablePopulations {
    pub populations: Vec<SavedPopulation>,
    pub loaded: bool,
    pub analysis_cache: std::collections::HashMap<String, PopulationAnalysis>,
}

impl SavedPopulation {
    pub fn from_current_state(
        simulation_id: usize,
        name: String,
        description: Option<String>,
        genotype: &Genotype,
        score: f32,
        sim_params: &SimulationParameters,
        grid_params: &GridParameters,
        food_params: &FoodParameters,
        particle_config: &ParticleTypesConfig,
        boundary_mode: &BoundaryMode,
        // NOUVEAUX PARAMÈTRES POUR LE CONTEXTE GÉNÉTIQUE
        rank_in_population: usize,
        population_size: usize,
        population_diversity: f32,
        average_population_score: f32,
        best_population_score: f32,
        improvement_trend: f32,
    ) -> Self {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

        Self {
            name,
            timestamp,
            genotype: SavedGenotype {
                force_matrix: genotype.force_matrix.clone(),
                food_forces: genotype.food_forces.clone(),
                type_count: genotype.type_count,
                fitness_history: genotype.fitness_history.clone(),
                strategy_coherence: genotype.strategy_coherence,
                genetic_diversity_score: None, // Calculé plus tard si nécessaire
            },
            score,
            simulation_params: SavedSimulationParams {
                particle_count: sim_params.particle_count,
                particle_types: sim_params.particle_types,
                max_force_range: sim_params.max_force_range,
                velocity_half_life: sim_params.velocity_half_life,
                epoch_duration: sim_params.epoch_duration,
                elite_ratio: sim_params.elite_ratio,
                mutation_rate: sim_params.mutation_rate,
                crossover_rate: sim_params.crossover_rate,
            },
            grid_params: SavedGridParams {
                width: grid_params.width,
                height: grid_params.height,
                depth: grid_params.depth,
            },
            food_params: SavedFoodParams {
                food_count: food_params.food_count,
                respawn_enabled: food_params.respawn_enabled,
                respawn_cooldown: food_params.respawn_cooldown,
                food_value: food_params.food_value,
            },
            particle_types_config: SavedParticleTypesConfig {
                type_count: particle_config.type_count,
                colors: particle_config.colors.iter()
                    .map(|(color, _emissive)| {
                        let srgba = color.to_srgba();
                        (srgba.red, srgba.green, srgba.blue, srgba.alpha)
                    })
                    .collect(),
            },
            boundary_mode: match boundary_mode {
                BoundaryMode::Bounce => SavedBoundaryMode::Bounce,
                BoundaryMode::Teleport => SavedBoundaryMode::Teleport,
            },
            description,
            genetic_metrics: SavedGeneticMetrics {
                coherence_score: genotype.strategy_coherence,
                fitness_trend: genotype.get_fitness_trend(),
                generation: sim_params.current_epoch,
                rank_in_population,
                population_size,
                diversity_contribution: calculate_diversity_contribution(genotype),
            },
            evolution_context: SavedEvolutionContext {
                epoch_number: sim_params.current_epoch,
                population_diversity,
                average_population_score,
                best_population_score,
                improvement_trend,
                genetic_algorithm_config: SavedGeneticConfig {
                    elite_ratio: sim_params.elite_ratio,
                    mutation_rate: sim_params.mutation_rate,
                    crossover_rate: sim_params.crossover_rate,
                    coherence_threshold: crate::globals::MIN_STRATEGY_COHERENCE,
                },
            },
        }
    }

    pub fn to_bevy_resources(&self) -> (
        Genotype,
        SimulationParameters,
        GridParameters,
        FoodParameters,
        ParticleTypesConfig,
        BoundaryMode,
    ) {
        let mut genotype = Genotype {
            force_matrix: self.genotype.force_matrix.clone(),
            food_forces: self.genotype.food_forces.clone(),
            type_count: self.genotype.type_count,
            fitness_history: self.genotype.fitness_history.clone(),
            strategy_coherence: self.genotype.strategy_coherence,
        };

        // Recalculer la cohérence si nécessaire (validation)
        if genotype.strategy_coherence == 0.0 {
            genotype.strategy_coherence = genotype.calculate_strategy_coherence();
        }

        let sim_params = SimulationParameters {
            current_epoch: 0,
            max_epochs: 100,
            epoch_duration: self.simulation_params.epoch_duration,
            epoch_timer: Timer::from_seconds(self.simulation_params.epoch_duration, TimerMode::Once),
            simulation_count: 1,
            particle_count: self.simulation_params.particle_count,
            particle_types: self.simulation_params.particle_types,
            simulation_speed: crate::resources::simulation::SimulationSpeed::Normal,
            max_force_range: self.simulation_params.max_force_range,
            velocity_half_life: self.simulation_params.velocity_half_life,
            // RESTAURER LES PARAMÈTRES GÉNÉTIQUES
            elite_ratio: self.simulation_params.elite_ratio,
            mutation_rate: self.simulation_params.mutation_rate,
            crossover_rate: self.simulation_params.crossover_rate,
        };

        let grid_params = GridParameters {
            width: self.grid_params.width,
            height: self.grid_params.height,
            depth: self.grid_params.depth,
        };

        let food_params = FoodParameters {
            food_count: self.food_params.food_count,
            respawn_enabled: self.food_params.respawn_enabled,
            respawn_cooldown: self.food_params.respawn_cooldown,
            food_value: self.food_params.food_value,
        };

        let colors = self.particle_types_config.colors.iter()
            .map(|(r, g, b, a)| {
                let base_color = Color::srgba(*r, *g, *b, *a);
                let emissive = base_color.to_linear() * 0.5;
                (base_color, emissive)
            })
            .collect();

        let particle_config = ParticleTypesConfig {
            type_count: self.particle_types_config.type_count,
            colors,
        };

        let boundary_mode = match self.boundary_mode {
            SavedBoundaryMode::Bounce => BoundaryMode::Bounce,
            SavedBoundaryMode::Teleport => BoundaryMode::Teleport,
        };

        (genotype, sim_params, grid_params, food_params, particle_config, boundary_mode)
    }

    /// Analyse comportementale de la population
    pub fn analyze_behavior(&self) -> PopulationAnalysis {
        let mut total_interactions = 0;
        let mut strong_attractions = 0;
        let mut strong_repulsions = 0;
        let mut neutral_interactions = 0;

        // Analyser la matrice des forces
        for i in 0..self.genotype.type_count {
            for j in 0..self.genotype.type_count {
                if i != j { // Ignorer auto-interactions
                    let force_idx = i * self.genotype.type_count + j;
                    if let Some(&force) = self.genotype.force_matrix.get(force_idx) {
                        total_interactions += 1;

                        if force > 0.5 {
                            strong_attractions += 1;
                        } else if force < -0.5 {
                            strong_repulsions += 1;
                        } else {
                            neutral_interactions += 1;
                        }
                    }
                }
            }
        }

        // Analyser les forces de nourriture
        let positive_food_forces = self.genotype.food_forces.iter().filter(|&&f| f > 0.0).count();
        let food_attraction_ratio = positive_food_forces as f32 / self.genotype.food_forces.len() as f32;

        // Calculer un score de complexité
        let complexity_score = calculate_behavior_complexity(&self.genotype);

        // Prédire les comportements probables
        let predicted_behaviors = predict_emergent_behaviors(&self.genotype);

        PopulationAnalysis {
            total_interactions,
            strong_attractions,
            strong_repulsions,
            neutral_interactions,
            food_attraction_ratio,
            complexity_score,
            predicted_behaviors,
        }
    }

    /// Valide la cohérence de la population sauvegardée
    pub fn validate_integrity(&self) -> Result<(), String> {
        // Vérifier la taille de la matrice
        let expected_matrix_size = self.genotype.type_count * self.genotype.type_count;
        if self.genotype.force_matrix.len() != expected_matrix_size {
            return Err(format!("Taille de matrice incorrecte: {} au lieu de {}",
                               self.genotype.force_matrix.len(), expected_matrix_size));
        }

        // Vérifier les forces de nourriture
        if self.genotype.food_forces.len() != self.genotype.type_count {
            return Err(format!("Nombre de forces de nourriture incorrect: {} au lieu de {}",
                               self.genotype.food_forces.len(), self.genotype.type_count));
        }

        // Vérifier les valeurs dans les plages acceptables
        for &force in &self.genotype.force_matrix {
            if !force.is_finite() || force < -3.0 || force > 3.0 {
                return Err(format!("Force hors limites: {}", force));
            }
        }

        for &force in &self.genotype.food_forces {
            if !force.is_finite() || force < -3.0 || force > 3.0 {
                return Err(format!("Force de nourriture hors limites: {}", force));
            }
        }

        // Vérifier la cohérence stratégique minimum
        if self.genotype.strategy_coherence < 0.0 || self.genotype.strategy_coherence > 1.0 {
            return Err(format!("Cohérence stratégique invalide: {}", self.genotype.strategy_coherence));
        }

        Ok(())
    }

    /// Génère un résumé textuel de la population
    pub fn generate_summary(&self) -> String {
        let analysis = self.analyze_behavior();

        format!(
            "Population '{}' (Époque {}, Rang {}/{})\n\
             Score: {:.1} | Cohérence: {:.2} | Tendance: {:.1}\n\
             Interactions: {} attractions fortes, {} répulsions fortes\n\
             Complexité: {:.2} | Attraction nourriture: {:.0}%\n\
             Comportements prédits: {}",
            self.name,
            self.evolution_context.epoch_number,
            self.genetic_metrics.rank_in_population + 1,
            self.genetic_metrics.population_size,
            self.score,
            self.genetic_metrics.coherence_score,
            self.genetic_metrics.fitness_trend,
            analysis.strong_attractions,
            analysis.strong_repulsions,
            analysis.complexity_score,
            analysis.food_attraction_ratio * 100.0,
            analysis.predicted_behaviors.join(", ")
        )
    }
}

/// Calcule la contribution à la diversité d'un génome
fn calculate_diversity_contribution(genotype: &Genotype) -> f32 {
    // Score basé sur l'unicité des patterns d'interaction
    let mut uniqueness_score = 0.0;

    // Analyser les patterns uniques dans la matrice
    for i in 0..genotype.type_count {
        for j in 0..genotype.type_count {
            let force = genotype.get_force(i, j);

            // Récompenser les forces inhabituelles mais cohérentes
            if force.abs() > 0.8 && force.abs() < 1.5 {
                uniqueness_score += 0.1;
            }

            // Récompenser les asymétries intéressantes
            if i != j {
                let reverse_force = genotype.get_force(j, i);
                let asymmetry = (force - reverse_force).abs();
                if asymmetry > 0.3 && asymmetry < 1.0 {
                    uniqueness_score += 0.05;
                }
            }
        }
    }

    // Analyser l'originalité des forces de nourriture
    let food_variety = {
        let mut unique_values = std::collections::HashSet::new();
        for &force in &genotype.food_forces {
            // Convertir en entier pour éviter les problèmes de Hash avec f32
            let quantized = (force * 10.0).round() as i32;
            unique_values.insert(quantized);
        }
        unique_values.len() as f32
    };

    uniqueness_score += food_variety / genotype.type_count as f32;

    uniqueness_score.min(1.0)
}

/// Calcule la complexité comportementale d'un génome
fn calculate_behavior_complexity(genotype: &SavedGenotype) -> f32 {
    let mut complexity = 0.0;

    // Complexité des interactions
    let mut different_force_levels = std::collections::HashSet::new();
    for &force in &genotype.force_matrix {
        different_force_levels.insert((force * 10.0).round() as i32);
    }
    complexity += (different_force_levels.len() as f32) / 20.0; // Normaliser

    // Complexité des cycles d'interaction
    for i in 0..genotype.type_count {
        for j in 0..genotype.type_count {
            if i != j {
                for k in 0..genotype.type_count {
                    if k != i && k != j {
                        let force_ij = genotype.force_matrix[i * genotype.type_count + j];
                        let force_jk = genotype.force_matrix[j * genotype.type_count + k];
                        let force_ki = genotype.force_matrix[k * genotype.type_count + i];

                        // Cycle cohérent = complexité
                        if (force_ij * force_jk * force_ki).abs() > 0.1 {
                            complexity += 0.1;
                        }
                    }
                }
            }
        }
    }

    // Équilibre des forces comme facteur de complexité
    let balance_score = 1.0 - (genotype.food_forces.iter().map(|f| f.abs()).sum::<f32>()
        / genotype.food_forces.len() as f32).min(1.0);
    complexity += balance_score * 0.3;

    complexity.min(2.0) / 2.0 // Normaliser entre 0.0 et 1.0
}

/// Prédit les comportements émergents probables
fn predict_emergent_behaviors(genotype: &SavedGenotype) -> Vec<String> {
    let mut behaviors = Vec::new();

    // Détecter les comportements d'essaim
    let mut swarm_indicators = 0;
    for i in 0..genotype.type_count {
        let self_force = genotype.force_matrix[i * genotype.type_count + i];
        if self_force < -0.2 { // Auto-répulsion = espacement
            swarm_indicators += 1;
        }
    }
    if swarm_indicators >= genotype.type_count / 2 {
        behaviors.push("Comportement d'essaim".to_string());
    }

    // Détecter les relations prédateur-proie
    for i in 0..genotype.type_count {
        for j in 0..genotype.type_count {
            if i != j {
                let force_ij = genotype.force_matrix[i * genotype.type_count + j];
                let force_ji = genotype.force_matrix[j * genotype.type_count + i];

                if force_ij > 0.7 && force_ji < -0.5 {
                    behaviors.push(format!("Relation prédateur-proie (Type {} → Type {})", i, j));
                }
            }
        }
    }

    // Détecter la compétition pour la nourriture
    let high_food_attraction = genotype.food_forces.iter().filter(|&&f| f > 0.5).count();
    if high_food_attraction > genotype.type_count / 2 {
        behaviors.push("Compétition alimentaire".to_string());
    }

    // Détecter les cycles complexes
    let mut cycles_found = 0;
    for i in 0..genotype.type_count {
        for j in 0..genotype.type_count {
            for k in 0..genotype.type_count {
                if i != j && j != k && k != i {
                    let force_ij = genotype.force_matrix[i * genotype.type_count + j];
                    let force_jk = genotype.force_matrix[j * genotype.type_count + k];
                    let force_ki = genotype.force_matrix[k * genotype.type_count + i];

                    if force_ij > 0.3 && force_jk > 0.3 && force_ki > 0.3 {
                        cycles_found += 1;
                    }
                }
            }
        }
    }
    if cycles_found > 0 {
        behaviors.push("Cycles d'attraction complexes".to_string());
    }

    // Détecter la territorialité
    let strong_repulsions = genotype.force_matrix.iter().filter(|&&f| f < -0.8).count();
    if strong_repulsions > genotype.type_count {
        behaviors.push("Territorialité".to_string());
    }

    if behaviors.is_empty() {
        behaviors.push("Comportement neutre/exploratoire".to_string());
    }

    behaviors
}

pub fn process_save_requests(
    mut save_events: ResMut<PopulationSaveEvents>,
    simulations: Query<(&SimulationId, &Genotype, &Score), With<Simulation>>,
    sim_params: Res<SimulationParameters>,
    grid_params: Res<GridParameters>,
    food_params: Res<FoodParameters>,
    particle_config: Res<ParticleTypesConfig>,
    boundary_mode: Res<BoundaryMode>,
) {
    for request in save_events.save_requests.drain(..) {
        if let Some((_, genotype, score)) = simulations.iter()
            .find(|(sim_id, _, _)| sim_id.0 == request.simulation_id) {

            // Calculer le contexte génétique
            let all_simulations: Vec<_> = simulations.iter().collect();
            let mut sorted_by_score = all_simulations.clone();
            sorted_by_score.sort_by(|a, b| b.2.get().partial_cmp(&a.2.get()).unwrap());

            let rank = sorted_by_score.iter()
                .position(|(sim_id, _, _)| sim_id.0 == request.simulation_id)
                .unwrap_or(0);

            let population_size = all_simulations.len();
            let scores: Vec<f32> = all_simulations.iter().map(|(_, _, s)| s.get()).collect();
            let average_score = scores.iter().sum::<f32>() / scores.len() as f32;
            let best_score = scores.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);

            // Calculer la diversité de la population
            let population_diversity = calculate_population_diversity(&all_simulations);

            // Estimer la tendance d'amélioration (simplifié)
            let improvement_trend = score.get() - average_score;

            let saved_population = SavedPopulation::from_current_state(
                request.simulation_id,
                request.name.clone(),
                request.description.clone(),
                genotype,
                score.get(),
                &sim_params,
                &grid_params,
                &food_params,
                &particle_config,
                &boundary_mode,
                rank,
                population_size,
                population_diversity,
                average_score,
                best_score,
                improvement_trend,
            );

            // Valider avant sauvegarde
            if let Err(error) = saved_population.validate_integrity() {
                error!("Validation échouée pour '{}': {}", request.name, error);
                continue;
            }

            match save_population_to_file(&saved_population) {
                Ok(()) => {
                    info!("🧬 Population '{}' sauvegardée avec succès", request.name);
                    info!("   • Score: {:.1} | Cohérence: {:.3} | Rang: {}/{}",
                        saved_population.score,
                        saved_population.genetic_metrics.coherence_score,
                        rank + 1,
                        population_size
                    );

                    // Afficher l'analyse comportementale
                    let analysis = saved_population.analyze_behavior();
                    info!("   • Analyse: {} attractions, {} répulsions, complexité {:.2}",
                        analysis.strong_attractions,
                        analysis.strong_repulsions,
                        analysis.complexity_score
                    );
                }
                Err(e) => {
                    error!("❌ Erreur lors de la sauvegarde de '{}': {}", request.name, e);
                }
            }
        }
    }
}

fn calculate_population_diversity(simulations: &[(&SimulationId, &Genotype, &Score)]) -> f32 {
    if simulations.len() < 2 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let mut comparisons = 0;

    for i in 0..simulations.len() {
        for j in i+1..simulations.len() {
            let genome1 = simulations[i].1;
            let genome2 = simulations[j].1;

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

pub fn save_population_to_file(population: &SavedPopulation) -> Result<(), Box<dyn std::error::Error>> {
    let populations_dir = Path::new("populations");
    if !populations_dir.exists() {
        fs::create_dir_all(populations_dir)?;
    }

    let safe_name = population.name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();

    let filename = format!("{}_{}_c{:.0}.json",
                           safe_name,
                           population.timestamp,
                           population.genetic_metrics.coherence_score * 100.0
    );
    let file_path = populations_dir.join(filename);

    let json = serde_json::to_string_pretty(population)?;
    fs::write(file_path, json)?;

    Ok(())
}

pub fn load_all_populations() -> Result<Vec<SavedPopulation>, Box<dyn std::error::Error>> {
    let populations_dir = Path::new("populations");
    if !populations_dir.exists() {
        return Ok(Vec::new());
    }

    let mut populations = Vec::new();
    let mut validation_errors = 0;

    for entry in fs::read_dir(populations_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<SavedPopulation>(&content) {
                        Ok(mut population) => {
                            // Validation et réparation si nécessaire
                            match population.validate_integrity() {
                                Ok(()) => {
                                    // Recalculer la cohérence si elle semble incorrecte
                                    if population.genotype.strategy_coherence == 0.0 {
                                        warn!("Recalcul de la cohérence pour '{}'", population.name);
                                        let mut genotype = Genotype {
                                            force_matrix: population.genotype.force_matrix.clone(),
                                            food_forces: population.genotype.food_forces.clone(),
                                            type_count: population.genotype.type_count,
                                            fitness_history: population.genotype.fitness_history.clone(),
                                            strategy_coherence: 0.0,
                                        };
                                        genotype.strategy_coherence = genotype.calculate_strategy_coherence();
                                        population.genotype.strategy_coherence = genotype.strategy_coherence;
                                        population.genetic_metrics.coherence_score = genotype.strategy_coherence;
                                    }

                                    populations.push(population);
                                }
                                Err(error) => {
                                    warn!("Population '{}' échouée à la validation: {}",
                                        path.file_name().unwrap_or_default().to_string_lossy(), error);
                                    validation_errors += 1;
                                }
                            }
                        }
                        Err(e) => warn!("Erreur lors du parsing de {:?}: {}", path, e),
                    }
                }
                Err(e) => warn!("Impossible de lire {:?}: {}", path, e),
            }
        }
    }

    // Trier par score combiné (performance + cohérence)
    populations.sort_by(|a, b| {
        let score_a = a.score * 0.7 + a.genetic_metrics.coherence_score * 30.0;
        let score_b = b.score * 0.7 + b.genetic_metrics.coherence_score * 30.0;
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    if validation_errors > 0 {
        warn!("⚠️  {} population(s) ont échoué à la validation", validation_errors);
    }

    info!("📁 {} population(s) chargée(s) avec succès", populations.len());

    Ok(populations)
}

pub fn load_available_populations(mut available: ResMut<AvailablePopulations>) {
    if available.loaded {
        return;
    }

    match load_all_populations() {
        Ok(populations) => {
            // Générer les analyses en cache
            for population in &populations {
                let analysis = population.analyze_behavior();
                available.analysis_cache.insert(population.timestamp.clone(), analysis);
            }

            available.populations = populations;
            available.loaded = true;

            if !available.populations.is_empty() {
                let avg_coherence = available.populations.iter()
                    .map(|p| p.genetic_metrics.coherence_score)
                    .sum::<f32>() / available.populations.len() as f32;

                info!("🧬 {} population(s) chargée(s) - Cohérence moyenne: {:.3}",
                    available.populations.len(), avg_coherence);
            }
        }
        Err(e) => {
            error!("❌ Erreur lors du chargement des populations: {}", e);
        }
    }
}

/// Fonction utilitaire pour exporter les statistiques des populations
pub fn export_population_statistics(populations: &[SavedPopulation]) -> Result<(), Box<dyn std::error::Error>> {
    let stats_dir = Path::new("populations/statistics");
    if !stats_dir.exists() {
        fs::create_dir_all(stats_dir)?;
    }

    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let stats_file = stats_dir.join(format!("population_stats_{}.csv", timestamp));

    let mut csv_content = String::new();
    csv_content.push_str("Name,Timestamp,Score,Coherence,Fitness_Trend,Rank,Population_Size,Epoch,Diversity,Complexity,Attractions,Repulsions\n");

    for pop in populations {
        let analysis = pop.analyze_behavior();
        csv_content.push_str(&format!(
            "{},{},{:.2},{:.3},{:.2},{},{},{},{:.3},{:.3},{},{}\n",
            pop.name,
            pop.timestamp,
            pop.score,
            pop.genetic_metrics.coherence_score,
            pop.genetic_metrics.fitness_trend,
            pop.genetic_metrics.rank_in_population + 1,
            pop.genetic_metrics.population_size,
            pop.evolution_context.epoch_number,
            pop.evolution_context.population_diversity,
            analysis.complexity_score,
            analysis.strong_attractions,
            analysis.strong_repulsions
        ));
    }

    fs::write(stats_file, csv_content)?;
    info!("📊 Statistiques exportées vers populations/statistics/");

    Ok(())
}