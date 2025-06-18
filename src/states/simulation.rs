use bevy::prelude::*;

#[derive(States, Default, PartialEq, Eq, Clone, Hash, Debug)]
pub enum SimulationState {
    #[default]
    Starting,
    Running,
    Paused,
    GeneticSelection,
}