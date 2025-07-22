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
use crate::systems::torus_spatial::{ParticleTree, TorusNeighborCache, get_torus_neighbors};

/// Système principal qui gère les itérations physiques avec timestep fixe - AMÉLIORÉ avec torus
pub fn physics_simulation_system(
    sim_params: Res<SimulationParameters>,
    particle_config: Res<ParticleTypesConfig>,
    grid: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
    // NOUVEAU : Systèmes spatiaux
    particle_tree: Res<ParticleTree>,
    torus_cache: Res<TorusNeighborCache>,
    // Queries CORRIGÉES
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
    mut particles: Query<
        (Entity, &mut Transform, &mut Velocity, &ParticleType, &ChildOf),
        With<Particle>,
    >,
    food_query: Query<(&Transform, &ViewVisibility), (With<Food>, Without<Particle>)>,
) {
    // Skip si en pause
    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        return;
    }

    // Calculer le nombre d'itérations selon la vitesse
    let iterations = match sim_params.simulation_speed {
        crate::resources::simulation::SimulationSpeed::Paused => 0,
        crate::resources::simulation::SimulationSpeed::Normal => 1,
        crate::resources::simulation::SimulationSpeed::Fast => 2,
        crate::resources::simulation::SimulationSpeed::VeryFast => 4,
    };

    // BOUCLE PRINCIPALE : chaque itération fait UN pas physique complet
    for _iteration in 0..iterations {
        // 1. Calculer les forces pour cette itération (AMÉLIORÉ avec torus)
        let particle_forces = calculate_forces_with_torus(
            &sim_params,
            &particle_config,
            &grid,
            &boundary_mode,
            &particle_tree,
            &torus_cache,
            &simulations,
            &particles,
            &food_query,
        );

        // 2. Appliquer les forces et le mouvement (inchangé)
        apply_physics_step(
            &grid,
            &boundary_mode,
            &mut particles,
            &particle_forces,
            &sim_params,
        );
    }
}

