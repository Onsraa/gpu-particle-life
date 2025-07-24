use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashSet;

use crate::components::{
    simulation::{Simulation, SimulationId},
    genotype::Genotype,
    score::Score,
};
use crate::resources::particle_types::ParticleTypesConfig;
use crate::resources::simulation::{SimulationParameters, SimulationSpeed};
use crate::systems::viewport_manager::UISpace;
use crate::plugins::compute::ComputeEnabled;

#[derive(Resource)]
pub struct ForceMatrixUI {
    pub selected_simulation: Option<usize>,
    pub show_matrix_window: bool,
    pub show_simulations_list: bool,
    pub selected_simulations: HashSet<usize>,
}

impl Default for ForceMatrixUI {
    fn default() -> Self {
        let mut selected_simulations = HashSet::new();
        selected_simulations.insert(0);

        Self {
            selected_simulation: None,
            show_matrix_window: false,
            show_simulations_list: true,
            selected_simulations,
        }
    }
}

pub fn simulations_list_ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<ForceMatrixUI>,
    mut ui_space: ResMut<UISpace>,
    simulations: Query<(&SimulationId, &Score, &Genotype), With<Simulation>>,
) {
    let ctx = contexts.ctx_mut();

    if !ui_state.show_simulations_list {
        ui_space.right_panel_width = 0.0;
        return;
    }

    let panel_width = 350.0;

    egui::SidePanel::right("simulations_panel")
        .exact_width(panel_width)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Simulations");

            ui.horizontal(|ui| {
                if ui.button("Tout sélectionner").clicked() {
                    for (sim_id, _, _) in simulations.iter() {
                        ui_state.selected_simulations.insert(sim_id.0);
                    }
                }
                if ui.button("Tout désélectionner").clicked() {
                    ui_state.selected_simulations.clear();
                }
            });

            ui.separator();

            let mut sim_list: Vec<_> = simulations.iter().collect();
            sim_list.sort_by(|a, b| b.1.get().partial_cmp(&a.1.get()).unwrap());

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("simulations_grid")
                    .num_columns(4)
                    .spacing([15.0, 5.0])
                    .striped(true)
                    .min_col_width(40.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Vue").strong());
                        ui.label(egui::RichText::new("Simulation").strong());
                        ui.label(egui::RichText::new("Score").strong());
                        ui.label(egui::RichText::new("Matrice").strong());
                        ui.end_row();

                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.end_row();

                        for (sim_id, score, _genotype) in sim_list {
                            let is_selected_for_matrix = ui_state.selected_simulation == Some(sim_id.0);

                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                let mut is_selected_for_view = ui_state.selected_simulations.contains(&sim_id.0);
                                if ui.checkbox(&mut is_selected_for_view, "").changed() {
                                    if is_selected_for_view {
                                        ui_state.selected_simulations.insert(sim_id.0);
                                    } else {
                                        ui_state.selected_simulations.remove(&sim_id.0);
                                    }
                                }
                            });

                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                let sim_label = if is_selected_for_matrix {
                                    egui::RichText::new(format!("#{}", sim_id.0 + 1))
                                        .color(egui::Color32::from_rgb(100, 200, 255))
                                        .strong()
                                } else {
                                    egui::RichText::new(format!("#{}", sim_id.0 + 1))
                                };

                                if ui.selectable_label(false, sim_label).clicked() {
                                    ui_state.selected_simulation = Some(sim_id.0);
                                    ui_state.show_matrix_window = true;
                                }
                            });

                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                let score_value = score.get();
                                let score_color = if score_value > 50.0 {
                                    egui::Color32::from_rgb(0, 255, 0)
                                } else if score_value > 20.0 {
                                    egui::Color32::from_rgb(255, 255, 0)
                                } else if score_value > 10.0 {
                                    egui::Color32::from_rgb(255, 150, 0)
                                } else {
                                    egui::Color32::from_rgb(200, 200, 200)
                                };
                                ui.label(egui::RichText::new(format!("{:.0}", score_value))
                                    .color(score_color)
                                    .monospace());
                            });

                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                if ui.button("Voir").clicked() {
                                    ui_state.selected_simulation = Some(sim_id.0);
                                    ui_state.show_matrix_window = true;
                                }
                            });

                            ui.end_row();
                        }
                    });
            });

            ui.separator();
            ui.label(format!("{} vue(s) active(s)", ui_state.selected_simulations.len()));
        });

    ui_space.right_panel_width = panel_width;
}

