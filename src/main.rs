use bevy::prelude::*;

mod globals;
mod plugins;
mod systems;
mod ui;
mod components;
mod resources;
mod states;

use plugins::{setup::SetupPlugin, simulation::SimulationPlugin};
use crate::plugins::camera::CameraPlugin;

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
        .add_plugins((
            SetupPlugin,
            SimulationPlugin,
            CameraPlugin
        ))

        // Lancer l'application
        .run();
}