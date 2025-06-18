use bevy::prelude::*;

mod components;
mod globals;
mod plugins;
mod resources;
mod states;
mod systems;
mod ui;

use crate::plugins::camera::CameraPlugin;
use crate::plugins::ui::UIPlugin;
use plugins::{setup::SetupPlugin, simulation::SimulationPlugin};

fn main() {
    App::new()
        // Plugins Bevy de base
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GPU Particle Life Simulator".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        // Nos plugins
        .add_plugins((SetupPlugin, SimulationPlugin, CameraPlugin, UIPlugin))
        // Lancer l'application
        .run();
}
