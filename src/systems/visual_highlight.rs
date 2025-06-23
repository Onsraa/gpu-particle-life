use crate::components::simulation::{Simulation, SimulationId};
use crate::ui::force_matrix::ForceMatrixUI;
use bevy::prelude::*;

/// Système pour mettre en évidence visuellement la simulation sélectionnée
pub fn highlight_selected_simulation(
    ui_state: Res<ForceMatrixUI>,
    mut simulations: Query<(&SimulationId, &Children), With<Simulation>>,
    mut particles: Query<&mut Transform, With<crate::components::particle::Particle>>,
) {
    // Parcourir toutes les simulations
    for (sim_id, children) in simulations.iter_mut() {
        let is_selected = sim_id.0 == ui_state.selected_simulation.unwrap();

        // Appliquer un effet visuel aux particules de la simulation
        for child in children.iter() {
            if let Ok(mut transform) = particles.get_mut(child) {
                // Augmenter légèrement la taille des particules de la simulation sélectionnée
                if is_selected {
                    transform.scale = Vec3::splat(1.2);
                } else {
                    transform.scale = Vec3::splat(1.0);
                }
            }
        }
    }
}
