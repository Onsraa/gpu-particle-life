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

/// Ressource pour stocker l'état de l'UI
#[derive(Resource)]
pub struct ForceMatrixUI {
    pub selected_simulation: Option<usize>,
    pub show_matrix_window: bool,
    pub show_simulations_list: bool,
    pub selected_simulations: HashSet<usize>, // Simulations à afficher
}

impl Default for ForceMatrixUI {
    fn default() -> Self {
        let mut selected_simulations = HashSet::new();
        selected_simulations.insert(0); // Sélectionner la première simulation par défaut

        Self {
            selected_simulation: None,
            show_matrix_window: false,
            show_simulations_list: true,
            selected_simulations,
        }
    }
}

/// Système pour afficher la liste des simulations avec checkboxes
pub fn simulations_list_ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<ForceMatrixUI>,
    mut ui_space: ResMut<UISpace>,
    simulations: Query<(&SimulationId, &Score, &Genotype), With<Simulation>>,
) {
    let ctx = contexts.ctx_mut();

    if !ui_state.show_simulations_list {
        // Si la fenêtre est fermée, libérer l'espace
        ui_space.right_panel_width = 0.0;
        return;
    }

    let panel_width = 350.0; // Largeur fixe du panneau

    egui::SidePanel::right("simulations_panel")
        .exact_width(panel_width)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Simulations");

            // Boutons pour sélectionner/désélectionner toutes
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

            // En-tête du tableau
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.label("Vue");
                ui.separator();
                ui.add_space(5.0);
                ui.label("Simulation");
                ui.separator();
                ui.add_space(5.0);
                ui.label("Score");
                ui.separator();
                ui.add_space(5.0);
                ui.label("Matrice");
            });

            ui.separator();

            // Liste des simulations avec scores
            let mut sim_list: Vec<_> = simulations.iter().collect();
            sim_list.sort_by(|a, b| b.1.get().partial_cmp(&a.1.get()).unwrap()); // Trier par score décroissant

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (sim_id, score, _genotype) in sim_list {
                    let is_selected_for_matrix = ui_state.selected_simulation == Some(sim_id.0);

                    ui.horizontal(|ui| {
                        // Style de sélection
                        if is_selected_for_matrix {
                            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(100, 200, 255));
                        }

                        ui.add_space(10.0);

                        let mut is_selected_for_view = ui_state.selected_simulations.contains(&sim_id.0);

                        // Checkbox pour la vue
                        if ui.checkbox(&mut is_selected_for_view, "").changed() {
                            if is_selected_for_view {
                                ui_state.selected_simulations.insert(sim_id.0);
                            } else {
                                ui_state.selected_simulations.remove(&sim_id.0);
                            }
                        }

                        ui.separator();
                        ui.add_space(15.0);

                        // Numéro de simulation
                        ui.label(format!("#{}", sim_id.0 + 1));

                        ui.separator();
                        ui.add_space(15.0);

                        // Score avec coloration selon la valeur
                        let score_value = score.get();
                        let score_color = if score_value > 10.0 {
                            egui::Color32::GREEN
                        } else if score_value > 5.0 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::WHITE
                        };

                        ui.label(egui::RichText::new(format!("{:.0}", score_value))
                            .color(score_color));

                        ui.separator();
                        ui.add_space(15.0);

                        // Bouton pour voir la matrice
                        if ui.small_button("Voir").clicked() {
                            ui_state.selected_simulation = Some(sim_id.0);
                            ui_state.show_matrix_window = true;
                        }
                    });

                    // Cliquer sur la ligne pour sélectionner
                    let response = ui.interact(ui.min_rect(), ui.id().with(sim_id.0), egui::Sense::click());
                    if response.clicked() {
                        ui_state.selected_simulation = Some(sim_id.0);
                        ui_state.show_matrix_window = true;
                    }
                }
            });

            ui.separator();
            ui.label(format!("{} vue(s) active(s)", ui_state.selected_simulations.len()));
        });

    // Mettre à jour l'espace occupé par l'UI
    ui_space.right_panel_width = panel_width;
}

