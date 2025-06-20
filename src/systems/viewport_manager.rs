use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::render::camera::{ClearColorConfig, Projection, PerspectiveProjection};
use bevy::window::WindowResized;
use crate::ui::force_matrix::ForceMatrixUI;

/// Marqueur pour les caméras des viewports
#[derive(Component)]
pub struct ViewportCamera {
    pub simulation_id: usize,
}

/// Ressource pour stocker les dimensions de l'UI
#[derive(Resource, Default)]
pub struct UISpace {
    pub right_panel_width: f32,
    pub top_panel_height: f32,
}

/// Ressource pour forcer la mise à jour des viewports
#[derive(Resource)]
pub struct ForceViewportUpdate;

/// Système pour forcer la mise à jour des viewports après le démarrage
pub fn force_viewport_update_after_startup(mut commands: Commands) {
    commands.insert_resource(ForceViewportUpdate);
}

/// Système pour forcer une mise à jour retardée des viewports
pub fn delayed_viewport_update(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
    mut update_count: Local<u32>,
) {
    // Forcer plusieurs mises à jour dans les premières secondes
    if *update_count < 10 {
        if timer.is_none() {
            *timer = Some(Timer::from_seconds(0.1, TimerMode::Once));
        }

        if let Some(ref mut t) = *timer {
            t.tick(time.delta());
            if t.just_finished() {
                commands.insert_resource(ForceViewportUpdate);
                *update_count += 1;
                t.reset();
            }
        }
    }
}

/// Gère les viewports et caméras pour les simulations sélectionnées
pub fn update_viewports(
    mut commands: Commands,
    ui_state: Res<ForceMatrixUI>,
    ui_space: Res<UISpace>,
    windows: Query<&Window>,
    mut existing_cameras: Query<(Entity, &mut Camera, &mut Transform, &mut RenderLayers, &mut ViewportCamera)>,
    force_update: Option<Res<ForceViewportUpdate>>,
    mut resize_events: EventReader<WindowResized>,
) {
    // Vérifier si on doit mettre à jour
    let has_resize = !resize_events.is_empty();
    resize_events.clear(); // Consommer les événements

    let should_update = force_update.is_some() ||
        ui_state.is_changed() ||
        ui_space.is_changed() ||
        has_resize;

    if force_update.is_some() {
        commands.remove_resource::<ForceViewportUpdate>();
    }

    if !should_update {
        return;
    }

    // Récupérer la fenêtre
    let Ok(window) = windows.single() else {
        return;
    };

    // Obtenir le scale factor actuel
    let scale_factor = window.resolution.scale_factor();

    // Calculer l'espace disponible en tenant compte du scale factor
    let window_width_physical = window.resolution.physical_width() as f32;
    let window_height_physical = window.resolution.physical_height() as f32;

    // Convertir l'espace UI en pixels physiques
    let ui_right_physical = ui_space.right_panel_width * scale_factor;
    let ui_top_physical = ui_space.top_panel_height * scale_factor;

    let available_width = window_width_physical - ui_right_physical;
    let available_height = window_height_physical - ui_top_physical;

    if available_width <= 0.0 || available_height <= 0.0 {
        return;
    }

    let mut selected_sims: Vec<usize> = ui_state.selected_simulations.iter().cloned().collect();
    selected_sims.sort();

    // Collecter les caméras existantes
    let mut cameras_to_reuse: Vec<Entity> = existing_cameras.iter().map(|(e, _, _, _, _)| e).collect();

    // Si aucune simulation sélectionnée, désactiver toutes les caméras
    if selected_sims.is_empty() {
        for (_, mut camera, _, _, _) in existing_cameras.iter_mut() {
            camera.is_active = false;
        }
        return;
    }

    // Calculer les viewports pour chaque simulation
    let viewport_count = selected_sims.len();
    let camera_distance = 600.0 + (viewport_count as f32 * 100.0);

    for (idx, &sim_id) in selected_sims.iter().enumerate() {
        // Calculer la position et taille du viewport en pixels physiques
        let (x, y, w, h) = calculate_viewport_rect(
            idx,
            viewport_count,
            available_width,
            available_height,
            ui_top_physical,
            window_height_physical
        );

        // S'assurer que les dimensions sont valides
        if w == 0 || h == 0 {
            continue;
        }

        if let Some(camera_entity) = cameras_to_reuse.pop() {
            // Réutiliser une caméra existante
            if let Ok((_, mut camera, mut transform, mut render_layers, mut viewport_camera)) = existing_cameras.get_mut(camera_entity) {
                update_camera_viewport(
                    &mut camera,
                    &mut transform,
                    &mut render_layers,
                    &mut viewport_camera,
                    x, y, w, h,
                    idx,
                    sim_id,
                    camera_distance
                );
            }
        } else {
            // Créer une nouvelle caméra
            spawn_viewport_camera(&mut commands, x, y, w, h, idx, sim_id, camera_distance);
        }
    }

    // Désactiver les caméras non utilisées
    for camera_entity in cameras_to_reuse {
        if let Ok((_, mut camera, _, _, _)) = existing_cameras.get_mut(camera_entity) {
            camera.is_active = false;
        }
    }
}

