use bevy::prelude::*;
use crate::states::simulation::SimulationState;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SimulationState>();
    }
}