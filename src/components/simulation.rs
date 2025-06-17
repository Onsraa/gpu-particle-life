use crate::components::score::Score;
use crate::components::genotype::Genotype;
use bevy::prelude::*;

#[derive(Component)]
#[require(Genotype, Score)]
pub struct Simulation;