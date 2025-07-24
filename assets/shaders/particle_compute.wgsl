// Paramètres de simulation
@group(0) @binding(0) var<uniform> num_particles: u32;
@group(0) @binding(1) var<uniform> dt: f32;
@group(0) @binding(2) var<uniform> world_size: f32;
@group(0) @binding(3) var<uniform> num_types: u32;
@group(0) @binding(4) var<uniform> max_force_range: f32;
@group(0) @binding(5) var<uniform> boundary_mode: u32; // 0=bounce, 1=teleport

// Positions des particules (x, y, z, particle_type)
@group(0) @binding(6) var<storage, read> positions: array<vec4<f32>>;

// Vélocités des particules (x, y, z, unused)
@group(0) @binding(7) var<storage, read> velocities: array<vec4<f32>>;

// Nouvelles positions (output)
@group(0) @binding(8) var<storage, read_write> new_positions: array<vec4<f32>>;

// Nouvelles vélocités (output)
@group(0) @binding(9) var<storage, read_write> new_velocities: array<vec4<f32>>;

// Matrice des forces d'interaction entre types (format linéaire)
@group(0) @binding(10) var<storage, read> force_matrix: array<f32>;

// Positions de nourriture (x, y, z, is_active)
@group(0) @binding(11) var<storage, read> food_positions: array<vec4<f32>>;
@group(0) @binding(12) var<uniform> food_count: u32;

// Forces de nourriture par type
@group(0) @binding(13) var<storage, read> food_forces: array<f32>;

// Constantes physiques
const PARTICLE_RADIUS: f32 = 2.5;
const FOOD_RADIUS: f32 = 1.0;
const MIN_DISTANCE: f32 = 0.001;
const FORCE_SCALE_FACTOR: f32 = 80.0;
const MAX_VELOCITY: f32 = 200.0;
const VELOCITY_HALF_LIFE: f32 = 0.043;
const MAX_INTERACTIONS_PER_PARTICLE: u32 = 100;

// Fonction pour obtenir la force entre deux types de particules
fn get_force_between_types(type_a: u32, type_b: u32) -> f32 {
    let index = type_a * num_types + type_b;
    return force_matrix[index];
}

// Calcule la distance minimale dans un espace torus 3D
fn torus_distance(pos1: vec3<f32>, pos2: vec3<f32>, grid_size: f32) -> f32 {
    let delta = pos2 - pos1;
    let half_size = grid_size * 0.5;

    let dx = abs(delta.x);
    let min_dx = min(dx, grid_size - dx);

    let dy = abs(delta.y);
    let min_dy = min(dy, grid_size - dy);

    let dz = abs(delta.z);
    let min_dz = min(dz, grid_size - dz);

    return sqrt(min_dx * min_dx + min_dy * min_dy + min_dz * min_dz);
}

// Calcule le vecteur de direction minimal dans un espace torus 3D
fn torus_direction_vector(from: vec3<f32>, to: vec3<f32>, grid_size: f32) -> vec3<f32> {
    var direction = vec3<f32>(0.0);
    let half_size = grid_size * 0.5;

    // Axe X
    let dx = to.x - from.x;
    if (abs(dx) <= half_size) {
        direction.x = dx;
    } else {
        direction.x = select(dx + grid_size, dx - grid_size, dx > 0.0);
    }

    // Axe Y
    let dy = to.y - from.y;
    if (abs(dy) <= half_size) {
        direction.y = dy;
    } else {
        direction.y = select(dy + grid_size, dy - grid_size, dy > 0.0);
    }

    // Axe Z
    let dz = to.z - from.z;
    if (abs(dz) <= half_size) {
        direction.z = dz;
    } else {
        direction.z = select(dz + grid_size, dz - grid_size, dz > 0.0);
    }

    return direction;
}