/// NOUVEAU : Calcule les forces avec support du torus spatial
fn calculate_forces_with_torus(
    sim_params: &SimulationParameters,
    particle_config: &ParticleTypesConfig,
    grid: &GridParameters,
    boundary_mode: &BoundaryMode,
    particle_tree: &ParticleTree,
    torus_cache: &TorusNeighborCache,
    simulations: &Query<(&SimulationId, &Genotype), With<Simulation>>,
    particles: &Query<(Entity, &mut Transform, &mut Velocity, &ParticleType, &ChildOf), With<Particle>>,
    food_query: &Query<(&Transform, &ViewVisibility), (With<Food>, Without<Particle>)>,
) -> std::collections::HashMap<Entity, Vec3> {

    // Cache des génotypes par simulation
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

    // Calculer les forces pour chaque particule
    let mut forces = std::collections::HashMap::new();

    for (entity_a, transform, _, particle_type, parent) in particles.iter() {
        let Ok((sim_id, _)) = simulations.get(parent.parent()) else { continue; };

        let mut total_force = Vec3::ZERO;
        let position = transform.translation;

        if let Some(genotype) = genotypes_cache.get(&sim_id.0) {
            // === FORCES AVEC LES AUTRES PARTICULES (AMÉLIORÉ AVEC TORUS) ===

            // NOUVEAU : Utiliser le système spatial avec support torus
            let neighbors = get_torus_neighbors(
                torus_cache,
                particle_tree,
                sim_id.0,
                entity_a,
                position,
                sim_params.max_force_range,
                *boundary_mode,
            );

            let max_interactions = 100;
            let mut interaction_count = 0;

            for (neighbor_entity, neighbor_pos, _) in neighbors {
                if interaction_count >= max_interactions {
                    break;
                }

                // Récupérer les données du voisin
                let Ok((_, neighbor_transform, _, neighbor_type, neighbor_parent)) =
                    particles.get(neighbor_entity) else { continue; };

                // Vérifier que c'est la même simulation
                let Ok((neighbor_sim_id, _)) = simulations.get(neighbor_parent.parent()) else { continue; };
                if neighbor_sim_id.0 != sim_id.0 {
                    continue;
                }

                // MODIFICATION CRITIQUE : Utiliser la logique de direction torus
                let distance_vec = match *boundary_mode {
                    BoundaryMode::Teleport => {
                        // Utiliser le calcul de direction torus
                        torus_cache.torus_direction_vector(position, neighbor_transform.translation)
                    }
                    BoundaryMode::Bounce => {
                        // Direction normale
                        neighbor_transform.translation - position
                    }
                };

                let distance_squared = distance_vec.dot(distance_vec);

                if distance_squared > sim_params.max_force_range * sim_params.max_force_range ||
                    distance_squared < 0.001 {
                    continue;
                }

                interaction_count += 1;

                let min_r = particle_config.type_count as f32 * PARTICLE_RADIUS;
                let attraction = genotype.get_scaled_force(particle_type.0, neighbor_type.0);
                let acceleration = calculate_acceleration(
                    min_r,
                    distance_vec,
                    attraction,
                    sim_params.max_force_range
                );

                total_force += acceleration * sim_params.max_force_range;
            }

            // === FORCES AVEC LA NOURRITURE (AMÉLIORÉES AVEC TORUS) ===
            let food_force = genotype.get_scaled_food_force(particle_type.0);

            if food_force.abs() > 0.001 {
                for food_pos in &food_positions {
                    // NOUVEAU : Utiliser le calcul de distance/direction torus pour la nourriture aussi
                    let (distance, distance_vec) = match *boundary_mode {
                        BoundaryMode::Teleport => {
                            let dir_vec = torus_cache.torus_direction_vector(position, *food_pos);
                            let dist = dir_vec.length();
                            (dist, dir_vec)
                        }
                        BoundaryMode::Bounce => {
                            let dir_vec = *food_pos - position;
                            let dist = dir_vec.length();
                            (dist, dir_vec)
                        }
                    };

                    if distance > 0.001 && distance < sim_params.max_force_range {
                        let force_direction = distance_vec.normalize();
                        let distance_factor = ((FOOD_RADIUS * 2.0) / distance).min(1.0).powf(0.5);
                        let force_magnitude = food_force * distance_factor;

                        total_force += force_direction * force_magnitude;
                    }
                }
            }
        }

        forces.insert(entity_a, total_force);
    }

    forces
}

/// Applique les forces et le mouvement pour UN pas de temps physique (inchangé)
fn apply_physics_step(
    grid: &GridParameters,
    boundary_mode: &BoundaryMode,
    particles: &mut Query<
        (Entity, &mut Transform, &mut Velocity, &ParticleType, &ChildOf),
        With<Particle>,
    >,
    forces: &std::collections::HashMap<Entity, Vec3>,
    sim_params: &SimulationParameters,
) {
    for (entity, mut transform, mut velocity, _, _) in particles.iter_mut() {
        // Appliquer les forces si disponibles
        if let Some(force) = forces.get(&entity) {
            // Accélération = Force / Masse (masse = 1.0)
            velocity.0 += *force * PHYSICS_TIMESTEP;

            // Amortissement indépendant du framerate
            velocity.0 *= (0.5_f32).powf(PHYSICS_TIMESTEP / sim_params.velocity_half_life);

            // Limiter la vélocité maximale
            if velocity.0.length() > MAX_VELOCITY {
                velocity.0 = velocity.0.normalize() * MAX_VELOCITY;
            }
        }

        // Appliquer la vélocité pour déplacer la particule
        transform.translation += velocity.0 * PHYSICS_TIMESTEP;

        // Gérer les collisions avec les bords
        grid.apply_bounds(&mut transform.translation, &mut velocity.0, *boundary_mode);
    }
}

/// Fonction d'accélération (identique au shader GPU)
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