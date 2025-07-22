// Constantes
const PARTICLE_RADIUS: f32 = 2.5;
const FOOD_RADIUS: f32 = 1.0;
const MIN_DISTANCE: f32 = 0.001;
const PARTICLE_REPULSION_STRENGTH: f32 = 100.0;
const FORCE_SCALE_FACTOR: f32 = 80.0;
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

// NOUVEAU : Calcule la distance minimale dans un espace torus 3D
fn torus_distance(pos1: vec3<f32>, pos2: vec3<f32>, grid_size: vec3<f32>) -> f32 {
    let delta = pos2 - pos1;

    // Calculer la distance minimale sur chaque axe
    let dx = abs(delta.x);
    let min_dx = min(dx, grid_size.x - dx);

    let dy = abs(delta.y);
    let min_dy = min(dy, grid_size.y - dy);

    let dz = abs(delta.z);
    let min_dz = min(dz, grid_size.z - dz);

    return sqrt(min_dx * min_dx + min_dy * min_dy + min_dz * min_dz);
}

// NOUVEAU : Calcule le vecteur de direction minimal dans un espace torus 3D
fn torus_direction_vector(from: vec3<f32>, to: vec3<f32>, grid_size: vec3<f32>) -> vec3<f32> {
    var direction = vec3<f32>(0.0);

    // Axe X
    let dx = to.x - from.x;
    if (abs(dx) <= grid_size.x / 2.0) {
        direction.x = dx;
    } else {
        // Plus court de passer par l'autre côté
        if (dx > 0.0) {
            direction.x = dx - grid_size.x;
        } else {
            direction.x = dx + grid_size.x;
        }
    }

    // Axe Y
    let dy = to.y - from.y;
    if (abs(dy) <= grid_size.y / 2.0) {
        direction.y = dy;
    } else {
        if (dy > 0.0) {
            direction.y = dy - grid_size.y;
        } else {
            direction.y = dy + grid_size.y;
        }
    }

    // Axe Z
    let dz = to.z - from.z;
    if (abs(dz) <= grid_size.z / 2.0) {
        direction.z = dz;
    } else {
        if (dz > 0.0) {
            direction.z = dz - grid_size.z;
        } else {
            direction.z = dz + grid_size.z;
        }
    }

    return direction;
}

// Calcule l'accélération entre deux particules
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

    // NOUVEAU : Taille de grille pour les calculs torus
    let grid_size = vec3<f32>(params.grid_width, params.grid_height, params.grid_depth);
    let is_teleport_mode = params.boundary_mode == 1u;

    // === Forces avec les autres particules (AMÉLIORÉ AVEC TORUS) ===
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

        // MODIFICATION PRINCIPALE : Calcul de distance selon le mode de bord
        let distance_vec: vec3<f32>;
        let distance_squared: f32;

        if (is_teleport_mode) {
            // Mode torus : utiliser la direction minimale
            distance_vec = torus_direction_vector(particle.position, other.position, grid_size);
            distance_squared = dot(distance_vec, distance_vec);
        } else {
            // Mode bounce : distance normale
            distance_vec = other.position - particle.position;
            distance_squared = dot(distance_vec, distance_vec);
        }

        // Vérifier si dans la portée
        if (distance_squared == 0.0 || distance_squared > max_distance * max_distance) {
            continue;
        }

        interactions_count++;

        // Calculer la force
        let attraction = decode_force(genome_low, genome_high, particle.particle_type, other.particle_type, params.type_count);

        // Normaliser les positions par max_distance
        let dpos_normalized = distance_vec / max_distance;
        let rmin_normalized = min_distance / max_distance;

        let accel = acceleration(rmin_normalized, dpos_normalized, attraction);

        // Multiplier par max_distance pour revenir aux unités du monde
        total_force += accel * max_distance;
    }

    // === Forces avec la nourriture (AMÉLIORÉES AVEC TORUS) ===
    let particle_food_force = decode_food_force(food_genome, particle.particle_type, params.type_count);

    if (abs(particle_food_force) > 0.001) {
        for (var i = 0u; i < food_count; i++) {
            let food = food_positions[i];

            if (food.is_active == 0u) {
                continue;
            }

            // NOUVEAU : Calcul de distance/direction selon le mode de bord pour la nourriture
            let distance_vec_food: vec3<f32>;
            let distance: f32;

            if (is_teleport_mode) {
                // Mode torus pour la nourriture
                distance_vec_food = torus_direction_vector(particle.position, food.position, grid_size);
                distance = length(distance_vec_food);
            } else {
                // Mode bounce pour la nourriture
                distance_vec_food = food.position - particle.position;
                distance = length(distance_vec_food);
            }

            if (distance > MIN_DISTANCE && distance < max_distance) {
                let force_direction = normalize(distance_vec_food);
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