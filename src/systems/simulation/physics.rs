use crate::components::entities::food::Food;
use crate::components::entities::particle::{Particle, ParticleType, Velocity};
use crate::components::entities::simulation::{Simulation, SimulationId};
use crate::components::genetics::genotype::Genotype;
use crate::globals::*;
use crate::resources::config::simulation::{SimulationParameters, SimulationSpeed};
use crate::resources::world::boundary::BoundaryMode;
use crate::resources::world::grid::GridParameters;
use bevy::prelude::*;

pub fn physics_simulation_system(
    sim_params: Res<SimulationParameters>,
    grid: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
    mut particles: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &ParticleType,
            &ChildOf,
        ),
        With<Particle>,
    >,
    food_query: Query<(&Transform, &ViewVisibility), (With<Food>, Without<Particle>)>,
) {
    if sim_params.simulation_speed == SimulationSpeed::Paused {
        return;
    }

    let iterations = match sim_params.simulation_speed {
        SimulationSpeed::Paused => 0,
        SimulationSpeed::Normal => 1,
        SimulationSpeed::Fast => 2,
        SimulationSpeed::VeryFast => 4,
    };

    for _iteration in 0..iterations {
        let particle_forces = calculate_forces(
            &sim_params,
            &grid,
            &boundary_mode,
            &simulations,
            &particles,
            &food_query,
        );

        apply_physics_step(
            &grid,
            &boundary_mode,
            &mut particles,
            &particle_forces,
            &sim_params,
        );
    }
}

fn calculate_forces(
    sim_params: &SimulationParameters,
    grid: &GridParameters,
    boundary_mode: &BoundaryMode,
    simulations: &Query<(&SimulationId, &Genotype), With<Simulation>>,
    particles: &Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &ParticleType,
            &ChildOf,
        ),
        With<Particle>,
    >,
    food_query: &Query<(&Transform, &ViewVisibility), (With<Food>, Without<Particle>)>,
) -> std::collections::HashMap<Entity, Vec3> {
    let mut genotypes_cache = std::collections::HashMap::new();
    for (sim_id, genotype) in simulations.iter() {
        genotypes_cache.insert(sim_id.0, genotype);
    }

    let food_positions: Vec<Vec3> = food_query
        .iter()
        .filter(|(_, visibility)| visibility.get())
        .map(|(transform, _)| transform.translation)
        .collect();

    let mut forces = std::collections::HashMap::new();

    for (entity_a, transform, _, particle_type, parent) in particles.iter() {
        let Ok((sim_id, _)) = simulations.get(parent.parent()) else {
            continue;
        };

        let mut total_force = Vec3::ZERO;
        let position = transform.translation;

        if let Some(genotype) = genotypes_cache.get(&sim_id.0) {
            // Forces avec autres particules
            let mut interaction_count = 0;
            for (entity_b, other_transform, _, other_type, other_parent) in particles.iter() {
                if entity_a == entity_b || interaction_count >= 100 {
                    continue;
                }

                let Ok((other_sim_id, _)) = simulations.get(other_parent.parent()) else {
                    continue;
                };
                if other_sim_id.0 != sim_id.0 {
                    continue;
                }

                let distance_vec = match *boundary_mode {
                    BoundaryMode::Teleport => {
                        torus_direction_vector(position, other_transform.translation, grid)
                    }
                    BoundaryMode::Bounce => other_transform.translation - position,
                };

                let distance_squared = distance_vec.dot(distance_vec);
                if distance_squared > sim_params.max_force_range * sim_params.max_force_range
                    || distance_squared < 0.001
                {
                    continue;
                }

                interaction_count += 1;

                let min_r = sim_params.particle_types as f32 * PARTICLE_RADIUS;
                let attraction =
                    genotype.get_force(particle_type.0, other_type.0) * FORCE_SCALE_FACTOR;
                let acceleration = calculate_acceleration(
                    min_r,
                    distance_vec,
                    attraction,
                    sim_params.max_force_range,
                );

                total_force += acceleration * sim_params.max_force_range;
            }

            // Forces avec nourriture
            let food_force = genotype.get_food_force(particle_type.0) * FORCE_SCALE_FACTOR;
            if food_force.abs() > 0.001 {
                for food_pos in &food_positions {
                    let distance_vec = match *boundary_mode {
                        BoundaryMode::Teleport => torus_direction_vector(position, *food_pos, grid),
                        BoundaryMode::Bounce => *food_pos - position,
                    };

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

        forces.insert(entity_a, total_force);
    }

    forces
}

fn apply_physics_step(
    grid: &GridParameters,
    boundary_mode: &BoundaryMode,
    particles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &ParticleType,
            &ChildOf,
        ),
        With<Particle>,
    >,
    forces: &std::collections::HashMap<Entity, Vec3>,
    sim_params: &SimulationParameters,
) {
    for (entity, mut transform, mut velocity, _, _) in particles.iter_mut() {
        if let Some(force) = forces.get(&entity) {
            velocity.0 += *force * PHYSICS_TIMESTEP;
            velocity.0 *= (0.5_f32).powf(PHYSICS_TIMESTEP / sim_params.velocity_half_life);

            if velocity.0.length() > MAX_VELOCITY {
                velocity.0 = velocity.0.normalize() * MAX_VELOCITY;
            }
        }

        transform.translation += velocity.0 * PHYSICS_TIMESTEP;
        grid.apply_bounds(&mut transform.translation, &mut velocity.0, *boundary_mode);
    }
}

fn calculate_acceleration(
    min_r: f32,
    relative_pos: Vec3,
    attraction: f32,
    max_force_range: f32,
) -> Vec3 {
    let dist = relative_pos.length();
    if dist < 0.001 {
        return Vec3::ZERO;
    }

    let normalized_pos = relative_pos / max_force_range;
    let normalized_dist = dist / max_force_range;
    let min_r_normalized = min_r / max_force_range;

    let force = if normalized_dist < min_r_normalized {
        (normalized_dist / min_r_normalized - 1.0)
    } else {
        attraction
            * (1.0
                - (1.0 + min_r_normalized - 2.0 * normalized_dist).abs() / (1.0 - min_r_normalized))
    };

    normalized_pos * force / normalized_dist
}

fn torus_direction_vector(from: Vec3, to: Vec3, grid: &GridParameters) -> Vec3 {
    let mut direction = Vec3::ZERO;

    let dx = to.x - from.x;
    if dx.abs() <= grid.width / 2.0 {
        direction.x = dx;
    } else {
        direction.x = if dx > 0.0 {
            dx - grid.width
        } else {
            dx + grid.width
        };
    }

    let dy = to.y - from.y;
    if dy.abs() <= grid.height / 2.0 {
        direction.y = dy;
    } else {
        direction.y = if dy > 0.0 {
            dy - grid.height
        } else {
            dy + grid.height
        };
    }

    let dz = to.z - from.z;
    if dz.abs() <= grid.depth / 2.0 {
        direction.z = dz;
    } else {
        direction.z = if dz > 0.0 {
            dz - grid.depth
        } else {
            dz + grid.depth
        };
    }

    direction
}