pub fn speed_control_ui(
    mut contexts: EguiContexts,
    mut sim_params: ResMut<SimulationParameters>,
    mut ui_space: ResMut<UISpace>,
    mut compute_enabled: ResMut<ComputeEnabled>,
    time: Res<Time>,
) {
    let ctx = contexts.ctx_mut();

    let top_panel_response = egui::TopBottomPanel::top("controls_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Vitesse:");

            if ui.selectable_label(
                sim_params.simulation_speed == SimulationSpeed::Paused,
                "⏸ Pause"
            ).clicked() {
                sim_params.simulation_speed = SimulationSpeed::Paused;
            }

            if ui.selectable_label(
                sim_params.simulation_speed == SimulationSpeed::Normal,
                "▶ Normal"
            ).clicked() {
                sim_params.simulation_speed = SimulationSpeed::Normal;
            }

            if ui.selectable_label(
                sim_params.simulation_speed == SimulationSpeed::Fast,
                "⏩ Rapide (2x)"
            ).clicked() {
                sim_params.simulation_speed = SimulationSpeed::Fast;
            }

            if ui.selectable_label(
                sim_params.simulation_speed == SimulationSpeed::VeryFast,
                "⏭ Très rapide (4x)"
            ).clicked() {
                sim_params.simulation_speed = SimulationSpeed::VeryFast;
            }

            ui.separator();

            let gpu_text = if compute_enabled.0 { "🚀 GPU Activé" } else { "💻 CPU Only" };
            if ui.selectable_label(compute_enabled.0, gpu_text).clicked() {
                compute_enabled.0 = !compute_enabled.0;
                info!("GPU Compute toggled to: {}", compute_enabled.0);
            }

            ui.separator();

            let progress = sim_params.epoch_timer.fraction();
            let remaining = sim_params.epoch_timer.remaining_secs();

            ui.label(format!("Époque {}/{}", sim_params.current_epoch + 1, sim_params.max_epochs));

            ui.add(egui::ProgressBar::new(progress)
                .text(format!("{:.0}s restantes", remaining))
                .desired_width(150.0));

            ui.separator();

            let fps = 1.0 / time.delta_secs();
            ui.label(format!("FPS: {:.0}", fps));
        });
    });

    ui_space.top_panel_height = top_panel_response.response.rect.height();
}

