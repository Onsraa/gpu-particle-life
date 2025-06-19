// src/plugins/ui.rs
use crate::systems::viewport_manager::{
    UISpace, assign_render_layers, draw_viewport_borders, update_viewports,
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
        app.init_resource::<UISpace>();

        // Systèmes d'assignation des render layers
        app.add_systems(
            Update,
            assign_render_layers
                .run_if(resource_exists::<ForceMatrixUI>)
                .run_if(resource_exists::<UISpace>),
        );

        // Systèmes UI et viewport
        app.add_systems(
            EguiContextPass,
            (
                // D'abord les UI qui peuvent modifier l'état
                (force_matrix_ui, simulations_list_ui),
                // Ensuite la mise à jour des viewports
                update_viewports
                    .after(force_matrix_ui)
                    .after(simulations_list_ui),
                // Enfin le dessin des bordures
                draw_viewport_borders.after(update_viewports),
            ),
        );
    }
}
