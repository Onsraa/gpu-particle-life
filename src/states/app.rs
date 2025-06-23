use bevy::prelude::*;

/// État principal de l'application
#[derive(States, Default, PartialEq, Eq, Clone, Hash, Debug)]
pub enum AppState {
    #[default]
    MainMenu,
    Simulation,
}