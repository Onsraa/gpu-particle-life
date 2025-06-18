use bevy::prelude::*;
use crate::resources::camera::CameraSettings;
use crate::resources::grid::GridParameters;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
        app.init_resource::<GridParameters>();
    }
}