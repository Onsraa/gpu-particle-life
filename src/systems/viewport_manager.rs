use bevy::prelude::*;
use bevy::render::view::RenderLayers;
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
}

/// Calcule la disposition des viewports selon le nombre de simulations sélectionnées
fn calculate_viewport_layout(
    count: usize,
    window_width: f32,
    window_height: f32,
    ui_space: &UISpace,
) -> Vec<(Vec2, Vec2)> {
    let mut viewports = Vec::new();

    // Espace disponible après l'UI
    let available_width = window_width - ui_space.right_panel_width;
    let available_height = window_height;

    // Vérifier que les dimensions sont valides
    if available_width <= 0.0 || available_height <= 0.0 {
        return viewports;
    }

    match count {
        0 => {},
        1 => {
            // Plein écran dans l'espace disponible
            viewports.push((Vec2::ZERO, Vec2::new(available_width, available_height)));
        },
        2 => {
            // Côte à côte
            let width = available_width / 2.0;
            viewports.push((Vec2::ZERO, Vec2::new(width, available_height)));
            viewports.push((Vec2::new(width, 0.0), Vec2::new(width, available_height)));
        },
        3 => {
            // 3 colonnes
            let width = available_width / 3.0;
            for i in 0..3 {
                viewports.push((
                    Vec2::new(i as f32 * width, 0.0),
                    Vec2::new(width, available_height)
                ));
            }
        },
        4 => {
            // Grille 2x2
            let width = available_width / 2.0;
            let height = available_height / 2.0;
            viewports.push((Vec2::ZERO, Vec2::new(width, height)));
            viewports.push((Vec2::new(width, 0.0), Vec2::new(width, height)));
            viewports.push((Vec2::new(0.0, height), Vec2::new(width, height)));
            viewports.push((Vec2::new(width, height), Vec2::new(width, height)));
        },
        _ => {
            // Pour plus de 4, on fait une grille
            let cols = (count as f32).sqrt().ceil() as usize;
            let rows = (count as f32 / cols as f32).ceil() as usize;
            let width = available_width / cols as f32;
            let height = available_height / rows as f32;

            for i in 0..count.min(cols * rows) {
                let col = i % cols;
                let row = i / cols;
                viewports.push((
                    Vec2::new(col as f32 * width, row as f32 * height),
                    Vec2::new(width, height)
                ));
            }
        }
    }

    viewports
}

/// Calcule la distance optimale de la caméra selon le nombre de viewports
fn calculate_camera_distance(viewport_count: usize) -> f32 {
    match viewport_count {
        1 => 500.0,
        2 => 600.0,
        3 => 700.0,
        4 => 800.0,
        _ => 900.0,
    }
}

/// Gère les viewports et caméras pour les simulations sélectionnées
pub fn update_viewports(
    mut commands: Commands,
    ui_state: Res<ForceMatrixUI>,
    ui_space: Res<UISpace>,
    windows: Query<&Window>,
    mut existing_cameras: Query<(Entity, &mut Camera, &mut Transform, &mut RenderLayers, &mut ViewportCamera)>,
) {
    // Si l'état n'a pas changé, on ne fait rien
    if !ui_state.is_changed() && !ui_space.is_changed() {
        return;
    }

    // Récupérer la fenêtre de manière sûre
    let Ok(window) = windows.single() else {
        return;
    };

    // Vérifier que la fenêtre a des dimensions valides
    if window.width() <= 0.0 || window.height() <= 0.0 {
        return;
    }

    let mut selected_sims: Vec<usize> = ui_state.selected_simulations.iter().cloned().collect();
    selected_sims.sort(); // Trier pour avoir un ordre stable

    // Collecter les caméras existantes
    let mut cameras_to_reuse: Vec<Entity> = existing_cameras.iter().map(|(e, _, _, _, _)| e).collect();

    // Si aucune simulation sélectionnée, désactiver toutes les caméras
    if selected_sims.is_empty() {
        for (_, mut camera, _, _, _) in existing_cameras.iter_mut() {
            camera.is_active = false;
        }
        return;
    }

    // Calculer les viewports en tenant compte de l'espace UI
    let viewports = calculate_viewport_layout(
        selected_sims.len(),
        window.width(),
        window.height(),
        &ui_space,
    );

    // Distance optimale selon le nombre de viewports
    let camera_distance = calculate_camera_distance(selected_sims.len());

    // Mettre à jour ou créer des caméras pour chaque simulation sélectionnée
    for (idx, &sim_id) in selected_sims.iter().enumerate() {
        if let Some((position, size)) = viewports.get(idx) {
            // Vérifier que les dimensions sont valides
            if size.x <= 4.0 || size.y <= 4.0 {
                continue;
            }

            // Ajuster pour les bordures (2 pixels de marge)
            let adjusted_pos = *position + Vec2::splat(2.0);
            let adjusted_size = *size - Vec2::splat(4.0);

            // IMPORTANT: Inverser Y car Bevy utilise Y=0 en bas
            let bevy_y = window.height() - adjusted_pos.y - adjusted_size.y;

            // S'assurer que les valeurs sont valides pour un u32
            let physical_x = (adjusted_pos.x.max(0.0) as u32).min(window.width() as u32);
            let physical_y = (bevy_y.max(0.0) as u32).min(window.height() as u32);
            let physical_width = (adjusted_size.x.max(1.0) as u32).min(window.width() as u32 - physical_x);
            let physical_height = (adjusted_size.y.max(1.0) as u32).min(window.height() as u32 - physical_y);

            if let Some(camera_entity) = cameras_to_reuse.pop() {
                // Réutiliser une caméra existante
                if let Ok((_, mut camera, mut transform, mut render_layers, mut viewport_camera)) = existing_cameras.get_mut(camera_entity) {
                    camera.is_active = true;
                    camera.viewport = Some(bevy::render::camera::Viewport {
                        physical_position: UVec2::new(physical_x, physical_y),
                        physical_size: UVec2::new(physical_width, physical_height),
                        ..default()
                    });
                    camera.order = idx as isize;

                    *transform = Transform::from_xyz(camera_distance, camera_distance, camera_distance)
                        .looking_at(Vec3::ZERO, Vec3::Y);

                    *render_layers = RenderLayers::from_layers(&[0, sim_id + 1]);
                    viewport_camera.simulation_id = sim_id;
                }
            } else {
                // Créer une nouvelle caméra si nécessaire
                commands.spawn((
                    Camera {
                        is_active: true,
                        viewport: Some(bevy::render::camera::Viewport {
                            physical_position: UVec2::new(physical_x, physical_y),
                            physical_size: UVec2::new(physical_width, physical_height),
                            ..default()
                        }),
                        order: idx as isize,
                        ..default()
                    },
                    Camera3d::default(),
                    Transform::from_xyz(camera_distance, camera_distance, camera_distance)
                        .looking_at(Vec3::ZERO, Vec3::Y),
                    ViewportCamera { simulation_id: sim_id },
                    RenderLayers::from_layers(&[0, sim_id + 1]),
                ));
            }
        }
    }

    // Désactiver les caméras non utilisées
    for camera_entity in cameras_to_reuse {
        if let Ok((_, mut camera, _, _, _)) = existing_cameras.get_mut(camera_entity) {
            camera.is_active = false;
        }
    }
}

