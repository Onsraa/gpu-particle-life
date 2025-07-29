use bevy::prelude::*;
use crate::resources::config::simulation::SimulationParameters;
use crate::states::simulation::SimulationState;

pub fn check_epoch_end(
    mut sim_params: ResMut<SimulationParameters>,
    mut next_state: ResMut<NextState<SimulationState>>,
    time: Res<Time>,
) {
    sim_params.tick(time.delta());

    if sim_params.is_epoch_finished() {
        info!("Époque {} terminée!", sim_params.current_epoch);
        sim_params.start_new_epoch();
        next_state.set(SimulationState::Starting);
    }
}

pub fn handle_pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<SimulationState>>,
    mut next_state: ResMut<NextState<SimulationState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        match state.get() {
            SimulationState::Running => {
                info!("Simulation en pause");
                next_state.set(SimulationState::Paused);
            }
            SimulationState::Paused => {
                info!("Reprise de la simulation");
                next_state.set(SimulationState::Running);
            }
            _ => {}
        }
    }
}