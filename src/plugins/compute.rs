use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        Render, RenderApp, RenderSet,
        MainWorld,
    },
};
use std::borrow::Cow;
use bevy::render::Extract;
use bytemuck::{Pod, Zeroable};

use crate::{
    components::{
        particle::{Particle, ParticleType, Velocity},
        simulation::{Simulation, SimulationId},
        genotype::Genotype,
        food::Food,
    },
    resources::{
        simulation::{SimulationParameters, SimulationSpeed},
        grid::GridParameters,
        boundary::BoundaryMode,
    },
    states::app::AppState,
    states::simulation::SimulationState,
    globals::{PARTICLE_RADIUS, PHYSICS_TIMESTEP}, // AJOUT : import de PHYSICS_TIMESTEP
};

/// Chemin vers le shader
const SHADER_ASSET_PATH: &str = "shaders/particle_life.wgsl";

/// Taille du workgroup (doit correspondre au shader)
const WORKGROUP_SIZE: u32 = 64;

/// Structure pour une particule sur le GPU (doit correspondre au shader)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, ShaderType)]
struct GpuParticle {
    position: [f32; 3],
    _padding1: f32,
    velocity: [f32; 3],
    particle_type: u32,
    simulation_id: u32,
    _padding2: [f32; 3],
}

/// Structure pour les paramètres de simulation sur le GPU
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, ShaderType)]
struct GpuSimulationParams {
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
    _padding: [f32; 2],
}

/// Structure pour la nourriture sur le GPU
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct GpuFood {
    position: [f32; 3],
    is_active: u32,
}

pub struct ParticleComputePlugin;

/// Ressource pour activer/désactiver le compute shader
#[derive(Resource, Default)]
pub struct ComputeEnabled(pub bool);

