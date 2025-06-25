use crate::systems::viewport_manager::{
    UISpace, assign_render_layers, draw_viewport_borders, update_viewports,
    force_viewport_update_after_startup, delayed_viewport_update,
};
use crate::ui::force_matrix::{ForceMatrixUI, force_matrix_window, simulations_list_ui, speed_control_ui};
use crate::ui::main_menu::{MenuConfig, main_menu_ui};
use crate::states::app::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContextPass, EguiPlugin};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        });

        // Resources
        app.init_resource::<ForceMatrixUI>();
        app.init_resource::<UISpace>();
        app.init_resource::<MenuConfig>();

        // Système pour forcer la mise à jour des viewports après le démarrage
        app.add_systems(Startup, force_viewport_update_after_startup);

        // Système de mise à jour retardée (s'exécute une fois après 0.5 secondes)
        app.add_systems(Update, delayed_viewport_update);

        // Système de debug pour les viewports
        app.add_systems(Update, crate::systems::viewport_debug::debug_window_and_viewports);

        // Systèmes d'assignation des render layers
        app.add_systems(
            Update,
            assign_render_layers
                .run_if(resource_exists::<ForceMatrixUI>)
                .run_if(resource_exists::<UISpace>)
                .run_if(in_state(AppState::Simulation)),
        );

        // Systèmes UI du menu principal
        app.add_systems(
            EguiContextPass,
            main_menu_ui.run_if(in_state(AppState::MainMenu)),
        );

        // Systèmes UI et viewport pour la simulation
        app.add_systems(
            EguiContextPass,
            (
                // Contrôles de vitesse
                speed_control_ui,
                // UI des simulations
                (simulations_list_ui, force_matrix_window),
                // Mise à jour des viewports
                update_viewports
                    .after(simulations_list_ui)
                    .after(force_matrix_window),
                // Dessin des bordures
                draw_viewport_borders.after(update_viewports),
            ).run_if(in_state(AppState::Simulation)),
        );
    }
}