pub fn force_matrix_window(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<ForceMatrixUI>,
    particle_config: Res<ParticleTypesConfig>,
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
) {
    if !ui_state.show_matrix_window || ui_state.selected_simulation.is_none() {
        return;
    }

    let ctx = contexts.ctx_mut();
    let selected_sim = ui_state.selected_simulation.unwrap();

    egui::Window::new(format!("Matrice des Forces - Simulation #{}", selected_sim + 1))
        .resizable(true)
        .collapsible(true)
        .min_width(500.0)
        .open(&mut ui_state.show_matrix_window)
        .show(ctx, |ui| {
            if let Some((_, genotype)) = simulations.iter()
                .find(|(sim_id, _)| sim_id.0 == selected_sim) {

                let type_count = particle_config.type_count;

                ui.label(format!("Types de particules: {}", type_count));
                ui.label(egui::RichText::new("Forces normalisées entre -2.000 et +2.000")
                    .small()
                    .color(egui::Color32::from_rgb(150, 150, 150)));
                ui.separator();

                // Matrice des forces particule-particule
                ui.label(egui::RichText::new("Forces Particule-Particule").size(14.0).strong());
                ui.add_space(5.0);

                egui::Grid::new("force_matrix_grid")
                    .num_columns(type_count + 1)
                    .spacing([10.0, 4.0])
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        ui.label("De\\Vers");

                        for j in 0..type_count {
                            let (color, _) = particle_config.get_color_for_type(j);
                            ui.label(egui::RichText::new(format!("Type {}", j))
                                .color(egui::Color32::from_rgb(
                                    (color.to_srgba().red * 255.0) as u8,
                                    (color.to_srgba().green * 255.0) as u8,
                                    (color.to_srgba().blue * 255.0) as u8,
                                ))
                                .strong());
                        }
                        ui.end_row();

                        for _ in 0..=type_count {
                            ui.separator();
                        }
                        ui.end_row();

                        for i in 0..type_count {
                            let (color, _) = particle_config.get_color_for_type(i);
                            ui.label(egui::RichText::new(format!("Type {}", i))
                                .color(egui::Color32::from_rgb(
                                    (color.to_srgba().red * 255.0) as u8,
                                    (color.to_srgba().green * 255.0) as u8,
                                    (color.to_srgba().blue * 255.0) as u8,
                                ))
                                .strong());

                            for j in 0..type_count {
                                let force = genotype.get_force(i, j);

                                let color = if force.abs() < 0.05 {
                                    egui::Color32::from_rgb(120, 120, 120)
                                } else if force > 0.0 {
                                    let intensity = (force.abs() * 127.5 + 127.5) as u8;
                                    egui::Color32::from_rgb(0, intensity.max(100), 0)
                                } else {
                                    let intensity = (force.abs() * 127.5 + 127.5) as u8;
                                    egui::Color32::from_rgb(intensity.max(100), 0, 0)
                                };

                                ui.label(egui::RichText::new(format!("{:+.3}", force))
                                    .color(color)
                                    .monospace()
                                    .size(11.0));
                            }
                            ui.end_row();
                        }
                    });

                ui.add_space(10.0);
                ui.separator();

                // Forces de nourriture
                ui.label(egui::RichText::new("Forces Nourriture → Particule").size(14.0).strong());
                ui.add_space(5.0);

                egui::Grid::new("food_forces_grid")
                    .num_columns(type_count)
                    .spacing([20.0, 5.0])
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        for i in 0..type_count {
                            let (color, _) = particle_config.get_color_for_type(i);
                            ui.label(egui::RichText::new(format!("Type {}", i))
                                .color(egui::Color32::from_rgb(
                                    (color.to_srgba().red * 255.0) as u8,
                                    (color.to_srgba().green * 255.0) as u8,
                                    (color.to_srgba().blue * 255.0) as u8,
                                ))
                                .strong());
                        }
                        ui.end_row();

                        for i in 0..type_count {
                            let food_force = genotype.get_food_force(i);

                            let color = if food_force.abs() < 0.05 {
                                egui::Color32::from_rgb(120, 120, 120)
                            } else if food_force > 0.0 {
                                let intensity = (food_force.abs() * 127.5 + 127.5) as u8;
                                egui::Color32::from_rgb(0, intensity.max(100), 0)
                            } else {
                                let intensity = (food_force.abs() * 127.5 + 127.5) as u8;
                                egui::Color32::from_rgb(intensity.max(100), 0, 0)
                            };

                            ui.label(egui::RichText::new(format!("{:+.3}", food_force))
                                .color(color)
                                .monospace()
                                .size(12.0));
                        }
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.separator();

                ui.collapsing("Détails techniques", |ui| {
                    ui.label(format!("Forces particule-particule: {} valeurs", genotype.force_matrix.len()));
                    ui.label(format!("Forces nourriture: {} valeurs", genotype.food_forces.len()));
                    ui.label(format!("Types de particules: {}", genotype.type_count));
                    ui.separator();
                    ui.label(egui::RichText::new("Facteur de force appliqué: 80.0").strong());
                    ui.label("Forces réelles = valeurs × 80.0");
                });
            }
        });
}