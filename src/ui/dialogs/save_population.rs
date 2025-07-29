use crate::components::entities::simulation::{Simulation, SimulationId};
use crate::components::genetics::genotype::Genotype;
use crate::components::genetics::score::Score;
use crate::systems::persistence::population_save::{PopulationSaveEvents, PopulationSaveRequest};
use crate::ui::panels::force_matrix::ForceMatrixUI;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

#[derive(Resource, Default)]
pub struct SavePopulationUI {
    pub show_save_dialog: bool,
    pub simulation_to_save: Option<usize>,
    pub save_name: String,
    pub save_description: String,
    pub save_in_progress: bool,
}

pub fn save_population_ui(
    mut contexts: EguiContexts,
    mut save_ui: ResMut<SavePopulationUI>,
    mut save_events: ResMut<PopulationSaveEvents>,
    simulations: Query<(&SimulationId, &Score, &Genotype), With<Simulation>>,
) {
    let ctx = contexts.ctx_mut();

    if save_ui.show_save_dialog {
        let mut is_open = true;

        egui::Window::new("Sauvegarder Population")
            .resizable(false)
            .collapsible(false)
            .default_width(400.0)
            .open(&mut is_open)
            .show(ctx, |ui| {
                if let Some(sim_id) = save_ui.simulation_to_save {
                    if let Some((_, score, genotype)) = simulations
                        .iter()
                        .find(|(simulation_id, _, _)| simulation_id.0 == sim_id)
                    {
                        ui.group(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Simulation #{}", sim_id + 1))
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.label(format!("Score actuel: {:.1}", score.get()));
                            ui.label(format!("Types de particules: {}", genotype.type_count));
                            ui.label(format!(
                                "Forces particule-particule: {}",
                                genotype.force_matrix.len()
                            ));
                            ui.label(format!("Forces nourriture: {}", genotype.food_forces.len()));
                        });

                        ui.separator();

                        ui.label("Nom de la population *");
                        ui.text_edit_singleline(&mut save_ui.save_name);

                        if save_ui.save_name.trim().is_empty() {
                            ui.label(
                                egui::RichText::new("Le nom est obligatoire")
                                    .color(egui::Color32::RED)
                                    .small(),
                            );
                        }

                        ui.add_space(10.0);

                        ui.label("Description (optionnelle)");
                        ui.text_edit_multiline(&mut save_ui.save_description);

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let can_save =
                                !save_ui.save_name.trim().is_empty() && !save_ui.save_in_progress;

                            if ui
                                .add_enabled(can_save, egui::Button::new("💾 Sauvegarder"))
                                .clicked()
                            {
                                save_events.save_requests.push(PopulationSaveRequest {
                                    simulation_id: sim_id,
                                    name: save_ui.save_name.trim().to_string(),
                                    description: if save_ui.save_description.trim().is_empty() {
                                        None
                                    } else {
                                        Some(save_ui.save_description.trim().to_string())
                                    },
                                });

                                save_ui.save_in_progress = true;
                                save_ui.show_save_dialog = false;
                                save_ui.simulation_to_save = None;
                                save_ui.save_name.clear();
                                save_ui.save_description.clear();
                                save_ui.save_in_progress = false;
                            }

                            if ui.button("❌ Annuler").clicked() {
                                save_ui.show_save_dialog = false;
                                save_ui.simulation_to_save = None;
                                save_ui.save_name.clear();
                                save_ui.save_description.clear();
                            }
                        });

                        if save_ui.save_in_progress {
                            ui.add_space(5.0);
                            ui.label("Sauvegarde en cours...");
                        }
                    }
                }
            });

        if !is_open {
            save_ui.show_save_dialog = false;
            save_ui.simulation_to_save = None;
            save_ui.save_name.clear();
            save_ui.save_description.clear();
        }
    }
}

pub fn simulations_list_ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<ForceMatrixUI>,
    mut save_ui: ResMut<SavePopulationUI>,
    mut ui_space: ResMut<crate::systems::rendering::viewport_manager::UISpace>,
    simulations: Query<(&SimulationId, &Score, &Genotype), With<Simulation>>,
) {
    let ctx = contexts.ctx_mut();

    if !ui_state.show_simulations_list {
        ui_space.right_panel_width = 0.0;
        return;
    }

    let panel_width = 400.0;

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
                    .num_columns(5)
                    .spacing([15.0, 5.0])
                    .striped(true)
                    .min_col_width(40.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Vue").strong());
                        ui.label(egui::RichText::new("Simulation").strong());
                        ui.label(egui::RichText::new("Score").strong());
                        ui.label(egui::RichText::new("Matrice").strong());
                        ui.label(egui::RichText::new("Sauvegarder").strong());
                        ui.end_row();

                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.end_row();

                        for (sim_id, score, _genotype) in sim_list {
                            let is_selected_for_matrix =
                                ui_state.selected_simulation == Some(sim_id.0);

                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
                                    let mut is_selected_for_view =
                                        ui_state.selected_simulations.contains(&sim_id.0);
                                    if ui.checkbox(&mut is_selected_for_view, "").changed() {
                                        if is_selected_for_view {
                                            ui_state.selected_simulations.insert(sim_id.0);
                                        } else {
                                            ui_state.selected_simulations.remove(&sim_id.0);
                                        }
                                    }
                                },
                            );

                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
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
                                },
                            );

                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
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
                                    ui.label(
                                        egui::RichText::new(format!("{:.0}", score_value))
                                            .color(score_color)
                                            .monospace(),
                                    );
                                },
                            );

                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
                                    if ui.button("Voir").clicked() {
                                        ui_state.selected_simulation = Some(sim_id.0);
                                        ui_state.show_matrix_window = true;
                                    }
                                },
                            );

                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
                                    if ui
                                        .button("💾")
                                        .on_hover_text("Sauvegarder cette population")
                                        .clicked()
                                    {
                                        save_ui.show_save_dialog = true;
                                        save_ui.simulation_to_save = Some(sim_id.0);
                                        save_ui.save_name = format!("Population_{}", sim_id.0 + 1);
                                        save_ui.save_description.clear();
                                    }
                                },
                            );

                            ui.end_row();
                        }
                    });
            });

            ui.separator();
            ui.label(format!(
                "{} vue(s) active(s)",
                ui_state.selected_simulations.len()
            ));
        });

    ui_space.right_panel_width = panel_width;
}
