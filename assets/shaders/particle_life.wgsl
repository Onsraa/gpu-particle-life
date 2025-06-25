// Constantes
const PARTICLE_RADIUS: f32 = 2.5;
const FOOD_RADIUS: f32 = 1.0;
const MIN_DISTANCE: f32 = 0.001;
const PARTICLE_REPULSION_STRENGTH: f32 = 100.0;
const FORCE_SCALE_FACTOR: f32 = 1000.0;
const MAX_VELOCITY: f32 = 200.0;
const PARTICLE_MASS: f32 = 1.0;
const DAMPING: f32 = 0.99;

// Structure pour une particule
struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    particle_type: u32,
    simulation_id: u32,
}

// Structure pour les paramètres de simulation
struct SimulationParams {
    delta_time: f32,
    particle_count: u32,
    simulation_count: u32,
    type_count: u32,
    max_force_range: f32,
    grid_width: f32,
    grid_height: f32,
    grid_depth: f32,
    boundary_mode: u32, // 0 = bounce, 1 = teleport
}

// Structure pour la nourriture
struct Food {
    position: vec3<f32>,
    is_active: u32, // 1 si active, 0 si mangée
}

// Buffers
@group(0) @binding(0) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(2) var<uniform> params: SimulationParams;
@group(0) @binding(3) var<storage, read> genomes: array<u32>; // [genome_low, genome_high, food_genome_low, padding] par simulation
@group(0) @binding(4) var<storage, read> food_positions: array<Food>;
@group(0) @binding(5) var<uniform> food_count: u32;

// Décode une force depuis le génome
fn decode_force(genome_low: u32, genome_high: u32, type_a: u32, type_b: u32, type_count: u32) -> f32 {
    let genome = (u64(genome_high) << 32u) | u64(genome_low);
    let interactions = type_count * type_count;
    let bits_per_interaction = 64u / max(interactions, 1u);

    let index = type_a * type_count + type_b;
    let bit_start = index * bits_per_interaction;

    if (bit_start >= 64u) {
        return 0.0;
    }

    // Extraire les bits
    let mask = (1u << bits_per_interaction) - 1u;
    let raw_value = u32((genome >> bit_start) & u64(mask));

    // Normaliser entre -1 et 1
    let max_value = f32((1u << bits_per_interaction) - 1u);
    let normalized = (f32(raw_value) / max_value) * 2.0 - 1.0;

    return normalized * FORCE_SCALE_FACTOR;
}

// Décode la force de nourriture depuis le génome
fn decode_food_force(food_genome: u32, particle_type: u32, type_count: u32) -> f32 {
    // Les 16 bits du food_genome sont répartis entre les types
    let bits_per_type = 16u / max(type_count, 1u);
    let bit_start = particle_type * bits_per_type;

    if (bit_start >= 16u) {
        return 0.0;
    }

    // Extraire les bits
    let mask = (1u << bits_per_type) - 1u;
    let raw_value = (food_genome >> bit_start) & mask;

    // Normaliser entre -1 et 1
    let max_value = f32((1u << bits_per_type) - 1u);
    let normalized = (f32(raw_value) / max_value) * 2.0 - 1.0;

    return normalized * FORCE_SCALE_FACTOR;
}

// Applique les limites avec rebond
fn apply_bounce_bounds(position: ptr<function, vec3<f32>>, velocity: ptr<function, vec3<f32>>) {
    let half_width = params.grid_width / 2.0;
    let half_height = params.grid_height / 2.0;
    let half_depth = params.grid_depth / 2.0;

    // X bounds
    if (abs((*position).x) > half_width - PARTICLE_RADIUS) {
        (*position).x = sign((*position).x) * (half_width - PARTICLE_RADIUS);
        (*velocity).x *= -0.5;
    }

    // Y bounds
    if (abs((*position).y) > half_height - PARTICLE_RADIUS) {
        (*position).y = sign((*position).y) * (half_height - PARTICLE_RADIUS);
        (*velocity).y *= -0.5;
    }

    // Z bounds
    if (abs((*position).z) > half_depth - PARTICLE_RADIUS) {
        (*position).z = sign((*position).z) * (half_depth - PARTICLE_RADIUS);
        (*velocity).z *= -0.5;
    }
}