impl Plugin for ParticleComputePlugin {
    fn build(&self, app: &mut App) {
        // Ressources pour le monde principal
        app.init_resource::<ComputeEnabled>()
            .init_resource::<SyncedComputeResults>()
            .add_plugins(ExtractResourcePlugin::<SyncedComputeResults>::default());

        // Système d'extraction dans le monde principal
        app.add_systems(
            ExtractSchedule,
            (extract_particle_data, extract_simulation_params)
                .run_if(in_state(AppState::Simulation)),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ExtractedParticleData>()
            .init_resource::<ComputeResultsBuffer>()
            .init_resource::<GpuBuffersState>()
            .init_resource::<SyncedComputeResults>()
            .insert_resource(SimulationParameters::default())
            .add_systems(
                Render,
                (
                    prepare_particle_buffers.in_set(RenderSet::PrepareResources),
                    write_compute_results.in_set(RenderSet::Cleanup),
                ),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ParticleComputeLabel, ParticleComputeNode::default());
        render_graph.add_node_edge(
            ParticleComputeLabel,
            bevy::render::graph::CameraDriverLabel,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ParticleComputePipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ParticleComputeLabel;

/// Ressource extraite contenant toutes les données des particules
#[derive(Resource, Clone, Default)]
pub struct ExtractedParticleData {
    pub particles: Vec<(Entity, Vec3, Vec3, usize, usize)>, // (entity, position, velocity, type, sim_id)
    pub genomes: Vec<(u64, u16)>, // (genome, food_genome) par simulation
    pub food_positions: Vec<(Vec3, bool)>, // (position, is_active)
    pub params: GpuSimulationParams,
    pub enabled: bool,
}

/// Buffer pour stocker les résultats du compute shader
#[derive(Resource, Default)]
struct ComputeResultsBuffer {
    data: Vec<GpuParticle>,
}

/// Ressource pour synchroniser les résultats entre le render world et le main world
#[derive(Resource, Default, Clone, ExtractResource)]
pub struct SyncedComputeResults {
    pub data: Vec<(Entity, GpuParticle)>,
}

/// État persistant des buffers GPU
#[derive(Resource, Default)]
struct GpuBuffersState {
    allocated_particles: usize,
    allocated_simulations: usize,
}

/// Système pour extraire les paramètres de simulation
fn extract_simulation_params(
    mut commands: Commands,
    sim_params: Extract<Res<SimulationParameters>>,
) {
    commands.insert_resource(sim_params.clone());
}

/// MODIFICATION PRINCIPALE : Système d'extraction avec timestep fixe
fn extract_particle_data(
    mut extracted_data: ResMut<ExtractedParticleData>,
    compute_enabled: Extract<Res<ComputeEnabled>>,
    sim_params: Extract<Res<SimulationParameters>>,
    grid_params: Extract<Res<GridParameters>>,
    boundary_mode: Extract<Res<BoundaryMode>>,
    time: Extract<Res<Time>>,
    // Queries pour extraire les données
    particles_query: Extract<Query<(Entity, &Transform, &Velocity, &ParticleType, &ChildOf), With<Particle>>>,
    simulations_query: Extract<Query<(&SimulationId, &Genotype), With<Simulation>>>,
    food_query: Extract<Query<(&Transform, &ViewVisibility), With<Food>>>,
) {
    // Réinitialiser les données extraites
    extracted_data.particles.clear();
    extracted_data.genomes.clear();
    extracted_data.food_positions.clear();

    // MODIFICATION CRITIQUE : Toujours utiliser le timestep physique constant
    extracted_data.params = GpuSimulationParams {
        delta_time: if compute_enabled.0 && sim_params.simulation_speed != SimulationSpeed::Paused {
            PHYSICS_TIMESTEP // CHANGEMENT : Utiliser la constante globale
        } else {
            0.0
        },
        particle_count: 0, // Sera mis à jour après
        simulation_count: sim_params.simulation_count as u32,
        type_count: sim_params.particle_types as u32,
        max_force_range: sim_params.max_force_range,
        min_distance: sim_params.particle_types as f32 * PARTICLE_RADIUS,
        grid_width: grid_params.width,
        grid_height: grid_params.height,
        grid_depth: grid_params.depth,
        boundary_mode: match **boundary_mode {
            BoundaryMode::Bounce => 0,
            BoundaryMode::Teleport => 1,
        },
        _padding: [0.0; 2],
    };

    extracted_data.enabled = compute_enabled.0;

    // Si le compute est désactivé, on continue quand même pour initialiser les buffers
    if !compute_enabled.0 {
        return;
    }

    // Créer un cache des simulations
    let mut sim_cache = std::collections::HashMap::new();
    for (sim_id, genotype) in simulations_query.iter() {
        sim_cache.insert(sim_id.0, (*genotype, sim_id.0));
    }

    // Extraire les particules
    for (entity, transform, velocity, particle_type, parent) in particles_query.iter() {
        // Trouver la simulation parente
        if let Ok((sim_id, _)) = simulations_query.get(parent.parent()) {
            extracted_data.particles.push((
                entity,
                transform.translation,
                velocity.0,
                particle_type.0,
                sim_id.0,
            ));
        }
    }

    // Mettre à jour le nombre de particules
    extracted_data.params.particle_count = extracted_data.particles.len() as u32;

    // Extraire les génomes (toujours créer un tableau de la bonne taille)
    let mut genomes = vec![(0u64, 0u16); sim_params.simulation_count];
    for (sim_id, genotype) in simulations_query.iter() {
        if sim_id.0 < genomes.len() {
            genomes[sim_id.0] = (genotype.genome, genotype.food_force_genome);
        }
    }
    extracted_data.genomes = genomes;

    // Extraire la nourriture visible
    for (transform, visibility) in food_query.iter() {
        extracted_data.food_positions.push((
            transform.translation,
            visibility.get(),
        ));
    }
}

/// Ressource contenant les buffers GPU - Avec deux buffers séparés
#[derive(Resource)]
struct ParticleBuffers {
    particle_buffer_in: Buffer,
    particle_buffer_out: Buffer,
    params_buffer: Buffer,
    genome_buffer: Buffer,
    food_buffer: Buffer,
    food_count_buffer: Buffer,
    bind_group: BindGroup,
    particle_count: usize,
}

/// Prépare les buffers GPU
fn prepare_particle_buffers(
    mut commands: Commands,
    pipeline: Res<ParticleComputePipeline>,
    render_device: Res<RenderDevice>,
    extracted_data: Res<ExtractedParticleData>,
    mut buffers_state: ResMut<GpuBuffersState>,
    existing_buffers: Option<Res<ParticleBuffers>>,
) {
    // Si le compute est désactivé ET qu'il n'y a pas de particules, ne pas créer de buffers
    if !extracted_data.enabled && extracted_data.particles.is_empty() {
        commands.remove_resource::<ParticleBuffers>();
        return;
    }

    // Toujours créer des buffers minimaux pour éviter les erreurs
    let particle_count = extracted_data.particles.len().max(1);
    let simulation_count = extracted_data.params.simulation_count.max(1) as usize;
    let food_count = extracted_data.food_positions.len().max(1);

    // Vérifier si on doit recréer les buffers
    let needs_recreation = buffers_state.allocated_particles < particle_count ||
        buffers_state.allocated_simulations < simulation_count;

    if !needs_recreation && existing_buffers.is_some() {
        return;
    }

    // Convertir les particules en format GPU
    let mut gpu_particles: Vec<GpuParticle> = Vec::with_capacity(particle_count);

    for (_, pos, vel, p_type, sim_id) in &extracted_data.particles {
        gpu_particles.push(GpuParticle {
            position: pos.to_array(),
            _padding1: 0.0,
            velocity: vel.to_array(),
            particle_type: *p_type as u32,
            simulation_id: *sim_id as u32,
            _padding2: [0.0; 3],
        });
    }

    // Ajouter des particules vides si nécessaire
    while gpu_particles.len() < particle_count {
        gpu_particles.push(GpuParticle {
            position: [0.0; 3],
            _padding1: 0.0,
            velocity: [0.0; 3],
            particle_type: 0,
            simulation_id: 0,
            _padding2: [0.0; 3],
        });
    }

    // Créer deux buffers séparés pour éviter les race conditions
    let particle_buffer_in = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Particle Buffer In"),
        contents: bytemuck::cast_slice(&gpu_particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let particle_buffer_out = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Particle Buffer Out"),
        contents: bytemuck::cast_slice(&gpu_particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    });

    // Buffer des paramètres
    let params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Simulation Params Buffer"),
        contents: bytemuck::bytes_of(&extracted_data.params),
        usage: BufferUsages::UNIFORM,
    });

    // Buffer des génomes (4 u32 par simulation)
    let mut genome_data: Vec<u32> = Vec::with_capacity(simulation_count * 4);

    for i in 0..simulation_count {
        if i < extracted_data.genomes.len() {
            let (genome, food_genome) = extracted_data.genomes[i];
            genome_data.push(genome as u32);
            genome_data.push((genome >> 32) as u32);
            genome_data.push(food_genome as u32);
            genome_data.push(0u32); // padding
        } else {
            genome_data.extend_from_slice(&[0u32; 4]);
        }
    }

    let genome_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Genome Buffer"),
        contents: bytemuck::cast_slice(&genome_data),
        usage: BufferUsages::STORAGE,
    });

