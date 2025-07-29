use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_app_compute::prelude::*;
use crate::components::entities::food::Food;
use crate::components::entities::particle::{Particle, ParticleType, Velocity};
use crate::components::entities::simulation::{Simulation, SimulationId};
use crate::components::genetics::genotype::Genotype;
use crate::resources::config::simulation::{SimulationParameters, SimulationSpeed};
use crate::resources::world::boundary::BoundaryMode;
use crate::resources::world::grid::GridParameters;
use crate::states::app::AppState;

pub struct ParticleComputePlugin;

/// Ressource pour activer/désactiver le compute shader
#[derive(Resource, Default)]
pub struct ComputeEnabled(pub bool);

impl Plugin for ParticleComputePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComputeEnabled>()
            .add_plugins(AppComputeWorkerPlugin::<ParticleComputeWorker>::default())
            .add_systems(
                Update,
                (
                    update_compute_buffers,
                    run_compute_simulation.after(update_compute_buffers),
                    apply_compute_results.after(run_compute_simulation),
                )
                    .chain()
                    .run_if(in_state(AppState::Simulation))
                    .run_if(compute_enabled),
            );
    }
}

#[derive(TypePath)]
struct ParticleComputeShader;

impl ComputeShader for ParticleComputeShader {
    fn shader() -> ShaderRef {
        "shaders/particle_compute.wgsl".into()
    }
}

#[derive(Resource)]
struct ParticleComputeWorker;

impl ComputeWorker for ParticleComputeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let sim_params = world.resource::<SimulationParameters>();
        let grid_params = world.resource::<GridParameters>();
        let boundary_mode = world.resource::<BoundaryMode>();

        let num_particles = sim_params.particle_count as u32;
        let dt = 1.0f32 / 60.0; // 60 FPS
        let world_size = grid_params
            .width
            .max(grid_params.height)
            .max(grid_params.depth);
        let num_types = sim_params.particle_types as u32;
        let max_force_range = sim_params.max_force_range;
        let boundary_mode_u32 = match boundary_mode {
            BoundaryMode::Bounce => 0u32,
            BoundaryMode::Teleport => 1u32,
        };

        // Buffers initiaux vides
        let positions = vec![[0.0f32; 4]; num_particles as usize];
        let velocities = vec![[0.0f32; 4]; num_particles as usize];
        let force_matrix = vec![0.0f32; (num_types * num_types) as usize];
        let food_positions = vec![[0.0f32; 4]; 1]; // Au moins 1 élément
        let food_forces = vec![0.0f32; num_types as usize];
        let food_count = 0u32;

        info!(
            "Initializing compute worker with {} particles, {} types",
            num_particles, num_types
        );

        AppComputeWorkerBuilder::new(world)
            // Paramètres uniformes
            .add_uniform("num_particles", &num_particles)
            .add_uniform("dt", &dt)
            .add_uniform("world_size", &world_size)
            .add_uniform("num_types", &num_types)
            .add_uniform("max_force_range", &max_force_range)
            .add_uniform("boundary_mode", &boundary_mode_u32)
            .add_uniform("food_count", &food_count)
            // Buffers de données
            .add_staging("positions", &positions)
            .add_staging("velocities", &velocities)
            .add_staging("new_positions", &positions)
            .add_staging("new_velocities", &velocities)
            .add_staging("force_matrix", &force_matrix)
            .add_staging("food_positions", &food_positions)
            .add_staging("food_forces", &food_forces)
            // Passe de calcul
            .add_pass::<ParticleComputeShader>(
                [((num_particles + 63) / 64) as u32, 1, 1],
                &[
                    "num_particles",
                    "dt",
                    "world_size",
                    "num_types",
                    "max_force_range",
                    "boundary_mode",
                    "positions",
                    "velocities",
                    "new_positions",
                    "new_velocities",
                    "force_matrix",
                    "food_positions",
                    "food_count",
                    "food_forces",
                ],
            )
            .build()
    }
}

fn compute_enabled(compute: Res<ComputeEnabled>) -> bool {
    compute.0
}

