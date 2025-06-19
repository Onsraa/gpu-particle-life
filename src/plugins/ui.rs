use crate::systems::visual_highlight::highlight_selected_simulation;
use crate::ui::force_matrix::{ForceMatrixUI, force_matrix_ui};
use bevy::prelude::*;
use bevy_egui::{EguiContextPass, EguiPlugin};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        });
        app.init_resource::<ForceMatrixUI>();
        app.add_systems(
            EguiContextPass,
            (force_matrix_ui, highlight_selected_simulation),
        );
    }
}
