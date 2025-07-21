use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::systems::population_save::{AvailablePopulations, SavedPopulation};
use crate::states::app::AppState;

/// Ressource pour stocker la population sélectionnée pour le visualizer
#[derive(Resource, Default)]
pub struct VisualizerSelection {
    pub selected_population: Option<SavedPopulation>,
    pub search_filter: String,
    pub sort_by: PopulationSortBy,
}

#[derive(Default, PartialEq)]
pub enum PopulationSortBy {
    #[default]
    Date,
    Name,
    Score,
    ParticleCount,
}

/// Ressource pour stocker le génome à visualiser
#[derive(Resource)]
pub struct VisualizerGenome(pub crate::components::genotype::Genotype);

/// Interface du mode Visualizer pour sélectionner une population - VERSION CORRIGÉE
pub fn visualizer_ui(
    mut contexts: EguiContexts,
    mut visualizer: ResMut<VisualizerSelection>,
    available: Res<AvailablePopulations>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Visualiseur de Populations Sauvegardées");
            ui.separator();
        });

        ui.horizontal(|ui| {
            // Barre de recherche
            ui.label("Recherche:");
            ui.text_edit_singleline(&mut visualizer.search_filter);

            ui.separator();

            // Options de tri
            ui.label("Trier par:");
            egui::ComboBox::from_label("")
                .selected_text(match visualizer.sort_by {
                    PopulationSortBy::Date => "Date",
                    PopulationSortBy::Name => "Nom",
                    PopulationSortBy::Score => "Score",
                    PopulationSortBy::ParticleCount => "Nb. Particules",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut visualizer.sort_by, PopulationSortBy::Date, "Date");
                    ui.selectable_value(&mut visualizer.sort_by, PopulationSortBy::Name, "Nom");
                    ui.selectable_value(&mut visualizer.sort_by, PopulationSortBy::Score, "Score");
                    ui.selectable_value(&mut visualizer.sort_by, PopulationSortBy::ParticleCount, "Nb. Particules");
                });

            ui.separator();

            // Bouton retour au menu
            if ui.button("↶ Retour au Menu").clicked() {
                next_state.set(AppState::MainMenu);
            }
        });

        ui.separator();

        if available.populations.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("Aucune population sauvegardée trouvée.");
                ui.label("Lancez d'abord des simulations et sauvegardez des génomes intéressants.");
            });
            return;
        }

        // Filtrer et trier les populations
        let mut filtered_populations: Vec<_> = available.populations.iter()
            .filter(|pop| {
                if visualizer.search_filter.is_empty() {
                    true
                } else {
                    let filter = visualizer.search_filter.to_lowercase();
                    pop.name.to_lowercase().contains(&filter) ||
                        pop.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&filter))
                }
            })
            .collect();

        // Trier selon le critère sélectionné
        match visualizer.sort_by {
            PopulationSortBy::Date => {
                filtered_populations.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            PopulationSortBy::Name => {
                filtered_populations.sort_by(|a, b| a.name.cmp(&b.name));
            }
            PopulationSortBy::Score => {
                filtered_populations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            PopulationSortBy::ParticleCount => {
                filtered_populations.sort_by(|a, b| b.simulation_params.particle_count.cmp(&a.simulation_params.particle_count));
            }
        }

        ui.label(format!("Populations trouvées: {} / {}", filtered_populations.len(), available.populations.len()));

        // Liste des populations avec détails
        egui::ScrollArea::vertical().show(ui, |ui| {
            for population in filtered_populations {
                ui.group(|ui| {

                    // En-tête avec nom et date
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&population.name)
                            .size(16.0)
                            .strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(&population.timestamp)
                                .small()
                                .color(egui::Color32::GRAY));
                        });
                    });

                    // Description si présente
                    if let Some(desc) = &population.description {
                        ui.label(egui::RichText::new(desc)
                            .italics()
                            .color(egui::Color32::LIGHT_GRAY));
                    }

                    ui.separator();

                    // Informations techniques en grille
                    egui::Grid::new(format!("pop_info_{}", population.timestamp))
                        .num_columns(4)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("Score:");
                            ui.label(format!("{:.1}", population.score));
                            ui.label("Particules:");
                            ui.label(format!("{}", population.simulation_params.particle_count));
                            ui.end_row();

                            ui.label("Types:");
                            ui.label(format!("{}", population.simulation_params.particle_types));
                            ui.label("Nourriture:");
                            ui.label(format!("{}", population.food_params.food_count));
                            ui.end_row();

                            ui.label("Grille:");
                            ui.label(format!("{:.0}×{:.0}×{:.0}",
                                             population.grid_params.width,
                                             population.grid_params.height,
                                             population.grid_params.depth));
                            ui.label("Bords:");
                            ui.label(match population.boundary_mode {
                                crate::systems::population_save::SavedBoundaryMode::Bounce => "Rebond",
                                crate::systems::population_save::SavedBoundaryMode::Teleport => "Téléport",
                            });
                            ui.end_row();
                        });

                    ui.add_space(10.0);

                    // BOUTONS D'ACTION - BIEN VISIBLES
                    ui.horizontal(|ui| {
                        // Bouton principal de visualisation - GRAND ET COLORÉ
                        if ui.add_sized([200.0, 40.0],
                                        egui::Button::new(egui::RichText::new("🔍 VISUALISER").size(16.0))
                                            .fill(egui::Color32::from_rgb(0, 150, 60)))
                            .on_hover_text("Lancer cette population dans le visualiseur")
                            .clicked() {

                            info!("Lancement de la visualisation: {}", population.name);

                            // Charger cette population et démarrer le visualizer
                            load_population_for_visualization(&mut commands, population.clone());
                            next_state.set(AppState::Visualization);
                        }

                        ui.add_space(10.0);

                        // Bouton détails
                        if ui.add_sized([120.0, 40.0],
                                        egui::Button::new(egui::RichText::new("ℹ️ Détails").size(14.0)))
                            .on_hover_text("Voir les détails de cette population")
                            .clicked() {
                            visualizer.selected_population = Some(population.clone());
                        }
                    });
                });

                ui.add_space(8.0);
            }
        });

        // Fenêtre de détails si une population est sélectionnée
        if let Some(ref selected) = visualizer.selected_population.clone() {
            show_population_details(ctx, &mut visualizer.selected_population, selected);
        }
    });
}