// Applique les limites avec téléportation
fn apply_teleport_bounds(position: ptr<function, vec3<f32>>) {
    let half_width = params.grid_width / 2.0;
    let half_height = params.grid_height / 2.0;
    let half_depth = params.grid_depth / 2.0;

    // X teleport
    if ((*position).x > half_width) {
        (*position).x = -half_width + ((*position).x - half_width);
    } else if ((*position).x < -half_width) {
        (*position).x = half_width + ((*position).x + half_width);
    }

    // Y teleport
    if ((*position).y > half_height) {
        (*position).y = -half_height + ((*position).y - half_height);
    } else if ((*position).y < -half_height) {
        (*position).y = half_height + ((*position).y + half_height);
    }

    // Z teleport
    if ((*position).z > half_depth) {
        (*position).z = -half_depth + ((*position).z - half_depth);
    } else if ((*position).z < -half_depth) {
        (*position).z = half_depth + ((*position).z + half_depth);
    }
}

@compute @workgroup_size(64, 1, 1)
fn update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var particle = particles_in[index];
    var total_force = vec3<f32>(0.0, 0.0, 0.0);

    // Récupérer le génome de cette simulation
    let genome_idx = particle.simulation_id * 4u; // 4 u32 par simulation
    let genome_low = genomes[genome_idx];
    let genome_high = genomes[genome_idx + 1u];
    let food_genome = genomes[genome_idx + 2u];

    // === Forces avec les autres particules ===
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == index) {
            continue;
        }

        let other = particles_in[i];

        // Ignorer les particules d'autres simulations
        if (other.simulation_id != particle.simulation_id) {
            continue;
        }

        let distance_vec = other.position - particle.position;
        let distance = length(distance_vec);

        // Ignorer si trop loin
        if (distance > params.max_force_range || distance < MIN_DISTANCE) {
            continue;
        }

        let force_direction = normalize(distance_vec);

        // Force de répulsion pour éviter la superposition
        let overlap_distance = PARTICLE_RADIUS * 2.0;
        if (distance < overlap_distance) {
            let overlap_amount = (overlap_distance - distance) / overlap_distance;
            let repulsion_force = -force_direction * PARTICLE_REPULSION_STRENGTH * overlap_amount * overlap_amount;
            total_force += repulsion_force;
        }

        // Force génétique
        if (distance > PARTICLE_RADIUS) {
            let genetic_force = decode_force(genome_low, genome_high, particle.particle_type, other.particle_type, params.type_count);
            let distance_factor = min(PARTICLE_RADIUS / distance, 1.0);
            let force_magnitude = genetic_force * distance_factor;
            total_force += force_direction * force_magnitude;
        }
    }

    // === Forces avec la nourriture ===
    let particle_food_force = decode_food_force(food_genome, particle.particle_type, params.type_count);

    if (abs(particle_food_force) > 0.001) {
        for (var i = 0u; i < food_count; i++) {
            let food = food_positions[i];

            // Ignorer la nourriture inactive
            if (food.is_active == 0u) {
                continue;
            }

            let distance_vec = food.position - particle.position;
            let distance = length(distance_vec);

            // Appliquer la force si dans la portée
            if (distance > MIN_DISTANCE && distance < params.max_force_range) {
                let force_direction = normalize(distance_vec);

                // Atténuation plus douce pour la nourriture
                let distance_factor = pow(min((FOOD_RADIUS * 2.0) / distance, 1.0), 0.5);
                let force_magnitude = particle_food_force * distance_factor;

                total_force += force_direction * force_magnitude;
            }
        }
    }

    // Appliquer les forces (F = ma => a = F/m)
    let acceleration = total_force / PARTICLE_MASS;
    particle.velocity += acceleration * params.delta_time;

    // Amortissement
    particle.velocity *= DAMPING;

    // Limiter la vitesse
    let speed = length(particle.velocity);
    if (speed > MAX_VELOCITY) {
        particle.velocity = normalize(particle.velocity) * MAX_VELOCITY;
    }

    // Appliquer la vélocité
    particle.position += particle.velocity * params.delta_time;

    // Appliquer les limites
    if (params.boundary_mode == 0u) {
        apply_bounce_bounds(&particle.position, &particle.velocity);
    } else {
        apply_teleport_bounds(&particle.position);
    }

    // Écrire le résultat
    particles_out[index] = particle;
}