// Constantes
const PARTICLE_RADIUS: f32 = 2.5;
const FOOD_RADIUS: f32 = 1.0;
const MIN_DISTANCE: f32 = 0.001;
const PARTICLE_REPULSION_STRENGTH: f32 = 100.0;
const FORCE_SCALE_FACTOR: f32 = 80.0; // NOUVEAU FACTEUR
const MAX_VELOCITY: f32 = 200.0;
const PARTICLE_MASS: f32 = 1.0;
const VELOCITY_HALF_LIFE: f32 = 0.043;
const MAX_INTERACTIONS_PER_PARTICLE: u32 = 100;

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
    min_distance: f32,
    grid_width: f32,
    grid_height: f32,
    grid_depth: f32,
    boundary_mode: u32,
}

// Structure pour la nourriture
struct Food {
    position: vec3<f32>,
    is_active: u32,
}

// Buffers séparés pour éviter les race conditions
@group(0) @binding(0) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(2) var<uniform> params: SimulationParams;
@group(0) @binding(3) var<storage, read> genomes: array<u32>;
@group(0) @binding(4) var<storage, read> food_positions: array<Food>;
@group(0) @binding(5) var<uniform> food_count: u32;

// Décode une force depuis le génome (retourne une valeur normalisée entre -1 et 1)
fn decode_force(genome_low: u32, genome_high: u32, type_a: u32, type_b: u32, type_count: u32) -> f32 {
    let genome = (u64(genome_high) << 32u) | u64(genome_low);
    let interactions = type_count * type_count;
    let bits_per_interaction = max(64u / max(interactions, 1u), 2u);

    let index = type_a * type_count + type_b;
    let bit_start = index * bits_per_interaction;

    if (bit_start >= 64u || bit_start + bits_per_interaction > 64u) {
        return 0.0;
    }

    let mask = (1u << bits_per_interaction) - 1u;
    let raw_value = u32((genome >> bit_start) & u64(mask));

    let max_value = f32((1u << bits_per_interaction) - 1u);
    let normalized = (f32(raw_value) / max_value) * 2.0 - 1.0;

    // Transformation non-linéaire pour plus de variété
    // Utilise x^0.7 signé pour avoir une meilleure distribution
    let shaped = sign(normalized) * pow(abs(normalized), 0.7);

    // Retourner la valeur mise à l'échelle
    return shaped * FORCE_SCALE_FACTOR;
}

// Décode la force de nourriture depuis le génome
fn decode_food_force(food_genome: u32, particle_type: u32, type_count: u32) -> f32 {
    let bits_per_type = max(16u / max(type_count, 1u), 3u);
    let bit_start = particle_type * bits_per_type;

    if (bit_start >= 16u || bit_start + bits_per_type > 16u) {
        return 0.0;
    }

    let mask = (1u << bits_per_type) - 1u;
    let raw_value = (food_genome >> bit_start) & mask;

    let max_value = f32((1u << bits_per_type) - 1u);
    let normalized = (f32(raw_value) / max_value) * 2.0 - 1.0;

    // Même transformation que pour les forces particule-particule
    let shaped = sign(normalized) * pow(abs(normalized), 0.7);

    return shaped * FORCE_SCALE_FACTOR;
}

// Calcule l'accélération entre deux particules (similaire au projet 2D)
fn acceleration(rmin: f32, dpos: vec3<f32>, a: f32) -> vec3<f32> {
    let dist = length(dpos);
    if (dist < 0.001) {
        return vec3<f32>(0.0);
    }

    var force: f32;
    if (dist < rmin) {
        // Force de répulsion (toujours négative)
        force = (dist / rmin - 1.0);
    } else {
        // Force d'attraction/répulsion basée sur le génome
        force = a * (1.0 - abs(1.0 + rmin - 2.0 * dist) / (1.0 - rmin));
    }

    return dpos * force / dist;
}

// Structure pour retourner position et vélocité modifiées
struct BounceResult {
    position: vec3<f32>,
    velocity: vec3<f32>,
}

// Applique les limites avec rebond
fn apply_bounce_bounds(position: vec3<f32>, velocity: vec3<f32>) -> BounceResult {
    var result: BounceResult;
    result.position = position;
    result.velocity = velocity;

    let half_width = params.grid_width / 2.0;
    let half_height = params.grid_height / 2.0;
    let half_depth = params.grid_depth / 2.0;

    // X bounds
    if (abs(result.position.x) > half_width - PARTICLE_RADIUS) {
        result.position.x = sign(result.position.x) * (half_width - PARTICLE_RADIUS);
        result.velocity.x *= -0.5;
    }

    // Y bounds
    if (abs(result.position.y) > half_height - PARTICLE_RADIUS) {
        result.position.y = sign(result.position.y) * (half_height - PARTICLE_RADIUS);
        result.velocity.y *= -0.5;
    }

    // Z bounds
    if (abs(result.position.z) > half_depth - PARTICLE_RADIUS) {
        result.position.z = sign(result.position.z) * (half_depth - PARTICLE_RADIUS);
        result.velocity.z *= -0.5;
    }

    return result;
}

