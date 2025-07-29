use crate::resources::config::food::FoodParameters;
use crate::resources::config::particle_types::ParticleTypesConfig;
use crate::resources::config::simulation::SimulationParameters;
use crate::resources::world::boundary::BoundaryMode;
use crate::resources::world::grid::GridParameters;
use crate::states::app::AppState;
use bevy::prelude::*;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
        app.init_resource::<GridParameters>();
        app.init_resource::<ParticleTypesConfig>();
        app.init_resource::<SimulationParameters>();
        app.init_resource::<FoodParameters>();
        app.init_resource::<BoundaryMode>();
    }
}