/// Assigne les RenderLayers aux simulations et particules
pub fn assign_render_layers(
    mut commands: Commands,
    simulations: Query<(Entity, &crate::components::simulation::SimulationId, &Children), (With<crate::components::simulation::Simulation>, Without<RenderLayers>)>,
    particles: Query<Entity, With<crate::components::particle::Particle>>,
) {
    for (sim_entity, sim_id, children) in simulations.iter() {
        // Vérifier que l'entité existe toujours avant d'ajouter des composants
        if let Ok(mut entity_commands) = commands.get_entity(sim_entity) {
            entity_commands.insert(RenderLayers::layer(sim_id.0 + 1));
        }

        // Assigner le même layer à toutes les particules enfants
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
    // Récupérer la fenêtre de manière sûre
    let Ok(window) = windows.single() else {
        return;
    };

    let selected_count = ui_state.selected_simulations.len();

    if selected_count <= 1 {
        return;
    }

    let available_width = window.width() - ui_space.right_panel_width;

    // Vérifier que la largeur est valide
    if available_width <= 0.0 {
        return;
    }

    let color = Color::srgb(0.5, 0.5, 0.5);

    match selected_count {
        2 => {
            // Ligne verticale au milieu
            let x = available_width / 2.0;
            gizmos.line_2d(
                Vec2::new(x, 0.0),
                Vec2::new(x, window.height()),
                color
            );
        },
        3 => {
            // 2 lignes verticales
            let width = available_width / 3.0;
            for i in 1..3 {
                let x = i as f32 * width;
                gizmos.line_2d(
                    Vec2::new(x, 0.0),
                    Vec2::new(x, window.height()),
                    color
                );
            }
        },
        4 => {
            // Une croix
            let half_width = available_width / 2.0;
            let half_height = window.height() / 2.0;

            // Ligne verticale
            gizmos.line_2d(
                Vec2::new(half_width, 0.0),
                Vec2::new(half_width, window.height()),
                color
            );

            // Ligne horizontale
            gizmos.line_2d(
                Vec2::new(0.0, half_height),
                Vec2::new(available_width, half_height),
                color
            );
        },
        _ => {
            // Grille pour plus de 4
            let cols = (selected_count as f32).sqrt().ceil() as usize;
            let rows = (selected_count as f32 / cols as f32).ceil() as usize;
            let width = available_width / cols as f32;
            let height = window.height() / rows as f32;

            // Lignes verticales
            for i in 1..cols {
                let x = i as f32 * width;
                gizmos.line_2d(
                    Vec2::new(x, 0.0),
                    Vec2::new(x, window.height()),
                    color
                );
            }

            // Lignes horizontales
            for i in 1..rows {
                let y = i as f32 * height;
                gizmos.line_2d(
                    Vec2::new(0.0, y),
                    Vec2::new(available_width, y),
                    color
                );
            }
        }
    }
}