use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::{
    particle::{Particle, ParticleType, Velocity},
    genotype::Genotype,
    simulation::Simulation,
};
use crate::resources::{
    grid::GridParameters,
    simulation::SimulationParameters,
};
use crate::globals::*;

/// Structure pour stocker les données d'une particule pour le calcul
#[derive(Clone)]
struct ParticleData {
    entity: Entity,
    position: Vec3,
    particle_type: usize,
    simulation_entity: Entity,
}

/// Calcule les forces entre particules et met à jour les vélocités
pub fn calculate_forces(
    time: Res<Time>,
    sim_params: Res<SimulationParameters>,
    simulations: Query<(Entity, &Genotype, &Children), With<Simulation>>,
    mut particles: Query<(Entity, &Transform, &mut Velocity, &ParticleType, &Parent), With<Particle>>,
) {
    // Skip si en pause
    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_params.simulation_speed.multiplier();

    // Créer un cache des génotypes par entité de simulation
    let genotypes: HashMap<Entity, Genotype> = simulations
        .iter()
        .map(|(entity, genotype, _)| (entity, *genotype))
        .collect();

    // Collecter toutes les données de particules
    let mut particle_data_by_sim: HashMap<Entity, Vec<ParticleData>> = HashMap::new();

    for (entity, transform, _, particle_type, parent) in particles.iter() {
        let data = ParticleData {
            entity,
            position: transform.translation,
            particle_type: particle_type.0,
            simulation_entity: parent.get(),
        };

        particle_data_by_sim
            .entry(parent.get())
            .or_default()
            .push(data);
    }

    // Calculer les forces pour chaque simulation
    for (sim_entity, particle_list) in &particle_data_by_sim {
        if let Some(genotype) = genotypes.get(sim_entity) {
            // Pour chaque particule de cette simulation
            for (i, particle_a) in particle_list.iter().enumerate() {
                let mut total_force = Vec3::ZERO;

                // Calculer les forces avec toutes les autres particules de la même simulation
                for (j, particle_b) in particle_list.iter().enumerate() {
                    if i == j {
                        continue;
                    }

                    let distance_vec = particle_b.position - particle_a.position;
                    let distance = distance_vec.length();

                    // Ignorer si trop loin
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
                        let genetic_force = genotype.decode_force(
                            particle_a.particle_type,
                            particle_b.particle_type
                        );

                        // Force inversement proportionnelle au carré de la distance
                        let force_magnitude = genetic_force / (distance * distance);
                        total_force += force_direction * force_magnitude;
                    }

                    // Force de répulsion pour éviter la superposition
                    let overlap_distance = PARTICLE_RADIUS * 2.0;
                    if distance < overlap_distance && distance > 0.0 {
                        // Force de répulsion forte quand les particules se chevauchent
                        let overlap_amount = (overlap_distance - distance) / overlap_distance;
                        let repulsion_force = -force_direction * PARTICLE_REPULSION_STRENGTH * overlap_amount;
                        total_force += repulsion_force;
                    }
                }

                // Appliquer la force à la vélocité
                if let Ok((_, _, mut velocity, _, _)) = particles.get_mut(particle_a.entity) {
                    // F = ma, avec m = 1, donc a = F
                    velocity.0 += total_force * delta;

                    // Limiter la vélocité maximale
                    if velocity.0.length() > MAX_VELOCITY {
                        velocity.0 = velocity.0.normalize() * MAX_VELOCITY;
                    }
                }
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