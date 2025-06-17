use bevy::prelude::*;

#[derive(Default)]
enum SimulationSpeed {
    #[default]
    Normal,
    Fast,
    VeryFast,
}

#[derive(Resource)]
struct SimulationParameters {
    current_epoch: usize,
    max_epoch: usize,
    simulation_speed: SimulationSpeed,
}

impl Default for SimulationParameters {
    fn default() -> Self {
        Self {
            current_epoch: 0,
            max_epoch: 1,
            simulation_speed: SimulationSpeed::default(),
        }
    }
}

