use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::{
    particle::{Particle, ParticleType},
    simulation::{Simulation, SimulationId},
};
use crate::globals::*;

/// Taille d'une cellule de la grille spatiale
pub const CELL_SIZE: f32 = DEFAULT_MAX_FORCE_RANGE / 2.0;

/// Structure pour stocker les particules dans une grille spatiale
#[derive(Resource, Default)]
pub struct SpatialGrid {
    /// Map de (simulation_id, cell_key) -> Vec<(Entity, position, type)>
    pub cells: HashMap<(usize, IVec3), Vec<(Entity, Vec3, usize)>>, // CHANGEMENT : rendu public
}

impl SpatialGrid {
    /// Calcule la clé de cellule pour une position
    pub fn get_cell_key(position: Vec3) -> IVec3 {
        IVec3::new(
            (position.x / CELL_SIZE).floor() as i32,
            (position.y / CELL_SIZE).floor() as i32,
            (position.z / CELL_SIZE).floor() as i32,
        )
    }

    /// Retourne toutes les cellules voisines (incluant la cellule actuelle)
    fn get_neighbor_cells(cell: IVec3) -> Vec<IVec3> {
        let mut neighbors = Vec::with_capacity(27);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    neighbors.push(cell + IVec3::new(dx, dy, dz));
                }
            }
        }
        neighbors
    }

    /// Reconstruit la grille spatiale (méthode originale)
    pub fn rebuild(
        &mut self,
        particles: &Query<(Entity, &Transform, &ParticleType, &ChildOf), With<Particle>>,
        simulations: &Query<&SimulationId>,
    ) {
        self.cells.clear();

        for (entity, transform, particle_type, parent) in particles.iter() {
            if let Ok(sim_id) = simulations.get(parent.parent()) {
                let cell_key = Self::get_cell_key(transform.translation);
                let key = (sim_id.0, cell_key);

                self.cells
                    .entry(key)
                    .or_default()
                    .push((entity, transform.translation, particle_type.0));
            }
        }
    }

    /// Trouve les voisins potentiels d'une particule
    pub fn get_potential_neighbors(
        &self,
        position: Vec3,
        simulation_id: usize,
    ) -> Vec<(Entity, Vec3, usize)> {
        let cell = Self::get_cell_key(position);
        let neighbor_cells = Self::get_neighbor_cells(cell);

        let mut neighbors = Vec::new();
        for neighbor_cell in neighbor_cells {
            let key = (simulation_id, neighbor_cell);
            if let Some(particles) = self.cells.get(&key) {
                neighbors.extend_from_slice(particles);
            }
        }

        neighbors
    }
}

/// Système pour reconstruire la grille spatiale (système original)
pub fn update_spatial_grid(
    mut spatial_grid: ResMut<SpatialGrid>,
    particles: Query<(Entity, &Transform, &ParticleType, &ChildOf), With<Particle>>,
    simulations: Query<&SimulationId>,
) {
    spatial_grid.rebuild(&particles, &simulations);
}