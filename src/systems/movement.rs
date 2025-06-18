use bevy::prelude::*;

use crate::components::{
    particle::{Particle, ParticleType, Velocity},
    genotype::Genotype,
    simulation::{Simulation, SimulationId},
};
use crate::resources::{
    grid::GridParameters,
    simulation::SimulationParameters,
};
use crate::systems::spatial_grid::SpatialGrid;
use crate::globals::*;

/// Calcule les forces entre particules en utilisant la grille spatiale
pub fn calculate_forces(
    time: Res<Time>,
    sim_params: Res<SimulationParameters>,
    spatial_grid: Res<SpatialGrid>,
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
    mut particles: Query<(Entity, &Transform, &mut Velocity, &ParticleType, &ChildOf), With<Particle>>,
) {
    // Skip si en pause
    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_params.simulation_speed.multiplier();

    // Créer un cache des génotypes par simulation
    let mut genotypes_cache = std::collections::HashMap::new();
    for (sim_id, genotype) in simulations.iter() {
        genotypes_cache.insert(sim_id.0, *genotype);
    }

    // Collecter les données nécessaires pour éviter les conflits
    let particle_data: Vec<_> = particles
        .iter()
        .filter_map(|(entity, transform, _, particle_type, parent)| {
            simulations.get(parent.parent()).ok().map(|(sim_id, _)| {
                (entity, transform.translation, particle_type.0, sim_id.0)
            })
        })
        .collect();

    // Calculer les forces pour chaque particule
    let mut forces = std::collections::HashMap::new();

    for (entity_a, pos_a, type_a, sim_id_a) in &particle_data {
        let mut total_force = Vec3::ZERO;

        if let Some(genotype) = genotypes_cache.get(sim_id_a) {
            // Utiliser la grille spatiale pour trouver les voisins
            let neighbors = spatial_grid.get_potential_neighbors(*pos_a, *sim_id_a);

            for (entity_b, pos_b, type_b) in neighbors {
                if entity_a == &entity_b {
                    continue;
                }

                let distance_vec = pos_b - *pos_a;
                let distance = distance_vec.length();

                // Ignorer si trop loin (double vérification)
                if distance > sim_params.max_force_range {
                    continue;
                }

                let force_direction = if distance > 0.0 {
                    distance_vec.normalize()
                } else {
                    Vec3::ZERO
                };

                // Force génétique
                if distance > MIN_DISTANCE {
                    let genetic_force = genotype.decode_force(*type_a, type_b);
                    let force_magnitude = genetic_force / (distance * distance);
                    total_force += force_direction * force_magnitude;
                }

                // Force de répulsion pour éviter la superposition
                let overlap_distance = PARTICLE_RADIUS * 2.0;
                if distance < overlap_distance && distance > 0.0 {
                    let overlap_amount = (overlap_distance - distance) / overlap_distance;
                    let repulsion_force = -force_direction * PARTICLE_REPULSION_STRENGTH * overlap_amount;
                    total_force += repulsion_force;
                }
            }
        }

        forces.insert(*entity_a, total_force);
    }

    // Appliquer les forces
    for (entity, _, mut velocity, _, _) in particles.iter_mut() {
        if let Some(force) = forces.get(&entity) {
            velocity.0 += *force * delta;

            // Limiter la vélocité maximale
            if velocity.0.length() > MAX_VELOCITY {
                velocity.0 = velocity.0.normalize() * MAX_VELOCITY;
            }
        }
    }
}

/// Applique les vélocités et gère les collisions avec les murs
pub fn apply_movement(
    time: Res<Time>,
    sim_params: Res<SimulationParameters>,
    grid: Res<GridParameters>,
    mut particles: Query<(&mut Transform, &mut Velocity), With<Particle>>,
) {
    // Skip si en pause
    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_params.simulation_speed.multiplier();

    for (mut transform, mut velocity) in particles.iter_mut() {
        // Appliquer la vélocité
        transform.translation += velocity.0 * delta;

        // Gérer les rebonds sur les murs
        grid.apply_bounds(&mut transform.translation, &mut velocity.0);
    }
}