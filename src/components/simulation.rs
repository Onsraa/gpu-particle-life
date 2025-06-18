use bevy::prelude::*;
use crate::components::score::Score;
use crate::components::genotype::Genotype;

/// ID de la simulation
#[derive(Component, Default)]
pub struct SimulationId(pub usize);

/// Marqueur pour une simulation
#[derive(Component)]
#[require(SimulationId, Genotype, Score, Transform, Visibility, InheritedVisibility, ViewVisibility)]
pub struct Simulation;