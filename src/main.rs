use bevy::diagnostic::{FrameCount, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowMode};

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
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Particles life".into(),
                    resolution: (1000., 1000.).into(),
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    present_mode: PresentMode::AutoNoVsync,
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    enabled_buttons: bevy::window::EnabledButtons {
                        maximize: false,
                        ..Default::default()
                    },
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_plugins((
            SetupPlugin,
            SimulationPlugin,
            CameraPlugin,
            UIPlugin))
        .add_systems(Update, (make_visible, exit_game))
        .run();
}

fn make_visible(mut window: Single<&mut Window>, frames: Res<FrameCount>) {
    if frames.0 == 3 {
        window.visible = true;
    }
}

fn exit_game(keyboard_input: Res<ButtonInput<KeyCode>>, mut app_exit_events: EventWriter<AppExit>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.write(AppExit::Success);
    }
}