/// Calcule la position et taille d'un viewport
fn calculate_viewport_rect(
    idx: usize,
    total: usize,
    available_width: f32,
    available_height: f32,
    ui_top: f32,
    window_height: f32
) -> (u32, u32, u32, u32) {
    let (x, y_from_top, w, h) = match total {
        1 => (0.0, 0.0, available_width, available_height),
        2 => {
            let width = available_width / 2.0;
            (idx as f32 * width, 0.0, width, available_height)
        },
        3 => {
            let width = available_width / 3.0;
            (idx as f32 * width, 0.0, width, available_height)
        },
        4 => {
            let width = available_width / 2.0;
            let height = available_height / 2.0;
            let col = idx % 2;
            let row = idx / 2;
            (col as f32 * width, row as f32 * height, width, height)
        },
        _ => {
            let cols = (total as f32).sqrt().ceil() as usize;
            let rows = ((total as f32) / (cols as f32)).ceil() as usize;
            let width = available_width / cols as f32;
            let height = available_height / rows as f32;
            let col = idx % cols;
            let row = idx / cols;
            (col as f32 * width, row as f32 * height, width, height)
        }
    };

    // Convertir en coordonnées Bevy (Y=0 en bas) et ajouter l'offset de l'UI
    let bevy_y = window_height - ui_top - y_from_top - h;

    (x as u32, bevy_y as u32, w as u32, h as u32)
}

/// Met à jour une caméra existante
fn update_camera_viewport(
    camera: &mut Camera,
    transform: &mut Transform,
    render_layers: &mut RenderLayers,
    viewport_camera: &mut ViewportCamera,
    x: u32, y: u32, w: u32, h: u32,
    order: usize,
    sim_id: usize,
    distance: f32,
) {
    camera.is_active = true;
    camera.viewport = Some(bevy::render::camera::Viewport {
        physical_position: UVec2::new(x, y),
        physical_size: UVec2::new(w, h),
        ..default()
    });
    camera.order = order as isize;
    camera.clear_color = ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.02));

    *transform = Transform::from_xyz(distance, distance, distance)
        .looking_at(Vec3::ZERO, Vec3::Y);

    *render_layers = RenderLayers::from_layers(&[0, sim_id + 1]);
    viewport_camera.simulation_id = sim_id;
}

/// Crée une nouvelle caméra de viewport
fn spawn_viewport_camera(
    commands: &mut Commands,
    x: u32, y: u32, w: u32, h: u32,
    order: usize,
    sim_id: usize,
    distance: f32,
) {
    commands.spawn((
        Camera {
            is_active: true,
            viewport: Some(bevy::render::camera::Viewport {
                physical_position: UVec2::new(x, y),
                physical_size: UVec2::new(w, h),
                ..default()
            }),
            order: order as isize,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.02)),
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(distance, distance, distance)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ViewportCamera { simulation_id: sim_id },
        RenderLayers::from_layers(&[0, sim_id + 1]),
    ));
}

/// Assigne les RenderLayers aux simulations et particules
pub fn assign_render_layers(
    mut commands: Commands,
    simulations: Query<(Entity, &crate::components::simulation::SimulationId, &Children), (With<crate::components::simulation::Simulation>, Without<RenderLayers>)>,
    particles: Query<Entity, With<crate::components::particle::Particle>>,
) {
    for (sim_entity, sim_id, children) in simulations.iter() {
        if let Ok(mut entity_commands) = commands.get_entity(sim_entity) {
            entity_commands.insert(RenderLayers::layer(sim_id.0 + 1));
        }

        for child in children.iter() {
            if particles.get(child).is_ok() {
                if let Ok(mut entity_commands) = commands.get_entity(child) {
                    entity_commands.insert(RenderLayers::layer(sim_id.0 + 1));
                }
            }
        }
    }
}

/// Dessine les bordures entre les viewports
pub fn draw_viewport_borders(
    mut gizmos: Gizmos,
    ui_state: Res<ForceMatrixUI>,
    ui_space: Res<UISpace>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let selected_count = ui_state.selected_simulations.len();
    if selected_count <= 1 {
        return;
    }

    let available_width = window.width() - ui_space.right_panel_width;
    if available_width <= 0.0 {
        return;
    }

    let color = Color::srgba(1.0, 1.0, 1.0, 0.2);

    match selected_count {
        2 => {
            let x = available_width / 2.0;
            gizmos.line_2d(
                Vec2::new(x, ui_space.top_panel_height),
                Vec2::new(x, window.height()),
                color
            );
        },
        3 => {
            let width = available_width / 3.0;
            for i in 1..3 {
                let x = i as f32 * width;
                gizmos.line_2d(
                    Vec2::new(x, ui_space.top_panel_height),
                    Vec2::new(x, window.height()),
                    color
                );
            }
        },
        4 => {
            let half_width = available_width / 2.0;
            let half_height = (window.height() - ui_space.top_panel_height) / 2.0;

            gizmos.line_2d(
                Vec2::new(half_width, ui_space.top_panel_height),
                Vec2::new(half_width, window.height()),
                color
            );

            gizmos.line_2d(
                Vec2::new(0.0, ui_space.top_panel_height + half_height),
                Vec2::new(available_width, ui_space.top_panel_height + half_height),
                color
            );
        },
        _ => {
            let cols = (selected_count as f32).sqrt().ceil() as usize;
            let rows = (selected_count as f32 / cols as f32).ceil() as usize;
            let width = available_width / cols as f32;
            let height = (window.height() - ui_space.top_panel_height) / rows as f32;

            for i in 1..cols {
                let x = i as f32 * width;
                gizmos.line_2d(
                    Vec2::new(x, ui_space.top_panel_height),
                    Vec2::new(x, window.height()),
                    color
                );
            }

            for i in 1..rows {
                let y = ui_space.top_panel_height + i as f32 * height;
                gizmos.line_2d(
                    Vec2::new(0.0, y),
                    Vec2::new(available_width, y),
                    color
                );
            }
        }
    }
}