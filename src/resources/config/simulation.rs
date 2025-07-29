use bevy::prelude::*;
use crate::globals::*;

#[derive(Default, PartialEq, Eq, Clone)]
pub enum SimulationSpeed {
    Paused,
    #[default]
    Normal,
    Fast,
    VeryFast,
}

impl SimulationSpeed {
    pub fn multiplier(&self) -> f32 {
        match self {
            SimulationSpeed::Paused => 0.0,
            SimulationSpeed::Normal => 1.0,
            SimulationSpeed::Fast => 2.0,
            SimulationSpeed::VeryFast => 4.0,
        }
    }
}

#[derive(Resource, Clone)]
pub struct SimulationParameters {
    // Paramètres d'époque
    pub current_epoch: usize,
    pub max_epochs: usize,
    pub epoch_duration: f32,
    pub epoch_timer: Timer,

    // Paramètres de simulation
    pub simulation_count: usize,
    pub particle_count: usize,
    pub particle_types: usize,
    pub simulation_speed: SimulationSpeed,

    // Paramètres des forces
    pub max_force_range: f32,
    pub velocity_half_life: f32,

    // Paramètres génétiques
    pub elite_ratio: f32,
    pub mutation_rate: f32,
    pub crossover_rate: f32,
}

impl Default for SimulationParameters {
    fn default() -> Self {
        Self {
            current_epoch: 0,
            max_epochs: 100,
            epoch_duration: DEFAULT_EPOCH_DURATION,
            epoch_timer: Timer::from_seconds(DEFAULT_EPOCH_DURATION, TimerMode::Once),

            simulation_count: DEFAULT_SIMULATION_COUNT,
            particle_count: DEFAULT_PARTICLE_COUNT,
            particle_types: DEFAULT_PARTICLE_TYPES,
            simulation_speed: SimulationSpeed::default(),

            max_force_range: DEFAULT_MAX_FORCE_RANGE,
            velocity_half_life: 0.043,

            elite_ratio: DEFAULT_ELITE_RATIO,
            mutation_rate: DEFAULT_MUTATION_RATE,
            crossover_rate: DEFAULT_CROSSOVER_RATE,
        }
    }
}

impl SimulationParameters {
    /// Met à jour le timer avec le delta time
    pub fn tick(&mut self, delta: std::time::Duration) {
        if self.simulation_speed != SimulationSpeed::Paused {
            let scaled_delta = delta.mul_f32(self.simulation_speed.multiplier());
            self.epoch_timer.tick(scaled_delta);
        }
    }

    /// Vérifie si l'époque est terminée
    pub fn is_epoch_finished(&self) -> bool {
        self.epoch_timer.finished()
    }

    /// Démarre une nouvelle époque
    pub fn start_new_epoch(&mut self) {
        self.current_epoch += 1;
        self.epoch_timer.reset();
    }
}