    // Buffer de la nourriture
    let mut gpu_food: Vec<GpuFood> = Vec::with_capacity(food_count);

    for (pos, active) in &extracted_data.food_positions {
        gpu_food.push(GpuFood {
            position: pos.to_array(),
            is_active: if *active { 1 } else { 0 },
        });
    }

    while gpu_food.len() < food_count {
        gpu_food.push(GpuFood {
            position: [0.0; 3],
            is_active: 0,
        });
    }

    let food_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Food Buffer"),
        contents: bytemuck::cast_slice(&gpu_food),
        usage: BufferUsages::STORAGE,
    });

    // Buffer pour le nombre de nourritures
    let food_count_val = extracted_data.food_positions.len() as u32;
    let food_count_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Food Count Buffer"),
        contents: bytemuck::bytes_of(&food_count_val),
        usage: BufferUsages::UNIFORM,
    });

    // Créer le bind group avec 6 entrées
    let bind_group = render_device.create_bind_group(
        Some("Particle Compute Bind Group"),
        &pipeline.bind_group_layout,
        &BindGroupEntries::sequential((
            particle_buffer_in.as_entire_binding(),
            particle_buffer_out.as_entire_binding(),
            params_buffer.as_entire_binding(),
            genome_buffer.as_entire_binding(),
            food_buffer.as_entire_binding(),
            food_count_buffer.as_entire_binding(),
        )),
    );

    // Mettre à jour l'état des buffers
    buffers_state.allocated_particles = particle_count;
    buffers_state.allocated_simulations = simulation_count;

    commands.insert_resource(ParticleBuffers {
        particle_buffer_in,
        particle_buffer_out,
        params_buffer,
        genome_buffer,
        food_buffer,
        food_count_buffer,
        bind_group,
        particle_count: extracted_data.particles.len(),
    });
}

/// Pipeline pour le compute shader
#[derive(Resource)]
struct ParticleComputePipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl FromWorld for ParticleComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Layout avec des buffers séparés pour éviter les race conditions
        let bind_group_layout = render_device.create_bind_group_layout(
            "Particle Compute Bind Group Layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // Particles in (read-only)
                    storage_buffer_read_only::<GpuParticle>(false),
                    // Particles out (write-only)
                    storage_buffer::<GpuParticle>(false),
                    // Parameters
                    uniform_buffer::<GpuSimulationParams>(false),
                    // Genomes
                    storage_buffer_read_only::<u32>(false),
                    // Food positions
                    storage_buffer_read_only::<GpuFood>(false),
                    // Food count
                    uniform_buffer::<u32>(false),
                ),
            ),
        );

        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Particle Compute Pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
            zero_initialize_workgroup_memory: false,
        });

        Self {
            bind_group_layout,
            pipeline,
        }
    }
}

