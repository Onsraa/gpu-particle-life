use bevy::prelude::*;

use crate::components::{
    genotype::Genotype,
    particle::{Particle, ParticleType, Velocity},
    simulation::{Simulation, SimulationId},
    food::Food,
};
use crate::globals::*;
use crate::resources::boundary::BoundaryMode;
use crate::resources::{grid::GridParameters, simulation::SimulationParameters};
use crate::resources::particle_types::ParticleTypesConfig;
use crate::systems::spatial_grid::SpatialGrid;

/// Calcule les forces entre particules et avec la nourriture
pub fn calculate_forces(
    sim_params: Res<SimulationParameters>,
    particle_config: Res<ParticleTypesConfig>,
    spatial_grid: Res<SpatialGrid>,
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
    mut particles: Query<
        (Entity, &Transform, &mut Velocity, &ParticleType, &ChildOf),
        With<Particle>,
    >,
    food_query: Query<(&Transform, &ViewVisibility), With<Food>>,
) {
    // Skip si en pause
    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        return;
    }

    // IMPORTANT: Utiliser un delta fixe pour la physique, pas le delta frame
    let delta = 0.016; // 60 FPS fixe pour la stabilité de la physique

    // Créer un cache des génotypes par simulation
    let mut genotypes_cache = std::collections::HashMap::new();
    for (sim_id, genotype) in simulations.iter() {
        genotypes_cache.insert(sim_id.0, *genotype);
    }

    // Collecter les positions de nourriture visible
    let food_positions: Vec<Vec3> = food_query
        .iter()
        .filter(|(_, visibility)| visibility.get())
        .map(|(transform, _)| transform.translation)
        .collect();

    // Collecter les données nécessaires pour éviter les conflits
    let particle_data: Vec<_> = particles
        .iter()
        .filter_map(|(entity, transform, _, particle_type, parent)| {
            simulations
                .get(parent.parent())
                .ok()
                .map(|(sim_id, _)| (entity, transform.translation, particle_type.0, sim_id.0))
        })
        .collect();

    // Calculer les forces pour chaque particule
    let mut forces = std::collections::HashMap::new();

    for (entity_a, pos_a, type_a, sim_id_a) in &particle_data {
        let mut total_force = Vec3::ZERO;

        if let Some(genotype) = genotypes_cache.get(sim_id_a) {
            // === FORCES AVEC LES AUTRES PARTICULES ===
            // Utiliser la grille spatiale pour trouver les voisins
            let neighbors = spatial_grid.get_potential_neighbors(*pos_a, *sim_id_a);

            // Limiter le nombre d'interactions (comme dans le GPU)
            let max_interactions = 100;
            let mut interaction_count = 0;

            for (entity_b, pos_b, type_b) in neighbors {
                if entity_a == &entity_b || interaction_count >= max_interactions {
                    continue;
                }

                let distance_vec = pos_b - *pos_a;
                let distance_squared = distance_vec.dot(distance_vec);

                // Ignorer si trop loin ou trop proche
                if distance_squared > sim_params.max_force_range * sim_params.max_force_range ||
                    distance_squared < 0.001 {
                    continue;
                }

                interaction_count += 1;

                // Utiliser la même fonction d'accélération que le shader GPU
                let min_r = particle_config.type_count as f32 * PARTICLE_RADIUS;
                let attraction = genotype.decode_force(*type_a, type_b);
                let acceleration = calculate_acceleration(min_r, distance_vec, attraction, sim_params.max_force_range);

                total_force += acceleration * sim_params.max_force_range;
            }

            // === FORCES AVEC LA NOURRITURE ===
            let food_force = genotype.decode_food_force(*type_a);

            // Si la force de nourriture n'est pas nulle, calculer l'interaction
            if food_force.abs() > 0.001 {
                for food_pos in &food_positions {
                    let distance_vec = *food_pos - *pos_a;
                    let distance = distance_vec.length();

                    // Appliquer la force si dans la portée
                    if distance > 0.001 && distance < sim_params.max_force_range {
                        let force_direction = distance_vec.normalize();

                        // Atténuation en fonction de la distance
                        let distance_factor = ((FOOD_RADIUS * 2.0) / distance).min(1.0).powf(0.5);
                        let force_magnitude = food_force * distance_factor;

                        total_force += force_direction * force_magnitude;
                    }
                }
            }
        }

        forces.insert(*entity_a, total_force);
    }

    // Appliquer les forces
    for (entity, _, mut velocity, _, _) in particles.iter_mut() {
        if let Some(force) = forces.get(&entity) {
            // Appliquer l'accélération
            velocity.0 += *force * delta;

            // CORRECTION: Amortissement indépendant du framerate (comme dans le shader)
            velocity.0 *= (0.5_f32).powf(delta / sim_params.velocity_half_life);

            // Limiter la vélocité maximale
            if velocity.0.length() > MAX_VELOCITY {
                velocity.0 = velocity.0.normalize() * MAX_VELOCITY;
            }
        }
    }
}

/// Calcule l'accélération entre deux particules (identique au shader GPU)
fn calculate_acceleration(min_r: f32, relative_pos: Vec3, attraction: f32, max_force_range: f32) -> Vec3 {
    let dist = relative_pos.length();
    if dist < 0.001 {
        return Vec3::ZERO;
    }

    let normalized_pos = relative_pos / max_force_range;
    let normalized_dist = dist / max_force_range;
    let min_r_normalized = min_r / max_force_range;

    let force = if normalized_dist < min_r_normalized {
        // Force de répulsion (toujours négative)
        (normalized_dist / min_r_normalized - 1.0)
    } else {
        // Force d'attraction/répulsion basée sur le génome
        attraction * (1.0 - (1.0 + min_r_normalized - 2.0 * normalized_dist).abs() / (1.0 - min_r_normalized))
    };

    normalized_pos * force / normalized_dist
}

/// Applique les vélocités et gère les collisions avec les murs
pub fn apply_movement(
    sim_params: Res<SimulationParameters>,
    grid: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
    mut particles: Query<(&mut Transform, &mut Velocity), With<Particle>>,
) {
    // Skip si en pause
    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        return;
    }

    // Calculer le nombre d'itérations basé sur la vitesse
    let iterations = match sim_params.simulation_speed {
        crate::resources::simulation::SimulationSpeed::Paused => 0,
        crate::resources::simulation::SimulationSpeed::Normal => 1,
        crate::resources::simulation::SimulationSpeed::Fast => 2,
        crate::resources::simulation::SimulationSpeed::VeryFast => 4,
    };

    let base_delta = 0.016; // 60 FPS fixe

    // Appliquer le mouvement plusieurs fois pour accélérer la simulation
    for _ in 0..iterations {
        for (mut transform, mut velocity) in particles.iter_mut() {
            // Appliquer la vélocité
            transform.translation += velocity.0 * base_delta;

            // Gérer les bords selon le mode
            grid.apply_bounds(&mut transform.translation, &mut velocity.0, *boundary_mode);
        }
    }
}