/// Fenêtre de détails d'une population
fn show_population_details(
    ctx: &egui::Context,
    selected_ref: &mut Option<SavedPopulation>,
    population: &SavedPopulation,
) {
    let mut is_open = true;

    egui::Window::new(format!("Détails: {}", population.name))
        .resizable(true)
        .default_width(600.0)
        .open(&mut is_open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Informations générales
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Informations Générales").size(14.0).strong());
                    ui.separator();

                    egui::Grid::new("general_info")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("Nom:");
                            ui.label(&population.name);
                            ui.end_row();

                            ui.label("Date de création:");
                            ui.label(&population.timestamp);
                            ui.end_row();

                            ui.label("Score obtenu:");
                            ui.label(format!("{:.2}", population.score));
                            ui.end_row();
                        });

                    if let Some(desc) = &population.description {
                        ui.label("Description:");
                        ui.label(desc);
                    }
                });

                ui.add_space(10.0);

                // Paramètres de simulation
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Paramètres de Simulation").size(14.0).strong());
                    ui.separator();

                    egui::Grid::new("sim_params")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("Nombre de particules:");
                            ui.label(format!("{}", population.simulation_params.particle_count));
                            ui.end_row();

                            ui.label("Types de particules:");
                            ui.label(format!("{}", population.simulation_params.particle_types));
                            ui.end_row();

                            ui.label("Portée des forces:");
                            ui.label(format!("{:.1}", population.simulation_params.max_force_range));
                            ui.end_row();

                            ui.label("Demi-vie vélocité:");
                            ui.label(format!("{:.3}s", population.simulation_params.velocity_half_life));
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Informations sur le génome
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Génome").size(14.0).strong());
                    ui.separator();

                    ui.label(format!("Génome principal: 0x{:016X}", population.genotype.genome));
                    ui.label(format!("Génome nourriture: 0x{:04X}", population.genotype.food_force_genome));

                    let interactions = population.genotype.type_count * population.genotype.type_count;
                    let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8);
                    ui.label(format!("Interactions possibles: {} ({} bits chacune)", interactions, bits_per_interaction));
                });

                ui.add_space(10.0);

                // Environnement
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Environnement").size(14.0).strong());
                    ui.separator();

                    egui::Grid::new("env_params")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("Taille grille:");
                            ui.label(format!("{:.0} × {:.0} × {:.0}",
                                             population.grid_params.width,
                                             population.grid_params.height,
                                             population.grid_params.depth));
                            ui.end_row();

                            ui.label("Mode bords:");
                            ui.label(match population.boundary_mode {
                                crate::systems::population_save::SavedBoundaryMode::Bounce => "Rebond",
                                crate::systems::population_save::SavedBoundaryMode::Teleport => "Téléportation",
                            });
                            ui.end_row();

                            ui.label("Nourritures:");
                            ui.label(format!("{}", population.food_params.food_count));
                            ui.end_row();

                            ui.label("Respawn nourriture:");
                            ui.label(if population.food_params.respawn_enabled { "Activé" } else { "Désactivé" });
                            ui.end_row();

                            if population.food_params.respawn_enabled {
                                ui.label("Temps respawn:");
                                ui.label(format!("{:.1}s", population.food_params.respawn_cooldown));
                                ui.end_row();
                            }
                        });
                });
            });
        });

    if !is_open {
        *selected_ref = None;
    }
}

/// Charge une population pour la visualisation
fn load_population_for_visualization(commands: &mut Commands, population: SavedPopulation) {
    let (genotype, sim_params, grid_params, food_params, particle_config, boundary_mode) =
        population.to_bevy_resources();

    // Insérer toutes les ressources nécessaires
    commands.insert_resource(sim_params);
    commands.insert_resource(grid_params);
    commands.insert_resource(food_params);
    commands.insert_resource(particle_config);
    commands.insert_resource(boundary_mode);

    // Ressource spéciale pour le visualizer avec le génome spécifique
    commands.insert_resource(VisualizerGenome(genotype));

    info!("Population '{}' chargée pour visualisation", population.name);
}