use crate::plugins::core::camera::CameraPlugin;
use crate::plugins::core::setup::SetupPlugin;
use crate::plugins::simulation::compute::ParticleComputePlugin;
use crate::plugins::simulation::simulation::SimulationPlugin;
use crate::plugins::simulation::visualizer::VisualizerPlugin;
use crate::plugins::ui::ui_plugin::UIPlugin;
use crate::states::app::AppState;
use bevy::diagnostic::{FrameCount, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowMode};
use bevy_app_compute::prelude::*;

mod components;
mod globals;
mod plugins;
mod resources;
mod states;
mod systems;
mod ui;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Simulation de Vie Artificielle".into(),
                    resolution: (1200., 800.).into(),
                    mode: WindowMode::Windowed,
                    present_mode: PresentMode::AutoNoVsync,
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    enabled_buttons: bevy::window::EnabledButtons {
                        maximize: true,
                        ..Default::default()
                    },
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            AppComputePlugin,
        ))
        .add_plugins((
            SetupPlugin,
            SimulationPlugin,
            ParticleComputePlugin,
            CameraPlugin,
            UIPlugin,
            VisualizerPlugin,
        ))
        .add_systems(Update, (make_visible, exit_game))
        .run();
}

fn make_visible(mut window: Single<&mut Window>, frames: Res<FrameCount>) {
    if frames.0 == 3 {
        window.visible = true;
    }
}

fn exit_game(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match state.get() {
            AppState::MainMenu => {
                app_exit_events.write(AppExit::Success);
            }
            AppState::Simulation => {
                next_state.set(AppState::MainMenu);
            }
            AppState::Visualization => {
                next_state.set(AppState::MainMenu);
            }
            AppState::Visualizer => {
                next_state.set(AppState::MainMenu);
            }
        }
    }
}