// Applique les limites avec téléportation
fn apply_teleport_bounds(position: vec3<f32>) -> vec3<f32> {
    var result = position;
    let half_width = params.grid_width / 2.0;
    let half_height = params.grid_height / 2.0;
    let half_depth = params.grid_depth / 2.0;

    // X teleport
    if (result.x > half_width) {
        result.x = -half_width + (result.x - half_width);
    } else if (result.x < -half_width) {
        result.x = half_width + (result.x + half_width);
    }

    // Y teleport
    if (result.y > half_height) {
        result.y = -half_height + (result.y - half_height);
    } else if (result.y < -half_height) {
        result.y = half_height + (result.y + half_height);
    }

    // Z teleport
    if (result.z > half_depth) {
        result.z = -half_depth + (result.z - half_depth);
    } else if (result.z < -half_depth) {
        result.z = half_depth + (result.z + half_depth);
    }

    return result;
}

@compute @workgroup_size(64, 1, 1)
fn update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    // Lire depuis le buffer d'entrée
    var particle = particles_in[index];
    var total_force = vec3<f32>(0.0, 0.0, 0.0);

    // Récupérer le génome de cette simulation
    let genome_idx = particle.simulation_id * 4u;
    let genome_low = genomes[genome_idx];
    let genome_high = genomes[genome_idx + 1u];
    let food_genome = genomes[genome_idx + 2u];

    // Utiliser les valeurs depuis les paramètres
    let min_distance = params.min_distance;
    let max_distance = params.max_force_range;

    // === Forces avec les autres particules ===
    var interactions_count = 0u;

    for (var i = 0u; i < params.particle_count && interactions_count < MAX_INTERACTIONS_PER_PARTICLE; i++) {
        if (i == index) {
            continue;
        }

        // Lire depuis le buffer d'entrée
        let other = particles_in[i];

        // Ignorer les particules d'autres simulations
        if (other.simulation_id != particle.simulation_id) {
            continue;
        }

        let distance_vec = other.position - particle.position;
        let distance_squared = dot(distance_vec, distance_vec);

        // Vérifier si dans la portée
        if (distance_squared == 0.0 || distance_squared > max_distance * max_distance) {
            continue;
        }

        interactions_count++;

        // Calculer la force (maintenant déjà mise à l'échelle)
        let attraction = decode_force(genome_low, genome_high, particle.particle_type, other.particle_type, params.type_count);

        // IMPORTANT: Normaliser les positions par max_distance comme dans le projet 2D
        let dpos_normalized = distance_vec / max_distance;
        let rmin_normalized = min_distance / max_distance;

        let accel = acceleration(rmin_normalized, dpos_normalized, attraction);

        // Multiplier par max_distance pour revenir aux unités du monde
        total_force += accel * max_distance;
    }

    // === Forces avec la nourriture ===
    let particle_food_force = decode_food_force(food_genome, particle.particle_type, params.type_count);

    if (abs(particle_food_force) > 0.001) {
        for (var i = 0u; i < food_count; i++) {
            let food = food_positions[i];

            if (food.is_active == 0u) {
                continue;
            }

            let distance_vec = food.position - particle.position;
            let distance = length(distance_vec);

            if (distance > MIN_DISTANCE && distance < max_distance) {
                let force_direction = normalize(distance_vec);
                let distance_factor = pow(min((FOOD_RADIUS * 2.0) / distance, 1.0), 0.5);
                let force_magnitude = particle_food_force * distance_factor;
                total_force += force_direction * force_magnitude;
            }
        }
    }

    // Appliquer les forces
    particle.velocity += total_force * params.delta_time;

    // Amortissement indépendant du framerate
    particle.velocity *= pow(0.5, params.delta_time / VELOCITY_HALF_LIFE);

    // Limiter la vitesse
    let speed = length(particle.velocity);
    if (speed > MAX_VELOCITY) {
        particle.velocity = normalize(particle.velocity) * MAX_VELOCITY;
    }

    // Appliquer la vélocité
    particle.position += particle.velocity * params.delta_time;

    // Appliquer les limites
    if (params.boundary_mode == 0u) {
        let bounce_result = apply_bounce_bounds(particle.position, particle.velocity);
        particle.position = bounce_result.position;
        particle.velocity = bounce_result.velocity;
    } else {
        particle.position = apply_teleport_bounds(particle.position);
    }

    // Écrire dans le buffer de sortie
    particles_out[index] = particle;
}