use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::components::{
    entities::simulation::*,
    entities::particle::*,
    entities::food::*,
    genetics::genotype::*,
    genetics::score::*,
};

use crate::resources::config::food::FoodParameters;
use crate::resources::config::particle_types::ParticleTypesConfig;
use crate::resources::config::simulation::{SimulationParameters, SimulationSpeed};
use crate::resources::world::boundary::BoundaryMode;
use crate::resources::world::grid::GridParameters;

/// Structure pour sauvegarder une population complète avec ses paramètres
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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedGenotype {
    pub force_matrix: Vec<f32>,
    pub food_forces: Vec<f32>,
    pub type_count: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedSimulationParams {
    pub particle_count: usize,
    pub particle_types: usize,
    pub max_force_range: f32,
    pub velocity_half_life: f32,
    pub epoch_duration: f32,
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
    ) -> Self {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

        Self {
            name,
            timestamp,
            genotype: SavedGenotype {
                force_matrix: genotype.force_matrix.clone(),
                food_forces: genotype.food_forces.clone(),
                type_count: genotype.type_count,
            },
            score,
            simulation_params: SavedSimulationParams {
                particle_count: sim_params.particle_count,
                particle_types: sim_params.particle_types,
                max_force_range: sim_params.max_force_range,
                velocity_half_life: sim_params.velocity_half_life,
                epoch_duration: sim_params.epoch_duration,
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
                colors: particle_config
                    .colors
                    .iter()
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
        }
    }

    pub fn to_bevy_resources(
        &self,
    ) -> (
        Genotype,
        SimulationParameters,
        GridParameters,
        FoodParameters,
        ParticleTypesConfig,
        BoundaryMode,
    ) {
        let genotype = Genotype {
            force_matrix: self.genotype.force_matrix.clone(),
            food_forces: self.genotype.food_forces.clone(),
            type_count: self.genotype.type_count,
        };

        let sim_params = SimulationParameters {
            current_epoch: 0,
            max_epochs: 100,
            epoch_duration: self.simulation_params.epoch_duration,
            epoch_timer: Timer::from_seconds(
                self.simulation_params.epoch_duration,
                TimerMode::Once,
            ),
            simulation_count: 1,
            particle_count: self.simulation_params.particle_count,
            particle_types: self.simulation_params.particle_types,
            simulation_speed: SimulationSpeed::Normal,
            max_force_range: self.simulation_params.max_force_range,
            velocity_half_life: self.simulation_params.velocity_half_life,
            elite_ratio: 0.1,
            mutation_rate: 0.1,
            crossover_rate: 0.7,
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

        let colors = self
            .particle_types_config
            .colors
            .iter()
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

        (
            genotype,
            sim_params,
            grid_params,
            food_params,
            particle_config,
            boundary_mode,
        )
    }
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
        if let Some((_, genotype, score)) = simulations
            .iter()
            .find(|(sim_id, _, _)| sim_id.0 == request.simulation_id)
        {
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
            );

            if let Err(e) = save_population_to_file(&saved_population) {
                error!("Erreur lors de la sauvegarde: {}", e);
            } else {
                info!("Population '{}' sauvegardée avec succès", request.name);
            }
        }
    }
}

pub fn save_population_to_file(
    population: &SavedPopulation,
) -> Result<(), Box<dyn std::error::Error>> {
    let populations_dir = Path::new("populations");
    if !populations_dir.exists() {
        fs::create_dir_all(populations_dir)?;
    }

    let safe_name = population
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    let filename = format!("{}_{}.json", safe_name, population.timestamp);
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

    for entry in fs::read_dir(populations_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<SavedPopulation>(&content) {
                    Ok(population) => populations.push(population),
                    Err(e) => warn!("Erreur lors du chargement de {:?}: {}", path, e),
                },
                Err(e) => warn!("Impossible de lire {:?}: {}", path, e),
            }
        }
    }

    populations.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(populations)
}

pub fn load_available_populations(mut available: ResMut<AvailablePopulations>) {
    if available.loaded {
        return;
    }

    match load_all_populations() {
        Ok(populations) => {
            available.populations = populations;
            available.loaded = true;
            info!(
                "Chargé {} population(s) sauvegardée(s)",
                available.populations.len()
            );
        }
        Err(e) => {
            error!("Erreur lors du chargement des populations: {}", e);
        }
    }
}
