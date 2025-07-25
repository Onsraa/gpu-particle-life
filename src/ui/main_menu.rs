use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::resources::{
    boundary::BoundaryMode,
    food::FoodParameters,
    grid::GridParameters,
    particle_types::ParticleTypesConfig,
    simulation::SimulationParameters,
};
use crate::states::app::AppState;
use crate::globals::*;
use crate::plugins::compute::ComputeEnabled;
use crate::systems::population_save::load_available_populations; 

/// Configuration temporaire pour le menu
#[derive(Resource)]
pub struct MenuConfig {
    // Paramètres de grille
    pub grid_width: f32,
    pub grid_height: f32,
    pub grid_depth: f32,

    // Paramètres de simulation
    pub simulation_count: usize,
    pub particle_count: usize,
    pub particle_types: usize,
    pub epoch_duration: f32,
    pub max_epochs: usize,
    pub max_force_range: f32,

    // Paramètres de nourriture
    pub food_count: usize,
    pub food_respawn_enabled: bool,
    pub food_respawn_time: f32,
    pub food_value: f32,

    // Mode de bords
    pub boundary_mode: BoundaryMode,

    // GPU compute
    pub use_gpu: bool,

    // Paramètres génétiques
    pub elite_ratio: f32,
    pub mutation_rate: f32,
    pub crossover_rate: f32,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            grid_width: DEFAULT_GRID_WIDTH,
            grid_height: DEFAULT_GRID_HEIGHT,
            grid_depth: DEFAULT_GRID_DEPTH,

            simulation_count: DEFAULT_SIMULATION_COUNT,
            particle_count: DEFAULT_PARTICLE_COUNT,
            particle_types: DEFAULT_PARTICLE_TYPES,
            epoch_duration: DEFAULT_EPOCH_DURATION,
            max_epochs: 100,
            max_force_range: DEFAULT_MAX_FORCE_RANGE,

            food_count: DEFAULT_FOOD_COUNT,
            food_respawn_enabled: true,
            food_respawn_time: DEFAULT_FOOD_RESPAWN_TIME,
            food_value: DEFAULT_FOOD_VALUE,

            boundary_mode: BoundaryMode::default(),
            use_gpu: false,

            elite_ratio: DEFAULT_ELITE_RATIO,
            mutation_rate: DEFAULT_MUTATION_RATE,
            crossover_rate: DEFAULT_CROSSOVER_RATE,
        }
    }
}