/// MODIFICATION PRINCIPALE : Nœud GPU avec itérations cohérentes
struct ParticleComputeNode;

impl Default for ParticleComputeNode {
    fn default() -> Self {
        Self
    }
}

impl render_graph::Node for ParticleComputeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let Some(buffers) = world.get_resource::<ParticleBuffers>() else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ParticleComputePipeline>();

        let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
            return Ok(());
        };

        let extracted_data = world.resource::<ExtractedParticleData>();
        let sim_params = world.resource::<SimulationParameters>();

        if !extracted_data.enabled || buffers.particle_count == 0 {
            return Ok(());
        }

        // MODIFICATION CRITIQUE : Même logique d'itération que le CPU
        let iterations = match sim_params.simulation_speed {
            SimulationSpeed::Paused => 0,
            SimulationSpeed::Normal => 1,
            SimulationSpeed::Fast => 2,
            SimulationSpeed::VeryFast => 4,
        };

        // CHAQUE itération = un pas physique complet avec timestep fixe
        for iteration in 0..iterations {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some(&format!("Particle Compute Pass {}", iteration)),
                    timestamp_writes: None,
                });

            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.set_pipeline(compute_pipeline);

            // Calculer le nombre de workgroups nécessaires
            let num_workgroups = (buffers.particle_count as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            pass.dispatch_workgroups(num_workgroups, 1, 1);

            drop(pass);

            // CRITIQUE : Copier les résultats pour la prochaine itération
            // Cela permet au GPU de voir les nouvelles positions comme le CPU
            render_context.command_encoder().copy_buffer_to_buffer(
                &buffers.particle_buffer_out,
                0,
                &buffers.particle_buffer_in,
                0,
                (std::mem::size_of::<GpuParticle>() * buffers.particle_count) as u64,
            );
        }

        Ok(())
    }
}

/// Système pour copier les résultats dans un buffer accessible (inchangé)
fn write_compute_results(
    buffers: Option<Res<ParticleBuffers>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut results_buffer: ResMut<ComputeResultsBuffer>,
    extracted_data: Res<ExtractedParticleData>,
    mut synced_results: ResMut<SyncedComputeResults>,
) {
    let Some(buffers) = buffers else { return; };

    if buffers.particle_count == 0 {
        return;
    }

    // Allouer le buffer de résultats si nécessaire
    if results_buffer.data.len() != buffers.particle_count {
        results_buffer.data.resize(buffers.particle_count, GpuParticle {
            position: [0.0; 3],
            _padding1: 0.0,
            velocity: [0.0; 3],
            particle_type: 0,
            simulation_id: 0,
            _padding2: [0.0; 3],
        });
    }

    // Créer un buffer de staging pour la lecture
    let staging_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (std::mem::size_of::<GpuParticle>() * buffers.particle_count) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Encoder la copie
    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
    });

    encoder.copy_buffer_to_buffer(
        &buffers.particle_buffer_out,
        0,
        &staging_buffer,
        0,
        (std::mem::size_of::<GpuParticle>() * buffers.particle_count) as u64,
    );

    render_queue.submit([encoder.finish()]);

    // Mapper le buffer pour la lecture
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = flume::bounded(1);

    buffer_slice.map_async(MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    render_device.poll(Maintain::Wait);

    if let Ok(Ok(())) = receiver.recv() {
        let data = buffer_slice.get_mapped_range();
        let gpu_particles: &[GpuParticle] = bytemuck::cast_slice(&data);
        results_buffer.data.copy_from_slice(gpu_particles);

        // Mettre à jour directement les résultats synchronisés
        synced_results.data.clear();
        for (i, (entity, _, _, _, _)) in extracted_data.particles.iter().enumerate() {
            if i < results_buffer.data.len() {
                synced_results.data.push((*entity, results_buffer.data[i]));
            }
        }
    }
}

/// Système pour appliquer les résultats du compute shader aux entités (inchangé)
pub fn apply_compute_results(
    mut particles: Query<(&mut Transform, &mut Velocity), With<Particle>>,
    results: Res<SyncedComputeResults>,
    compute_enabled: Res<ComputeEnabled>,
) {
    if !compute_enabled.0 || results.data.is_empty() {
        return;
    }

    // Appliquer les résultats aux entités
    for (entity, gpu_particle) in results.data.iter() {
        if let Ok((mut transform, mut velocity)) = particles.get_mut(*entity) {
            transform.translation = Vec3::from_array(gpu_particle.position);
            velocity.0 = Vec3::from_array(gpu_particle.velocity);
        }
    }
}