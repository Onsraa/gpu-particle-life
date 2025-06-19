use crate::systems::viewport_manager::{
    draw_viewport_borders, update_viewports, assign_render_layers,
};
use crate::ui::force_matrix::{ForceMatrixUI, force_matrix_ui, simulations_list_ui};
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
            Update,
            assign_render_layers,  // Assigner les layers aux entités
        );
        app.add_systems(
            EguiContextPass,
            (
                force_matrix_ui,
                simulations_list_ui,
                update_viewports,
                draw_viewport_borders,
            ),
        );
    }
}