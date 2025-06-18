use bevy::prelude::*;
use bevy_egui::{EguiContextPass, EguiPlugin};

use crate::ui::force_matrix::{ForceMatrixUI, force_matrix_ui};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        });
        app.init_resource::<ForceMatrixUI>();
        app.add_systems(EguiContextPass, force_matrix_ui);
    }
}
