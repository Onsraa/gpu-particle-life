use bevy::prelude::*;
use crate::resources::grid::GridParameters;

struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridParameters>();
    }
}