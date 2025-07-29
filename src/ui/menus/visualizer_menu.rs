use crate::components::genetics::genotype::Genotype;
use crate::states::app::AppState;
use crate::systems::persistence::population_save::*;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

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
pub struct VisualizerGenome(pub Genotype);

pub fn visualizer_ui(
    mut contexts: EguiContexts,
    mut visualizer: ResMut<VisualizerSelection>,
    mut available: ResMut<AvailablePopulations>, // Changé en mut
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    let ctx = contexts.ctx_mut();

    // Charger les populations si pas encore fait
    if !available.loaded {
        match load_all_populations() {
            Ok(populations) => {
                available.populations = populations;
                available.loaded = true;
                info!(
                    "Populations chargées dans le visualizer: {}",
                    available.populations.len()
                );
            }
            Err(e) => {
                error!("Erreur lors du chargement des populations: {}", e);
            }
        }
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Visualiseur de Populations Sauvegardées");
            ui.separator();
        });

        ui.horizontal(|ui| {
            ui.label("Recherche:");
            ui.text_edit_singleline(&mut visualizer.search_filter);

            ui.separator();

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
                    ui.selectable_value(
                        &mut visualizer.sort_by,
                        PopulationSortBy::ParticleCount,
                        "Nb. Particules",
                    );
                });

            ui.separator();

            if ui
                .button("🔄 Recharger")
                .on_hover_text("Recharge les populations du dossier")
                .clicked()
            {
                match load_all_populations() {
                    Ok(populations) => {
                        available.populations = populations;
                        available.loaded = true;
                        info!("Populations rechargées: {}", available.populations.len());
                    }
                    Err(e) => {
                        error!("Erreur lors du rechargement: {}", e);
                    }
                }
            }

            ui.separator();

            if ui.button("Retour au Menu").clicked() {
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
        let mut filtered_populations: Vec<_> = available
            .populations
            .iter()
            .filter(|pop| {
                if visualizer.search_filter.is_empty() {
                    true
                } else {
                    let filter = visualizer.search_filter.to_lowercase();
                    pop.name.to_lowercase().contains(&filter)
                        || pop
                            .description
                            .as_ref()
                            .map_or(false, |d| d.to_lowercase().contains(&filter))
                }
            })
            .collect();

        match visualizer.sort_by {
            PopulationSortBy::Date => {
                filtered_populations.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            PopulationSortBy::Name => {
                filtered_populations.sort_by(|a, b| a.name.cmp(&b.name));
            }
            PopulationSortBy::Score => {
                filtered_populations.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            PopulationSortBy::ParticleCount => {
                filtered_populations.sort_by(|a, b| {
                    b.simulation_params
                        .particle_count
                        .cmp(&a.simulation_params.particle_count)
                });
            }
        }

        ui.label(format!(
            "Populations trouvées: {} / {}",
            filtered_populations.len(),
            available.populations.len()
        ));

        egui::ScrollArea::vertical().show(ui, |ui| {
            for population in filtered_populations {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&population.name).size(16.0).strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&population.timestamp)
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                    });

                    if let Some(desc) = &population.description {
                        ui.label(
                            egui::RichText::new(desc)
                                .italics()
                                .color(egui::Color32::LIGHT_GRAY),
                        );
                    }

                    ui.separator();

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
                            ui.label(format!(
                                "{:.0}×{:.0}×{:.0}",
                                population.grid_params.width,
                                population.grid_params.height,
                                population.grid_params.depth
                            ));
                            ui.label("Bords:");
                            ui.label(match population.boundary_mode {
                                SavedBoundaryMode::Bounce => "Rebond",
                                SavedBoundaryMode::Teleport => "Téléport",
                            });
                            ui.end_row();
                        });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui
                            .add_sized(
                                [200.0, 40.0],
                                egui::Button::new(egui::RichText::new("🔍 VISUALISER").size(16.0))
                                    .fill(egui::Color32::from_rgb(0, 150, 60)),
                            )
                            .on_hover_text("Lancer cette population dans le visualiseur")
                            .clicked()
                        {
                            info!("Lancement de la visualisation: {}", population.name);
                            load_population_for_visualization(&mut commands, population.clone());
                            next_state.set(AppState::Visualization);
                        }

                        ui.add_space(10.0);

                        if ui
                            .add_sized(
                                [120.0, 40.0],
                                egui::Button::new(egui::RichText::new("ℹ Détails").size(14.0)),
                            )
                            .on_hover_text("Voir les détails de cette population")
                            .clicked()
                        {
                            visualizer.selected_population = Some(population.clone());
                        }
                    });
                });

                ui.add_space(8.0);
            }
        });

        if let Some(ref selected) = visualizer.selected_population.clone() {
            show_population_details(ctx, &mut visualizer.selected_population, selected);
        }
    });
}

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
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Informations Générales")
                            .size(14.0)
                            .strong(),
                    );
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

                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Paramètres de Simulation")
                            .size(14.0)
                            .strong(),
                    );
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
                            ui.label(format!(
                                "{:.1}",
                                population.simulation_params.max_force_range
                            ));
                            ui.end_row();

                            ui.label("Demi-vie vélocité:");
                            ui.label(format!(
                                "{:.3}s",
                                population.simulation_params.velocity_half_life
                            ));
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("Génome").size(14.0).strong());
                    ui.separator();

                    ui.label(format!(
                        "Forces particule-particule: {} valeurs",
                        population.genotype.force_matrix.len()
                    ));
                    ui.label(format!(
                        "Forces nourriture: {} valeurs",
                        population.genotype.food_forces.len()
                    ));
                    ui.label(format!("Types gérés: {}", population.genotype.type_count));

                    let interactions =
                        population.genotype.type_count * population.genotype.type_count;
                    ui.label(format!("Interactions possibles: {}", interactions));
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("Environnement").size(14.0).strong());
                    ui.separator();

                    egui::Grid::new("env_params")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("Taille grille:");
                            ui.label(format!(
                                "{:.0} × {:.0} × {:.0}",
                                population.grid_params.width,
                                population.grid_params.height,
                                population.grid_params.depth
                            ));
                            ui.end_row();

                            ui.label("Mode bords:");
                            ui.label(match population.boundary_mode {
                                SavedBoundaryMode::Bounce => "Rebond",
                                SavedBoundaryMode::Teleport => "Téléportation",
                            });
                            ui.end_row();

                            ui.label("Nourritures:");
                            ui.label(format!("{}", population.food_params.food_count));
                            ui.end_row();

                            ui.label("Respawn nourriture:");
                            ui.label(if population.food_params.respawn_enabled {
                                "Activé"
                            } else {
                                "Désactivé"
                            });
                            ui.end_row();

                            if population.food_params.respawn_enabled {
                                ui.label("Temps respawn:");
                                ui.label(format!(
                                    "{:.1}s",
                                    population.food_params.respawn_cooldown
                                ));
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

fn load_population_for_visualization(commands: &mut Commands, population: SavedPopulation) {
    let (genotype, sim_params, grid_params, food_params, particle_config, boundary_mode) =
        population.to_bevy_resources();

    commands.insert_resource(sim_params);
    commands.insert_resource(grid_params);
    commands.insert_resource(food_params);
    commands.insert_resource(particle_config);
    commands.insert_resource(boundary_mode);
    commands.insert_resource(VisualizerGenome(genotype));

    info!(
        "Population '{}' chargée pour visualisation",
        population.name
    );
}