// Calcule l'accélération entre deux particules
fn acceleration(rmin: f32, dpos: vec3<f32>, a: f32, max_range: f32) -> vec3<f32> {
    let dist = length(dpos);
    if (dist < MIN_DISTANCE || dist > max_range) {
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

// Applique les limites avec rebond
fn apply_bounce_bounds(position: vec3<f32>, velocity: vec3<f32>) -> vec4<f32> {
    var result_pos = position;
    var result_vel = velocity;
    let half_size = world_size * 0.5;

    // Rebonds sur les murs
    if (abs(result_pos.x) > half_size - PARTICLE_RADIUS) {
        result_pos.x = sign(result_pos.x) * (half_size - PARTICLE_RADIUS);
        result_vel.x *= -0.5;
    }

    if (abs(result_pos.y) > half_size - PARTICLE_RADIUS) {
        result_pos.y = sign(result_pos.y) * (half_size - PARTICLE_RADIUS);
        result_vel.y *= -0.5;
    }

    if (abs(result_pos.z) > half_size - PARTICLE_RADIUS) {
        result_pos.z = sign(result_pos.z) * (half_size - PARTICLE_RADIUS);
        result_vel.z *= -0.5;
    }

    return vec4<f32>(result_pos, length(result_vel));
}

// Applique les limites avec téléportation
fn apply_teleport_bounds(position: vec3<f32>) -> vec3<f32> {
    var result = position;
    let half_size = world_size * 0.5;

    // Téléportation
    if (result.x > half_size) {
        result.x = -half_size + (result.x - half_size);
    } else if (result.x < -half_size) {
        result.x = half_size + (result.x + half_size);
    }

    if (result.y > half_size) {
        result.y = -half_size + (result.y - half_size);
    } else if (result.y < -half_size) {
        result.y = half_size + (result.y + half_size);
    }

    if (result.z > half_size) {
        result.z = -half_size + (result.z - half_size);
    } else if (result.z < -half_size) {
        result.z = half_size + (result.z + half_size);
    }

    return result;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= num_particles) {
        return;
    }

    // Lire les données de la particule courante
    let current_pos = positions[index].xyz;
    let current_type = u32(positions[index].w);
    let current_vel = velocities[index].xyz;

    var total_force = vec3<f32>(0.0, 0.0, 0.0);

    // Forces avec les autres particules
    var interactions_count = 0u;
    let min_distance = f32(num_types) * PARTICLE_RADIUS;

    for (var i = 0u; i < num_particles && interactions_count < MAX_INTERACTIONS_PER_PARTICLE; i++) {
        if (i == index) {
            continue;
        }

        let other_pos = positions[i].xyz;
        let other_type = u32(positions[i].w);

        // Calcul de distance selon le mode de bord
        let distance_vec = select(
            other_pos - current_pos,
            torus_direction_vector(current_pos, other_pos, world_size),
            boundary_mode == 1u
        );

        let distance_squared = dot(distance_vec, distance_vec);

        if (distance_squared == 0.0 || distance_squared > max_force_range * max_force_range) {
            continue;
        }

        interactions_count++;

        let attraction = get_force_between_types(current_type, other_type) * FORCE_SCALE_FACTOR;
        let accel = acceleration(min_distance, distance_vec, attraction, max_force_range);
        total_force += accel;
    }

    // Forces avec la nourriture
    let particle_food_force = food_forces[current_type] * FORCE_SCALE_FACTOR;

    if (abs(particle_food_force) > 0.001) {
        for (var i = 0u; i < food_count; i++) {
            let food_pos = food_positions[i].xyz;
            let is_active = food_positions[i].w > 0.5;

            if (!is_active) {
                continue;
            }

            let distance_vec_food = select(
                food_pos - current_pos,
                torus_direction_vector(current_pos, food_pos, world_size),
                boundary_mode == 1u
            );

            let distance = length(distance_vec_food);

            if (distance > MIN_DISTANCE && distance < max_force_range) {
                let force_direction = normalize(distance_vec_food);
                let distance_factor = pow(min((FOOD_RADIUS * 2.0) / distance, 1.0), 0.5);
                let force_magnitude = particle_food_force * distance_factor;
                total_force += force_direction * force_magnitude;
            }
        }
    }

    // Appliquer les forces
    var new_vel = current_vel + total_force * dt;

    // Amortissement
    new_vel *= pow(0.5, dt / VELOCITY_HALF_LIFE);

    // Limiter la vitesse
    let speed = length(new_vel);
    if (speed > MAX_VELOCITY) {
        new_vel = normalize(new_vel) * MAX_VELOCITY;
    }

    // Appliquer la vélocité
    var new_pos = current_pos + new_vel * dt;

    // Appliquer les limites
    if (boundary_mode == 0u) {
        let bounce_result = apply_bounce_bounds(new_pos, new_vel);
        new_pos = bounce_result.xyz;
        new_vel = current_vel * (bounce_result.w / speed);
    } else {
        new_pos = apply_teleport_bounds(new_pos);
    }

    // Écrire les résultats
    new_positions[index] = vec4<f32>(new_pos, f32(current_type));
    new_velocities[index] = vec4<f32>(new_vel, 0.0);
}