use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::components::{
    simulation::{Simulation, SimulationId},
    genotype::Genotype,
};
use crate::resources::particle_types::ParticleTypesConfig;

/// Ressource pour stocker l'état de l'UI
#[derive(Resource, Default)]
pub struct ForceMatrixUI {
    pub selected_simulation: usize,
    pub show_window: bool,
}

/// Système pour afficher la matrice des forces
pub fn force_matrix_ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<ForceMatrixUI>,
    particle_config: Res<ParticleTypesConfig>,
    mut simulations: Query<(&SimulationId, &mut Genotype), With<Simulation>>,
) {
    let ctx = contexts.ctx_mut();

    // Menu pour toggle la fenêtre
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Matrice des Forces").clicked() {
                ui_state.show_window = !ui_state.show_window;
            }
        });
    });

    if !ui_state.show_window {
        return;
    }

    egui::Window::new("Matrice des Forces")
        .resizable(true)
        .show(ctx, |ui| {
            // Sélection de la simulation
            ui.horizontal(|ui| {
                ui.label("Simulation:");
                if let Some((_, genotype)) = simulations.iter_mut().nth(ui_state.selected_simulation) {
                    ui.label(format!("#{}", ui_state.selected_simulation));

                    // Boutons pour changer de simulation
                    if ui.button("<").clicked() && ui_state.selected_simulation > 0 {
                        ui_state.selected_simulation -= 1;
                    }

                    let sim_count = simulations.iter().count();
                    if ui.button(">").clicked() && ui_state.selected_simulation < sim_count - 1 {
                        ui_state.selected_simulation += 1;
                    }
                }
            });

            ui.separator();

            // Afficher et éditer la matrice
            if let Some((_, mut genotype)) = simulations.iter_mut().nth(ui_state.selected_simulation) {
                let type_count = particle_config.type_count;

                ui.label(format!("Types de particules: {}", type_count));
                ui.separator();

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

                // Matrice avec sliders
                let mut new_forces = vec![vec![0.0; type_count]; type_count];
                let mut genome_changed = false;

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
                            let mut force = genotype.decode_force(i, j);

                            // Slider pour modifier la force
                            let response = ui.add(
                                egui::Slider::new(&mut force, -10.0..=10.0)
                                    .fixed_decimals(1)
                                    .custom_formatter(|n, _| {
                                        if n > 0.0 { format!("+{:.1}", n) }
                                        else { format!("{:.1}", n) }
                                    })
                            );

                            if response.changed() {
                                genome_changed = true;
                                new_forces[i][j] = force;
                            } else {
                                new_forces[i][j] = force;
                            }
                        }
                    });
                }

                // Si le génome a changé, encoder les nouvelles forces
                if genome_changed {
                    genotype.genome = encode_forces_to_genome(&new_forces, type_count);
                }

                ui.separator();

                // Boutons d'actions
                ui.horizontal(|ui| {
                    if ui.button("Randomiser").clicked() {
                        *genotype = Genotype::random(type_count);
                    }

                    if ui.button("Tout à zéro").clicked() {
                        genotype.genome = 0;
                    }

                    if ui.button("Symétrique").clicked() {
                        make_symmetric(&mut genotype);
                    }
                });

                // Afficher le génome encodé
                ui.separator();
                ui.label(format!("Génome: 0x{:016X}", genotype.genome));
            }
        });
}

/// Encode une matrice de forces en génome
fn encode_forces_to_genome(forces: &Vec<Vec<f32>>, type_count: usize) -> u64 {
    let interactions = type_count * type_count;
    let bits_per_interaction = 64 / interactions.max(1);
    let max_value = (1u64 << bits_per_interaction) - 1;

    let mut genome = 0u64;

    for i in 0..type_count {
        for j in 0..type_count {
            let normalized = (forces[i][j] / crate::globals::FORCE_SCALE_FACTOR + 1.0) / 2.0;
            let clamped = normalized.clamp(0.0, 1.0);
            let raw_value = (clamped * max_value as f32) as u64;

            let index = i * type_count + j;
            let bit_start = index * bits_per_interaction;

            genome |= raw_value << bit_start;
        }
    }

    genome
}

/// Rend la matrice symétrique
fn make_symmetric(genotype: &mut Genotype) {
    let type_count = genotype.type_count;
    let mut forces = vec![vec![0.0; type_count]; type_count];

    // Lire les forces actuelles
    for i in 0..type_count {
        for j in 0..type_count {
            forces[i][j] = genotype.decode_force(i, j);
        }
    }

    // Rendre symétrique
    for i in 0..type_count {
        for j in i+1..type_count {
            let avg = (forces[i][j] + forces[j][i]) / 2.0;
            forces[i][j] = avg;
            forces[j][i] = avg;
        }
    }

    // Réencoder
    genotype.genome = encode_forces_to_genome(&forces, type_count);
}