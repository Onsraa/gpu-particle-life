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

/// Système principal qui gère les itérations physiques avec timestep fixe
pub fn physics_simulation_system(
    sim_params: Res<SimulationParameters>,
    particle_config: Res<ParticleTypesConfig>,
    mut spatial_grid: ResMut<SpatialGrid>, // Mutable pour mise à jour multiple
    grid: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
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

    // Calculer le nombre d'itérations physiques selon la vitesse de simulation
    let iterations = match sim_params.simulation_speed {
        crate::resources::simulation::SimulationSpeed::Paused => 0,
        crate::resources::simulation::SimulationSpeed::Normal => 1,
        crate::resources::simulation::SimulationSpeed::Fast => 2,
        crate::resources::simulation::SimulationSpeed::VeryFast => 4,
    };

    // BOUCLE PRINCIPALE : chaque itération fait UN pas physique complet
    for _iteration in 0..iterations {
        // 1. Mettre à jour la grille spatiale avec les positions actuelles
        // CORRECTION : Créer une query compatible pour rebuild()
        rebuild_spatial_grid(&mut spatial_grid, &particles, &simulations);

        // 2. Calculer les forces pour cette itération
        let particle_forces = calculate_forces_for_timestep(
            &sim_params,
            &particle_config,
            &spatial_grid,
            &simulations,
            &particles,
            &food_query,
        );

        // 3. Appliquer les forces et le mouvement pour cette itération
        apply_physics_step(
            &grid,
            &boundary_mode,
            &mut particles,
            &particle_forces,
            &sim_params,
        );
    }
}

/// CORRECTION : Fonction helper pour rebuild avec les bons types
fn rebuild_spatial_grid(
    spatial_grid: &mut SpatialGrid,
    particles: &Query<(Entity, &mut Transform, &mut Velocity, &ParticleType, &ChildOf), With<Particle>>,
    simulations: &Query<(&SimulationId, &Genotype), With<Simulation>>,
) {
    spatial_grid.cells.clear(); // Accès direct au champ cells

    // Reconstruire manuellement avec les données disponibles
    for (entity, transform, _, particle_type, parent) in particles.iter() {
        if let Ok((sim_id, _)) = simulations.get(parent.parent()) {
            let cell_key = SpatialGrid::get_cell_key(transform.translation);
            let key = (sim_id.0, cell_key);

            spatial_grid.cells
                .entry(key)
                .or_default()
                .push((entity, transform.translation, particle_type.0));
        }
    }
}

/// Calcule les forces pour UN pas de temps physique
fn calculate_forces_for_timestep(
    sim_params: &SimulationParameters,
    particle_config: &ParticleTypesConfig,
    spatial_grid: &SpatialGrid,
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

    // Collecter les données des particules pour éviter les conflits
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
            let neighbors = spatial_grid.get_potential_neighbors(*pos_a, *sim_id_a);
            let max_interactions = 100;
            let mut interaction_count = 0;

            for (entity_b, pos_b, type_b) in neighbors {
                if entity_a == &entity_b || interaction_count >= max_interactions {
                    continue;
                }

                let distance_vec = pos_b - *pos_a;
                let distance_squared = distance_vec.dot(distance_vec);

                if distance_squared > sim_params.max_force_range * sim_params.max_force_range ||
                    distance_squared < 0.001 {
                    continue;
                }

                interaction_count += 1;

                let min_r = particle_config.type_count as f32 * PARTICLE_RADIUS;
                let attraction = genotype.get_scaled_force(*type_a, type_b);
                let acceleration = calculate_acceleration(
                    min_r,
                    distance_vec,
                    attraction,
                    sim_params.max_force_range
                );

                total_force += acceleration * sim_params.max_force_range;
            }

            // === FORCES AVEC LA NOURRITURE ===
            let food_force = genotype.get_scaled_food_force(*type_a);

            if food_force.abs() > 0.001 {
                for food_pos in &food_positions {
                    let distance_vec = *food_pos - *pos_a;
                    let distance = distance_vec.length();

                    if distance > 0.001 && distance < sim_params.max_force_range {
                        let force_direction = distance_vec.normalize();
                        let distance_factor = ((FOOD_RADIUS * 2.0) / distance).min(1.0).powf(0.5);
                        let force_magnitude = food_force * distance_factor;

                        total_force += force_direction * force_magnitude;
                    }
                }
            }
        }

        forces.insert(*entity_a, total_force);
    }

    forces
}

/// Applique les forces et le mouvement pour UN pas de temps physique
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