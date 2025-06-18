use crate::resources::camera::CameraSettings;
use crate::systems::camera::orbit;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
        app.add_systems(Update, orbit);
    }
}
