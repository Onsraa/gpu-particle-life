use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use crate::resources::{
    grid::GridParameters,
    simulation::SimulationParameters,
    particle_types::ParticleTypesConfig,
    food::FoodParameters,
};
use crate::resources::boundary::BoundaryMode;
use crate::states::app::AppState;

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