pub fn main_menu_ui(
    mut contexts: EguiContexts,
    mut menu_config: ResMut<MenuConfig>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut available_populations: ResMut<crate::systems::population_save::AvailablePopulations>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        // Titre avec style amélioré
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Simulation de Vie Artificielle")
                .size(28.0)
                .strong()
                .color(egui::Color32::from_rgb(100, 200, 255)));
            ui.label(egui::RichText::new("Évolution génétique de particules de vie")
                .size(14.0)
                .italics()
                .color(egui::Color32::GRAY));
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);
        });

        // Utiliser un ScrollArea pour tout le contenu
        egui::ScrollArea::vertical().show(ui, |ui| {
            // === Paramètres de grille ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Paramètres de Grille").size(16.0).strong());
                ui.separator();

                egui::Grid::new("grid_params")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Largeur:");
                        ui.add(egui::DragValue::new(&mut menu_config.grid_width)
                            .range(100.0..=2000.0)
                            .suffix(" unités"));
                        ui.end_row();

                        ui.label("Hauteur:");
                        ui.add(egui::DragValue::new(&mut menu_config.grid_height)
                            .range(100.0..=2000.0)
                            .suffix(" unités"));
                        ui.end_row();

                        ui.label("Profondeur:");
                        ui.add(egui::DragValue::new(&mut menu_config.grid_depth)
                            .range(100.0..=2000.0)
                            .suffix(" unités"));
                        ui.end_row();
                    });

                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("Volume total: {:.0} unités³",
                                                     menu_config.grid_width * menu_config.grid_height * menu_config.grid_depth))
                    .small()
                    .color(egui::Color32::GRAY));
            });

            ui.add_space(10.0);

            // === Paramètres de simulation ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("⚙ Paramètres de Simulation").size(16.0).strong());
                ui.separator();

                egui::Grid::new("sim_params")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Nombre de simulations:");
                        ui.add(egui::DragValue::new(&mut menu_config.simulation_count)
                            .range(1..=20));
                        ui.end_row();

                        ui.label("Nombre de particules:");
                        ui.add(egui::DragValue::new(&mut menu_config.particle_count)
                            .range(10..=2000));
                        ui.end_row();

                        ui.label("Types de particules:");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut menu_config.particle_types)
                                .range(2..=5));

                            // Indicateur de diversité
                            let interactions = menu_config.particle_types * menu_config.particle_types;
                            let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8);
                            let diversity_levels = 1 << bits_per_interaction;

                            let diversity_color = match diversity_levels {
                                256.. => egui::Color32::GREEN,
                                64..=255 => egui::Color32::YELLOW,
                                16..=63 => egui::Color32::from_rgb(255, 165, 0), // Orange
                                _ => egui::Color32::RED,
                            };

                            ui.label(egui::RichText::new(format!("({} niveaux)", diversity_levels))
                                .small()
                                .color(diversity_color));
                        });
                        ui.end_row();

                        ui.label("Durée d'une époque:");
                        ui.add(egui::DragValue::new(&mut menu_config.epoch_duration)
                            .range(10.0..=300.0)
                            .suffix(" secondes"));
                        ui.end_row();

                        ui.label("Nombre max d'époques:");
                        ui.add(egui::DragValue::new(&mut menu_config.max_epochs)
                            .range(1..=1000));
                        ui.end_row();

                        ui.label("Portée max des forces:");
                        ui.add(egui::DragValue::new(&mut menu_config.max_force_range)
                            .range(10.0..=500.0)
                            .suffix(" unités"));
                        ui.end_row();
                    });

                ui.add_space(5.0);

                // Informations de diversité détaillées
                ui.collapsing("ℹ Diversité génétique", |ui| {
                    let interactions = menu_config.particle_types * menu_config.particle_types;
                    let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8);
                    let diversity_levels = 1 << bits_per_interaction;
                    let resolution = 2.0 / (diversity_levels - 1) as f32;

                    ui.label(format!("• {} interactions possibles ({}×{})", interactions, menu_config.particle_types, menu_config.particle_types));
                    ui.label(format!("• {} bits par interaction", bits_per_interaction));
                    ui.label(format!("• {} niveaux de force distincts", diversity_levels));
                    ui.label(format!("• Résolution: {:.4} par step", resolution));

                    match menu_config.particle_types {
                        2 => ui.label("Excellent: très fine granularité"),
                        3 => ui.label("Recommandé: bon équilibre diversité/granularité"),
                        4 => ui.label("Acceptable: granularité moyenne"),
                        5 => ui.label("Limité: seulement 4 niveaux par interaction"),
                        _ => ui.label("Non recommandé"),
                    };
                });
            });

            ui.add_space(10.0);

            // === Paramètres génétiques ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Paramètres Génétiques").size(16.0).strong());
                ui.separator();

                egui::Grid::new("genetic_params")
                    .num_columns(3)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Ratio d'élites:");
                        ui.add(egui::DragValue::new(&mut menu_config.elite_ratio)
                            .range(0.01..=0.5)
                            .speed(0.01)
                            .fixed_decimals(2));
                        ui.label(format!("({:.0}% conservés)", menu_config.elite_ratio * 100.0));
                        ui.end_row();

                        ui.label("Taux de mutation:");
                        ui.add(egui::DragValue::new(&mut menu_config.mutation_rate)
                            .range(0.0..=1.0)
                            .speed(0.01)
                            .fixed_decimals(2));
                        ui.label(format!("({:.0}% de chance)", menu_config.mutation_rate * 100.0));
                        ui.end_row();

                        ui.label("Taux de crossover:");
                        ui.add(egui::DragValue::new(&mut menu_config.crossover_rate)
                            .range(0.0..=1.0)
                            .speed(0.01)
                            .fixed_decimals(2));
                        ui.label(format!("({:.0}% de chance)", menu_config.crossover_rate * 100.0));
                        ui.end_row();
                    });

                ui.add_space(5.0);
                ui.label(egui::RichText::new("ℹ Algorithme génétique amélioré avec mutation adaptative")
                    .small()
                    .color(egui::Color32::GRAY));
            });

            ui.add_space(10.0);

            // === Paramètres de nourriture ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Paramètres de Nourriture").size(16.0).strong());
                ui.separator();

                egui::Grid::new("food_params")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Nombre de nourritures:");
                        ui.add(egui::DragValue::new(&mut menu_config.food_count)
                            .range(0..=200));
                        ui.end_row();

                        ui.label("Réapparition:");
                        ui.checkbox(&mut menu_config.food_respawn_enabled, "Activée");
                        ui.end_row();

                        if menu_config.food_respawn_enabled {
                            ui.label("Temps de réapparition:");
                            ui.add(egui::DragValue::new(&mut menu_config.food_respawn_time)
                                .range(1.0..=60.0)
                                .suffix(" secondes"));
                            ui.end_row();
                        }

                        ui.label("Valeur nutritive:");
                        ui.add(egui::DragValue::new(&mut menu_config.food_value)
                            .range(0.1..=10.0)
                            .fixed_decimals(1));
                        ui.end_row();
                    });

                ui.add_space(5.0);
                let density = menu_config.food_count as f32 / (menu_config.grid_width * menu_config.grid_height * menu_config.grid_depth / 1000000.0);
                ui.label(egui::RichText::new(format!("Densité: {:.2} nourritures/million unités³", density))
                    .small()
                    .color(egui::Color32::GRAY));
            });

            ui.add_space(10.0);

            // === Mode de bords ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Mode de Bords").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.radio_value(&mut menu_config.boundary_mode, BoundaryMode::Bounce, "🏀 Rebond");
                    ui.radio_value(&mut menu_config.boundary_mode, BoundaryMode::Teleport, "🌀 Téléportation");
                });

                ui.add_space(5.0);
                match menu_config.boundary_mode {
                    BoundaryMode::Bounce => {
                        ui.label("Les particules rebondissent sur les murs avec amortissement");
                    },
                    BoundaryMode::Teleport => {
                        ui.label("Les particules réapparaissent de l'autre côté (tore 3D)");
                    },
                }
            });

            ui.add_space(10.0);

            // === Paramètres de performance ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Performance").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.checkbox(&mut menu_config.use_gpu, "Utiliser le GPU (Compute Shader)");

                    if menu_config.use_gpu {
                        ui.label("🚀");
                    } else {
                        ui.label("💻");
                    }
                });

                ui.add_space(5.0);
                if menu_config.use_gpu {
                    ui.label("Les calculs d'interactions seront effectués sur le GPU");
                    ui.label("Recommandé pour plus de 500 particules");
                } else {
                    ui.label("Les calculs seront effectués sur le CPU");
                    ui.label("Plus flexible mais plus lent avec beaucoup de particules");
                }
            });

            ui.add_space(20.0);

            // === Boutons d'action ===
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    // Bouton principal : Lancer Simulation
                    if ui.add_sized([200.0, 50.0],
                                    egui::Button::new(egui::RichText::new("Lancer la Simulation").size(18.0))
                                        .fill(egui::Color32::from_rgb(0, 120, 215)))
                        .on_hover_text("Démarre une nouvelle simulation avec algorithme génétique")
                        .clicked() {

                        apply_configuration(&mut commands, &menu_config);
                        next_state.set(AppState::Simulation);
                    }

                    ui.add_space(10.0);

                    // Bouton Visualiseur
                    if ui.add_sized([180.0, 50.0],
                                    egui::Button::new(egui::RichText::new("Visualiseur").size(16.0))
                                        .fill(egui::Color32::from_rgb(40, 160, 90)))
                        .on_hover_text("Visualise les populations sauvegardées")
                        .clicked() {

                        // Recharger les populations disponibles
                        match crate::systems::population_save::load_all_populations() {
                            Ok(populations) => {
                                available_populations.populations = populations;
                                available_populations.loaded = true;
                                info!("Populations rechargées: {}", available_populations.populations.len());
                            }
                            Err(e) => {
                                error!("Erreur lors du rechargement des populations: {}", e);
                            }
                        }

                        next_state.set(AppState::Visualizer);
                    }
                });

                ui.add_space(10.0);

                // Bouton secondaire : Réinitialiser
                if ui.button(egui::RichText::new("⚙ Réinitialiser").size(14.0))
                    .on_hover_text("Remet tous les paramètres aux valeurs par défaut")
                    .clicked() {
                    *menu_config = MenuConfig::default();
                }
            });

            ui.add_space(20.0);

            // === Informations système ===
            ui.separator();
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Simulation 3D avec Bevy 0.16 • Algorithme génétique adaptatif")
                    .small()
                    .color(egui::Color32::GRAY));
                ui.label(egui::RichText::new("Échap: Quitter • Espace: Pause simulation • Sauvegarde: bouton 💾")
                    .small()
                    .color(egui::Color32::GRAY));
                ui.add_space(10.0);
            });
        });
    });
}

