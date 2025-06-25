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
            use_gpu: true, 
        }
    }
}

pub fn main_menu_ui(
    mut contexts: EguiContexts,
    mut menu_config: ResMut<MenuConfig>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        // Titre
        ui.vertical_centered(|ui| {
            ui.heading("Configuration de la Simulation");
            ui.separator();
        });

        // Utiliser un ScrollArea pour tout le contenu
        egui::ScrollArea::vertical().show(ui, |ui| {
            // === Paramètres de grille ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Paramètres de Grille").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Largeur:");
                    ui.add(egui::DragValue::new(&mut menu_config.grid_width)
                        .range(100.0..=2000.0)
                        .suffix(" unités"));
                });

                ui.horizontal(|ui| {
                    ui.label("Hauteur:");
                    ui.add(egui::DragValue::new(&mut menu_config.grid_height)
                        .range(100.0..=2000.0)
                        .suffix(" unités"));
                });

                ui.horizontal(|ui| {
                    ui.label("Profondeur:");
                    ui.add(egui::DragValue::new(&mut menu_config.grid_depth)
                        .range(100.0..=2000.0)
                        .suffix(" unités"));
                });
            });

            ui.add_space(10.0);

            // === Paramètres de simulation ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Paramètres de Simulation").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Nombre de simulations:");
                    ui.add(egui::DragValue::new(&mut menu_config.simulation_count)
                        .range(1..=20));
                });

                ui.horizontal(|ui| {
                    ui.label("Nombre de particules:");
                    ui.add(egui::DragValue::new(&mut menu_config.particle_count)
                        .range(10..=2000));
                });

                ui.horizontal(|ui| {
                    ui.label("Types de particules:");
                    ui.add(egui::DragValue::new(&mut menu_config.particle_types)
                        .range(2..=8));
                });

                ui.horizontal(|ui| {
                    ui.label("Durée d'une époque:");
                    ui.add(egui::DragValue::new(&mut menu_config.epoch_duration)
                        .range(10.0..=300.0)
                        .suffix(" secondes"));
                });

                ui.horizontal(|ui| {
                    ui.label("Nombre max d'époques:");
                    ui.add(egui::DragValue::new(&mut menu_config.max_epochs)
                        .range(1..=1000));
                });

                ui.horizontal(|ui| {
                    ui.label("Portée max des forces:");
                    ui.add(egui::DragValue::new(&mut menu_config.max_force_range)
                        .range(10.0..=500.0)
                        .suffix(" unités"));
                });
            });

            ui.add_space(10.0);

            // === Paramètres de nourriture ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Paramètres de Nourriture").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Nombre de nourritures:");
                    ui.add(egui::DragValue::new(&mut menu_config.food_count)
                        .range(0..=200));
                });

                ui.checkbox(&mut menu_config.food_respawn_enabled, "Réapparition activée");

                if menu_config.food_respawn_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Temps de réapparition:");
                        ui.add(egui::DragValue::new(&mut menu_config.food_respawn_time)
                            .range(1.0..=60.0)
                            .suffix(" secondes"));
                    });
                }

                ui.horizontal(|ui| {
                    ui.label("Valeur nutritive:");
                    ui.add(egui::DragValue::new(&mut menu_config.food_value)
                        .range(0.1..=10.0)
                        .fixed_decimals(1));
                });
            });

            ui.add_space(10.0);

            // === Mode de bords ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Mode de Bords").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.radio_value(&mut menu_config.boundary_mode, BoundaryMode::Bounce, "Rebond");
                    ui.radio_value(&mut menu_config.boundary_mode, BoundaryMode::Teleport, "Téléportation");
                });

                match menu_config.boundary_mode {
                    BoundaryMode::Bounce => {
                        ui.label("Les particules rebondissent sur les murs");
                    },
                    BoundaryMode::Teleport => {
                        ui.label("Les particules réapparaissent de l'autre côté");
                    },
                }
            });

            ui.add_space(10.0);

            // === Paramètres de performance ===
            ui.group(|ui| {
                ui.label(egui::RichText::new("Performance").size(16.0).strong());
                ui.separator();

                ui.checkbox(&mut menu_config.use_gpu, "Utiliser le GPU (Compute Shader)");

                if menu_config.use_gpu {
                    ui.label("🚀 Les calculs d'interactions seront effectués sur le GPU");
                    ui.label("Recommandé pour plus de 500 particules");
                } else {
                    ui.label("💻 Les calculs seront effectués sur le CPU");
                    ui.label("Plus flexible mais plus lent avec beaucoup de particules");
                }
            });

            ui.add_space(20.0);

            // === Boutons d'action ===
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("Lancer la Simulation").size(18.0)).clicked() {
                        // Appliquer la configuration aux ressources
                        apply_configuration(&mut commands, &menu_config);
                        // Changer d'état
                        next_state.set(AppState::Simulation);
                    }

                    if ui.button(egui::RichText::new("Réinitialiser").size(14.0)).clicked() {
                        *menu_config = MenuConfig::default();
                    }
                });
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
    info!("GPU Compute enabled: {}", config.use_gpu);
}