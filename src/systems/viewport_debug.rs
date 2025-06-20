use bevy::prelude::*;
use crate::systems::viewport_manager::{ViewportCamera, UISpace};

/// Système pour débugger les dimensions de fenêtre et viewports
pub fn debug_window_and_viewports(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &ViewportCamera)>,
    ui_space: Res<UISpace>,
    mut timer: Local<Timer>,
    time: Res<Time>,
) {
    // Débugger toutes les 2 secondes
    if timer.duration() == std::time::Duration::ZERO {
        *timer = Timer::from_seconds(2.0, TimerMode::Repeating);
    }

    timer.tick(time.delta());

    if !timer.just_finished() {
        return;
    }

    if let Ok(window) = windows.single() {
        info!("=== Window Debug Info ===");
        info!("Resolution: {}x{}", window.resolution.physical_width(), window.resolution.physical_height());
        info!("Logical size: {}x{}", window.width(), window.height());
        info!("Scale factor: {}", window.resolution.scale_factor());
        info!("Mode: {:?}", window.mode);
        info!("UI Space - Right: {}, Top: {}", ui_space.right_panel_width, ui_space.top_panel_height);

        let available_width_physical = window.resolution.physical_width() as f32 - (ui_space.right_panel_width * window.resolution.scale_factor());
        let available_height_physical = window.resolution.physical_height() as f32 - (ui_space.top_panel_height * window.resolution.scale_factor());
        info!("Available space (physical): {}x{}", available_width_physical, available_height_physical);

        // Afficher les infos des viewports
        let active_count = cameras.iter().filter(|(c, _)| c.is_active).count();
        info!("Active viewports: {}", active_count);

        for (camera, viewport_camera) in cameras.iter() {
            if camera.is_active {
                if let Some(viewport) = &camera.viewport {
                    info!("Viewport {} - Pos: ({}, {}), Size: {}x{}",
                        viewport_camera.simulation_id,
                        viewport.physical_position.x,
                        viewport.physical_position.y,
                        viewport.physical_size.x,
                        viewport.physical_size.y
                    );
                }
            }
        }
    }
}