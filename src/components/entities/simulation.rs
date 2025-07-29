use bevy::prelude::*;
use crate::components::genetics::genotype::*;
use crate::components::genetics::score::*;

/// ID de la simulation
#[derive(Component, Default)]
pub struct SimulationId(pub usize);

/// Marqueur pour une simulation
#[derive(Component)]
#[require(SimulationId, Genotype, Score, Transform, Visibility, InheritedVisibility, ViewVisibility)]
pub struct Simulation;