/// Système pour afficher les contrôles de vitesse
pub fn speed_control_ui(
    mut contexts: EguiContexts,
    mut sim_params: ResMut<SimulationParameters>,
    mut ui_space: ResMut<UISpace>,
    mut compute_enabled: ResMut<ComputeEnabled>,
    time: Res<Time>,
) {
    let ctx = contexts.ctx_mut();

    // Panneau du haut pour les contrôles
    let top_panel_response = egui::TopBottomPanel::top("controls_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Contrôles de vitesse
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

            // Toggle GPU
            let gpu_text = if compute_enabled.0 { "🚀 GPU Activé" } else { "💻 CPU Only" };
            if ui.selectable_label(compute_enabled.0, gpu_text).clicked() {
                compute_enabled.0 = !compute_enabled.0;
                info!("GPU Compute toggled to: {}", compute_enabled.0);
            }

            ui.separator();

            // Informations sur l'époque
            let progress = sim_params.epoch_timer.fraction();
            let remaining = sim_params.epoch_timer.remaining_secs();

            ui.label(format!("Époque {}/{}", sim_params.current_epoch + 1, sim_params.max_epochs));

            // Barre de progression
            ui.add(egui::ProgressBar::new(progress)
                .text(format!("{:.0}s restantes", remaining))
                .desired_width(150.0));

            ui.separator();

            // FPS
            let fps = 1.0 / time.delta_secs();
            ui.label(format!("FPS: {:.0}", fps));
        });
    });

    // Stocker la hauteur du panneau du haut
    ui_space.top_panel_height = top_panel_response.response.rect.height();
}

/// Fenêtre de visualisation de la matrice (lecture seule)
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
        .open(&mut ui_state.show_matrix_window)
        .show(ctx, |ui| {
            // Trouver la simulation sélectionnée
            if let Some((_, genotype)) = simulations.iter()
                .find(|(sim_id, _)| sim_id.0 == selected_sim) {

                let type_count = particle_config.type_count;

                ui.label(format!("Types de particules: {}", type_count));
                ui.separator();

                // === Matrice des forces particule-particule ===
                ui.label(egui::RichText::new("Forces Particule-Particule").strong());

                // En-têtes de colonnes
                ui.horizontal(|ui| {
                    ui.label("De\\Vers");
                    for j in 0..type_count {
                        let (color, _) = particle_config.get_color_for_type(j);
                        ui.label(egui::RichText::new(format!("Type {}", j))
                            .color(egui::Color32::from_rgb(
                                (color.to_srgba().red * 255.0) as u8,
                                (color.to_srgba().green * 255.0) as u8,
                                (color.to_srgba().blue * 255.0) as u8,
                            )));
                    }
                });

                // Matrice en lecture seule
                for i in 0..type_count {
                    ui.horizontal(|ui| {
                        let (color, _) = particle_config.get_color_for_type(i);
                        ui.label(egui::RichText::new(format!("Type {}", i))
                            .color(egui::Color32::from_rgb(
                                (color.to_srgba().red * 255.0) as u8,
                                (color.to_srgba().green * 255.0) as u8,
                                (color.to_srgba().blue * 255.0) as u8,
                            )));

                        for j in 0..type_count {
                            let force = genotype.decode_force(i, j);

                            // Couleur selon la valeur de la force
                            let color = if force > 0.0 {
                                egui::Color32::from_rgb(0, 200, 0)
                            } else if force < 0.0 {
                                egui::Color32::from_rgb(200, 0, 0)
                            } else {
                                egui::Color32::GRAY
                            };

                            ui.label(egui::RichText::new(format!("{:+.1}", force))
                                .color(color)
                                .monospace());
                        }
                    });
                }

                ui.separator();

                // === Forces de nourriture ===
                ui.label(egui::RichText::new("Forces Nourriture → Particule").strong());

                ui.horizontal(|ui| {
                    for i in 0..type_count {
                        let (color, _) = particle_config.get_color_for_type(i);
                        let food_force = genotype.decode_food_force(i);

                        let force_color = if food_force > 0.0 {
                            egui::Color32::from_rgb(0, 200, 0)
                        } else if food_force < 0.0 {
                            egui::Color32::from_rgb(200, 0, 0)
                        } else {
                            egui::Color32::GRAY
                        };

                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(format!("Type {}", i))
                                .color(egui::Color32::from_rgb(
                                    (color.to_srgba().red * 255.0) as u8,
                                    (color.to_srgba().green * 255.0) as u8,
                                    (color.to_srgba().blue * 255.0) as u8,
                                )));
                            ui.label(egui::RichText::new(format!("{:+.1}", food_force))
                                .color(force_color)
                                .monospace());
                        });
                    }
                });

                ui.separator();

                // Informations sur le génome
                ui.label(format!("Génome principal: 0x{:016X}", genotype.genome));
                ui.label(format!("Génome nourriture: 0x{:04X}", genotype.food_force_genome));
            }
        });
}