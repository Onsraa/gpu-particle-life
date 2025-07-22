use crate::states::app::AppState;
use crate::systems::viewport_manager::{
    UISpace, assign_render_layers, delayed_viewport_update,
    force_viewport_update_after_startup, update_viewports,
};
use crate::systems::viewport_overlay::draw_viewport_overlays;
use crate::ui::force_matrix::{ForceMatrixUI, force_matrix_window, speed_control_ui};
use crate::ui::main_menu::{MenuConfig, main_menu_ui};
use crate::ui::save_population::{
    SavePopulationUI, enhanced_simulations_list_ui, save_population_ui,
};
use crate::ui::visualizer::{VisualizerSelection, visualizer_ui};
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
        app.init_resource::<SavePopulationUI>();
        app.init_resource::<VisualizerSelection>();

        // Système pour forcer la mise à jour des viewports après le démarrage
        app.add_systems(Startup, force_viewport_update_after_startup);

        // Système de mise à jour retardée
        app.add_systems(Update, delayed_viewport_update);

        // Système de debug pour les viewports
        app.add_systems(
            Update,
            crate::systems::viewport_debug::debug_window_and_viewports,
        );

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

        // Systèmes UI du visualiseur
        app.add_systems(
            EguiContextPass,
            visualizer_ui.run_if(in_state(AppState::Visualizer)),
        );

        // Systèmes UI et viewport pour la simulation
        app.add_systems(
            EguiContextPass,
            (
                // Contrôles de vitesse
                speed_control_ui,
                // UI des simulations avec sauvegarde
                (
                    enhanced_simulations_list_ui,
                    force_matrix_window,
                    save_population_ui,
                ),
                // Mise à jour des viewports
                update_viewports
                    .after(enhanced_simulations_list_ui)
                    .after(force_matrix_window),
                // Overlays des numéros de simulation (gardé)
                draw_viewport_overlays.after(update_viewports),
            )
                .run_if(in_state(AppState::Simulation)),
        );

        // Systèmes pour la visualisation
        app.add_systems(
            EguiContextPass,
            (speed_control_ui, draw_viewport_overlays).run_if(in_state(AppState::Visualization)),
        );
    }
}