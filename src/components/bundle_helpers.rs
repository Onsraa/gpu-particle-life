use bevy::prelude::*;
use crate::components::{
    simulation::{SimulationBundle},
    particle::{ParticleBundle},
    genotype::Genotype,
};

/// Crée un bundle de simulation avec toutes ses particules comme enfants
pub fn simulation_with_particles(
    sim_id: usize,
    genotype: Genotype,
    particles: Vec<ParticleBundle>,
) -> impl Bundle {
    (
        SimulationBundle::new(sim_id, genotype),
        children![particles],
    )
}