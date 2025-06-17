use bevy::prelude::*;
type Genome = u32;

#[derive(Component, Default)]
pub struct Genotype(Genome);
