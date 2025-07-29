use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::systems::rendering::viewport_manager::{ViewportCamera, UISpace};
use crate::ui::panels::force_matrix::ForceMatrixUI;

/// Système pour dessiner les overlays des numéros de simulation sur chaque viewport
pub fn draw_viewport_overlays(
    mut contexts: EguiContexts,
    ui_state: Res<ForceMatrixUI>,
    ui_space: Res<UISpace>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &ViewportCamera)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let ctx = contexts.ctx_mut();

    // Obtenir le scale factor
    let scale_factor = window.resolution.scale_factor();

    // Calculer l'espace disponible
    let window_width_physical = window.resolution.physical_width() as f32;
    let window_height_physical = window.resolution.physical_height() as f32;
    let ui_right_physical = ui_space.right_panel_width * scale_factor;
    let ui_top_physical = ui_space.top_panel_height * scale_factor;

    let available_width = window_width_physical - ui_right_physical;
    let available_height = window_height_physical - ui_top_physical;

    if available_width <= 0.0 || available_height <= 0.0 {
        return;
    }

    let selected_sims: Vec<usize> = ui_state.selected_simulations.iter().cloned().collect();

    if selected_sims.is_empty() {
        return;
    }

    // Pour chaque caméra active, dessiner l'overlay
    for (camera, viewport_camera) in cameras.iter() {
        if !camera.is_active {
            continue;
        }

        if let Some(viewport) = &camera.viewport {
            let sim_id = viewport_camera.simulation_id;

            // Convertir les coordonnées physiques en coordonnées logiques pour egui
            let logical_x = viewport.physical_position.x as f32 / scale_factor;
            let logical_y = viewport.physical_position.y as f32 / scale_factor;
            let logical_width = viewport.physical_size.x as f32 / scale_factor;
            let logical_height = viewport.physical_size.y as f32 / scale_factor;

            // Convertir en coordonnées egui (Y=0 en haut)
            let egui_y = (window_height_physical / scale_factor) - logical_y - logical_height;

            // Créer une fenêtre overlay pour ce viewport
            egui::Window::new(format!("viewport_overlay_{}", sim_id))
                .title_bar(false)
                .resizable(false)
                .movable(false)
                .collapsible(false)
                .fixed_pos(egui::pos2(logical_x + 10.0, egui_y + 10.0))
                .fixed_size(egui::vec2(100.0, 40.0))
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    // Style du texte avec fond semi-transparent
                    let text_color = egui::Color32::WHITE;
                    let bg_color = egui::Color32::from_rgba_premultiplied(0, 0, 0, 180);

                    ui.painter().rect_filled(
                        ui.available_rect_before_wrap(),
                        egui::CornerRadius::same(4),
                        bg_color,
                    );

                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(format!("#{}", sim_id + 1))
                                .color(text_color)
                                .size(14.0)
                                .strong()
                        );
                    });
                });
        }
    }
}