fn apply_configuration(commands: &mut Commands, config: &MenuConfig) {
    // Insérer les ressources configurées
    commands.insert_resource(GridParameters {
        width: config.grid_width,
        height: config.grid_height,
        depth: config.grid_depth,
    });

    commands.insert_resource(SimulationParameters {
        current_epoch: 0,
        max_epochs: config.max_epochs,
        epoch_duration: config.epoch_duration,
        epoch_timer: Timer::from_seconds(config.epoch_duration, TimerMode::Once),
        simulation_count: config.simulation_count,
        particle_count: config.particle_count,
        particle_types: config.particle_types,
        simulation_speed: crate::resources::simulation::SimulationSpeed::Normal,
        max_force_range: config.max_force_range,
        velocity_half_life: 0.043,
        elite_ratio: config.elite_ratio,
        mutation_rate: config.mutation_rate,
        crossover_rate: config.crossover_rate,
    });

    commands.insert_resource(ParticleTypesConfig::new(config.particle_types));

    commands.insert_resource(FoodParameters {
        food_count: config.food_count,
        respawn_enabled: config.food_respawn_enabled,
        respawn_cooldown: config.food_respawn_time,
        food_value: config.food_value,
    });

    commands.insert_resource(config.boundary_mode);

    commands.insert_resource(ComputeEnabled(config.use_gpu));

    info!("Configuration appliquée:");
    info!("  • Grille: {}×{}×{}", config.grid_width, config.grid_height, config.grid_depth);
    info!("  • Simulations: {} avec {} particules chacune", config.simulation_count, config.particle_count);
    info!("  • Types: {} (diversité: {} niveaux)", config.particle_types, 1 << ((64 / (config.particle_types * config.particle_types).max(1)).max(2).min(8)));
    info!("  • Algorithme génétique: {:.0}% élites, {:.0}% mutation, {:.0}% crossover",
          config.elite_ratio * 100.0, config.mutation_rate * 100.0, config.crossover_rate * 100.0);
    info!("  • GPU Compute: {}", if config.use_gpu { "Activé" } else { "CPU seulement" });
}