/// Met à jour les buffers GPU avec les données actuelles des entités
fn update_compute_buffers(
    mut compute_worker: ResMut<AppComputeWorker<ParticleComputeWorker>>,
    sim_params: Res<SimulationParameters>,
    grid_params: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
    particles: Query<(&Transform, &Velocity, &ParticleType, &ChildOf), With<Particle>>,
    simulations: Query<(&SimulationId, &Genotype), With<Simulation>>,
    food_query: Query<(&Transform, &ViewVisibility), With<Food>>,
) {
    if !compute_worker.ready() {
        return;
    }

    // Collecte des positions et vélocités des particules
    let mut positions = Vec::new();
    let mut velocities = Vec::new();

    for (transform, velocity, particle_type, parent) in particles.iter() {
        if simulations.get(parent.parent()).is_ok() {
            positions.push([
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                particle_type.0 as f32,
            ]);
            velocities.push([velocity.0.x, velocity.0.y, velocity.0.z, 0.0]);
        }
    }

    if positions.is_empty() {
        warn!("GPU: Aucune particule trouvée!");
        return;
    }

    // Mettre à jour seulement les données qui changent
    compute_worker.write_slice("positions", &positions);
    compute_worker.write_slice("velocities", &velocities);

    // Forces des simulations (peuvent changer entre époques)
    if let Some((_, genotype)) = simulations.iter().next() {
        compute_worker.write_slice("force_matrix", &genotype.force_matrix);
        compute_worker.write_slice("food_forces", &genotype.food_forces);
    } else {
        warn!("GPU: Aucune simulation trouvée!");
        return;
    }

    // Nourriture
    let mut food_positions = Vec::new();
    for (transform, visibility) in food_query.iter() {
        food_positions.push([
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            if visibility.get() { 1.0 } else { 0.0 },
        ]);
    }

    if food_positions.is_empty() {
        food_positions.push([0.0, 0.0, 0.0, 0.0]);
    }

    compute_worker.write_slice("food_positions", &food_positions);

    info!(
        "GPU Update: {} particules, forces={}, nourriture={}",
        positions.len(),
        simulations
            .iter()
            .next()
            .map_or(0, |(_, g)| g.force_matrix.len()),
        food_positions.len()
    );
}

/// Exécute la simulation compute selon la vitesse de simulation
fn run_compute_simulation(
    mut compute_worker: ResMut<AppComputeWorker<ParticleComputeWorker>>,
    sim_params: Res<SimulationParameters>,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    if !compute_worker.ready() {
        return;
    }

    // Initialiser le timer pour 60 FPS
    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(1.0 / 60.0, TimerMode::Repeating);
    }

    timer.tick(time.delta());

    if !timer.just_finished() {
        return;
    }

    // Calculer le nombre d'itérations selon la vitesse
    let iterations = match sim_params.simulation_speed {
        SimulationSpeed::Paused => 0,
        SimulationSpeed::Normal => 1,
        SimulationSpeed::Fast => 2,
        SimulationSpeed::VeryFast => 4,
    };

    // Debug: afficher le nombre d'itérations
    if iterations > 0 {
        // Exécuter les itérations
        for _ in 0..iterations {
            compute_worker.execute();

            // Copier les résultats pour la prochaine itération
            if iterations > 1 {
                let new_positions: Vec<[f32; 4]> = compute_worker.read_vec("new_positions");
                let new_velocities: Vec<[f32; 4]> = compute_worker.read_vec("new_velocities");

                compute_worker.write_slice("positions", &new_positions);
                compute_worker.write_slice("velocities", &new_velocities);
            }
        }
    }
}

/// Applique les résultats du compute aux entités
fn apply_compute_results(
    compute_worker: Res<AppComputeWorker<ParticleComputeWorker>>,
    mut particles: Query<(Entity, &mut Transform, &mut Velocity), With<Particle>>,
) {
    if !compute_worker.ready() {
        return;
    }

    let new_positions: Vec<[f32; 4]> = compute_worker.read_vec("new_positions");
    let new_velocities: Vec<[f32; 4]> = compute_worker.read_vec("new_velocities");

    if new_positions.is_empty() || new_velocities.is_empty() {
        warn!("GPU: Résultats vides!");
        return;
    }

    // Appliquer les résultats aux entités avec index sécurisé
    for (i, (_, mut transform, mut velocity)) in particles.iter_mut().enumerate() {
        if let (Some(pos), Some(vel)) = (new_positions.get(i), new_velocities.get(i)) {
            let new_pos = Vec3::new(pos[0], pos[1], pos[2]);
            let new_vel = Vec3::new(vel[0], vel[1], vel[2]);

            // Vérifier que les valeurs sont valides
            if new_pos.is_finite() && new_vel.is_finite() {
                transform.translation = new_pos;
                velocity.0 = new_vel;
            }
        }
    }
}
