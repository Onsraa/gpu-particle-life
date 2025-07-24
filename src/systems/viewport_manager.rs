use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::render::camera::{ClearColorConfig, Projection, PerspectiveProjection};
use bevy::window::WindowResized;
use crate::ui::force_matrix::ForceMatrixUI;
use crate::resources::grid::GridParameters;

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

/// Calcule la distance adaptative de la caméra selon la taille de la grille
fn calculate_adaptive_camera_distance(grid: &GridParameters, viewport_count: usize) -> f32 {
    let diagonal_3d = (grid.width.powi(2) + grid.height.powi(2) + grid.depth.powi(2)).sqrt();
    let base_distance = diagonal_3d * 0.8;

    let viewport_factor = match viewport_count {
        1 => 1.0,
        2 => 1.1,
        3..=4 => 1.2,
        _ => 1.3,
    };

    let final_distance = base_distance * viewport_factor;
    final_distance
}

/// Gère les viewports et caméras pour les simulations sélectionnées
pub fn update_viewports(
    mut commands: Commands,
    ui_state: Res<ForceMatrixUI>,
    ui_space: Res<UISpace>,
    grid_params: Res<GridParameters>,
    windows: Query<&Window>,
    mut existing_cameras: Query<(Entity, &mut Camera, &mut Transform, &mut RenderLayers, &mut ViewportCamera)>,
    force_update: Option<Res<ForceViewportUpdate>>,
    mut resize_events: EventReader<WindowResized>,
) {
    let has_resize = !resize_events.is_empty();
    resize_events.clear();

    let should_update = force_update.is_some() ||
        ui_state.is_changed() ||
        ui_space.is_changed() ||
        grid_params.is_changed() ||
        has_resize;

    if force_update.is_some() {
        commands.remove_resource::<ForceViewportUpdate>();
    }

    if !should_update {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let scale_factor = window.resolution.scale_factor();
    let window_width_physical = window.resolution.physical_width() as f32;
    let window_height_physical = window.resolution.physical_height() as f32;
    let ui_right_physical = ui_space.right_panel_width * scale_factor;
    let ui_top_physical = ui_space.top_panel_height * scale_factor;

    let available_width = window_width_physical - ui_right_physical;
    let available_height = window_height_physical - ui_top_physical;

    if available_width <= 0.0 || available_height <= 0.0 {
        return;
    }

    let mut selected_sims: Vec<usize> = ui_state.selected_simulations.iter().cloned().collect();
    selected_sims.sort();

    let mut cameras_to_reuse: Vec<Entity> = existing_cameras.iter().map(|(e, _, _, _, _)| e).collect();

    if selected_sims.is_empty() {
        for (_, mut camera, _, _, _) in existing_cameras.iter_mut() {
            camera.is_active = false;
        }
        return;
    }

    let viewport_count = selected_sims.len();
    let camera_distance = calculate_adaptive_camera_distance(&grid_params, viewport_count);

    for (idx, &sim_id) in selected_sims.iter().enumerate() {
        let (x, y, w, h) = calculate_viewport_rect(
            idx,
            viewport_count,
            available_width,
            available_height,
            ui_top_physical,
            window_height_physical
        );

        if w == 0 || h == 0 {
            continue;
        }

        if let Some(camera_entity) = cameras_to_reuse.pop() {
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
            spawn_viewport_camera(&mut commands, x, y, w, h, idx, sim_id, camera_distance);
        }
    }

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
    let margin = 8.0; // Marge plus grande pour éviter les chevauchements

    let (x, y_from_top, w, h) = match total {
        1 => (
            margin,
            margin,
            available_width - 2.0 * margin,
            available_height - 2.0 * margin
        ),
        2 => {
            let width = (available_width - 3.0 * margin) / 2.0;
            (
                margin + (idx as f32 * (width + margin)),
                margin,
                width,
                available_height - 2.0 * margin
            )
        },
        3 => {
            let width = (available_width - 4.0 * margin) / 3.0;
            (
                margin + (idx as f32 * (width + margin)),
                margin,
                width,
                available_height - 2.0 * margin
            )
        },
        4 => {
            let width = (available_width - 3.0 * margin) / 2.0;
            let height = (available_height - 3.0 * margin) / 2.0;
            let col = idx % 2;
            let row = idx / 2;
            (
                margin + (col as f32 * (width + margin)),
                margin + (row as f32 * (height + margin)),
                width,
                height
            )
        },
        _ => {
            let cols = (total as f32).sqrt().ceil() as usize;
            let rows = ((total as f32) / (cols as f32)).ceil() as usize;
            let width = (available_width - (cols + 1) as f32 * margin) / cols as f32;
            let height = (available_height - (rows + 1) as f32 * margin) / rows as f32;
            let col = idx % cols;
            let row = idx / cols;
            (
                margin + (col as f32 * (width + margin)),
                margin + (row as f32 * (height + margin)),
                width,
                height
            )
        }
    };

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

    let camera_pos = Vec3::new(
        distance * 0.7,
        distance * 0.8,
        distance * 0.7
    );

    *transform = Transform::from_translation(camera_pos)
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
    let camera_pos = Vec3::new(
        distance * 0.7,
        distance * 0.8,
        distance * 0.7
    );

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
        Transform::from_translation(camera_pos)
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