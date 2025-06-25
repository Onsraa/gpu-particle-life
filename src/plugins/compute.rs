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
        simulation::SimulationParameters,
        grid::GridParameters,
        boundary::BoundaryMode,
    },
    states::app::AppState,
    states::simulation::SimulationState,
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
    grid_width: f32,
    grid_height: f32,
    grid_depth: f32,
    boundary_mode: u32,
    _padding: [f32; 3],
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
        app.init_resource::<ComputeEnabled>()
            .init_resource::<SyncedComputeResults>()
            .add_plugins(ExtractResourcePlugin::<SyncedComputeResults>::default());

        app.add_systems(
            ExtractSchedule,
            extract_particle_data,
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ExtractedParticleData>()
            .init_resource::<ComputeResultsBuffer>()
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

/// Système pour extraire les données des particules du monde principal
fn extract_particle_data(
    mut commands: Commands,
    main_world: Extract<&World>,
) {
    let mut extracted_data = ExtractedParticleData::default();

    // Vérifier si le compute est activé
    let compute_enabled = main_world.get_resource::<ComputeEnabled>()
        .map(|c| c.0)
        .unwrap_or(false);

    if !compute_enabled {
        extracted_data.enabled = false;
        info!("Compute disabled in extract_particle_data");
        commands.insert_resource(extracted_data);
        return;
    }

    // Récupérer les paramètres
    let Some(sim_params) = main_world.get_resource::<SimulationParameters>() else {
        extracted_data.enabled = false;
        info!("No SimulationParameters found");
        commands.insert_resource(extracted_data);
        return;
    };

    if sim_params.simulation_speed == crate::resources::simulation::SimulationSpeed::Paused {
        extracted_data.enabled = false;
        info!("Simulation paused");
        commands.insert_resource(extracted_data);
        return;
    }

    let Some(grid_params) = main_world.get_resource::<GridParameters>() else {
        extracted_data.enabled = false;
        commands.insert_resource(extracted_data);
        return;
    };

    let Some(boundary_mode) = main_world.get_resource::<BoundaryMode>() else {
        extracted_data.enabled = false;
        commands.insert_resource(extracted_data);
        return;
    };

    let Some(time) = main_world.get_resource::<Time>() else {
        extracted_data.enabled = false;
        commands.insert_resource(extracted_data);
        return;
    };

    extracted_data.enabled = true;

    // Parcourir toutes les entités pour extraire les données
    let mut particle_data = Vec::new();
    let mut simulation_data = Vec::new();
    let mut food_data = Vec::new();

    for entity_ref in main_world.iter_entities() {
        let entity = entity_ref.id();

        // Vérifier si c'est une particule
        if let Some(transform) = entity_ref.get::<Transform>() {
            if let Some(velocity) = entity_ref.get::<Velocity>() {
                if let Some(particle_type) = entity_ref.get::<ParticleType>() {
                    if let Some(parent) = entity_ref.get::<ChildOf>() {
                        // C'est une particule, essayer de trouver sa simulation parente
                        let parent_entity = parent.parent();
                        if let Ok(parent_ref) = main_world.get_entity(parent_entity) {
                            if let Some(sim_parent) = parent_ref.get::<ChildOf>() {
                                let sim_entity = sim_parent.parent();
                                if let Ok(sim_ref) = main_world.get_entity(sim_entity) {
                                    if let Some(sim_id) = sim_ref.get::<SimulationId>() {
                                        particle_data.push((
                                            entity,
                                            transform.translation,
                                            velocity.0,
                                            particle_type.0,
                                            sim_id.0,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Vérifier si c'est une simulation
        if let Some(sim_id) = entity_ref.get::<SimulationId>() {
            if let Some(genotype) = entity_ref.get::<Genotype>() {
                simulation_data.push((sim_id.0, genotype.genome, genotype.food_force_genome));
            }
        }

        // Vérifier si c'est de la nourriture
        if entity_ref.get::<Food>().is_some() {
            if let Some(transform) = entity_ref.get::<Transform>() {
                if let Some(visibility) = entity_ref.get::<ViewVisibility>() {
                    food_data.push((transform.translation, visibility.get()));
                }
            }
        }
    }

    info!("Found {} particles, {} simulations, {} food",
          particle_data.len(), simulation_data.len(), food_data.len());

    // Trier et organiser les données
    extracted_data.particles = particle_data;

    // Créer le tableau des génomes
    let mut genomes = vec![(0u64, 0u16); sim_params.simulation_count];
    for (sim_id, genome, food_genome) in simulation_data {
        if sim_id < genomes.len() {
            genomes[sim_id] = (genome, food_genome);
        }
    }
    extracted_data.genomes = genomes;

    // Ajouter la nourriture
    extracted_data.food_positions = food_data;

    // Paramètres
    extracted_data.params = GpuSimulationParams {
        delta_time: time.delta_secs() * sim_params.simulation_speed.multiplier(),
        particle_count: extracted_data.particles.len() as u32,
        simulation_count: sim_params.simulation_count as u32,
        type_count: sim_params.particle_types as u32,
        max_force_range: sim_params.max_force_range,
        grid_width: grid_params.width,
        grid_height: grid_params.height,
        grid_depth: grid_params.depth,
        boundary_mode: match *boundary_mode {
            BoundaryMode::Bounce => 0,
            BoundaryMode::Teleport => 1,
        },
        _padding: [0.0; 3],
    };

    info!("Extracted particle data: enabled={}, particles={}",
          extracted_data.enabled, extracted_data.particles.len());

    commands.insert_resource(extracted_data);
}

/// Ressource contenant les buffers GPU
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
    pipeline: Option<Res<ParticleComputePipeline>>,
    render_device: Res<RenderDevice>,
    extracted_data: Res<ExtractedParticleData>,
) {
    let Some(pipeline) = pipeline else {
        warn!("ParticleComputePipeline not ready");
        return;
    };

    info!("prepare_particle_buffers: enabled={}, particle_count={}",
          extracted_data.enabled,
          extracted_data.particles.len());

    if !extracted_data.enabled || extracted_data.particles.is_empty() {
        commands.remove_resource::<ParticleBuffers>();
        warn!("Removing ParticleBuffers: enabled={}, particles={}",
              extracted_data.enabled,
              extracted_data.particles.len());
        return;
    }

    info!("Creating ParticleBuffers for {} particles", extracted_data.particles.len());

    // Convertir les particules en format GPU
    let gpu_particles: Vec<GpuParticle> = extracted_data.particles
        .iter()
        .map(|(_, pos, vel, p_type, sim_id)| GpuParticle {
            position: pos.to_array(),
            _padding1: 0.0,
            velocity: vel.to_array(),
            particle_type: *p_type as u32,
            simulation_id: *sim_id as u32,
            _padding2: [0.0; 3],
        })
        .collect();

    // Créer le buffer des particules (entrée)
    let particle_buffer_in = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Particle Buffer In"),
        contents: bytemuck::cast_slice(&gpu_particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    // Créer le buffer des particules (sortie)
    let particle_buffer_out = render_device.create_buffer(&BufferDescriptor {
        label: Some("Particle Buffer Out"),
        size: (std::mem::size_of::<GpuParticle>() * gpu_particles.len()) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Buffer des paramètres
    let params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Simulation Params Buffer"),
        contents: bytemuck::bytes_of(&extracted_data.params),
        usage: BufferUsages::UNIFORM,
    });

    // Buffer des génomes (4 u32 par simulation)
    let genome_data: Vec<u32> = extracted_data.genomes
        .iter()
        .flat_map(|(genome, food_genome)| {
            vec![
                *genome as u32,
                (*genome >> 32) as u32,
                *food_genome as u32,
                0u32, // padding
            ]
        })
        .collect();

    let genome_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Genome Buffer"),
        contents: bytemuck::cast_slice(&genome_data),
        usage: BufferUsages::STORAGE,
    });

    // Buffer de la nourriture
    let gpu_food: Vec<GpuFood> = extracted_data.food_positions
        .iter()
        .map(|(pos, active)| GpuFood {
            position: pos.to_array(),
            is_active: if *active { 1 } else { 0 },
        })
        .collect();

    let food_buffer = if !gpu_food.is_empty() {
        render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Food Buffer"),
            contents: bytemuck::cast_slice(&gpu_food),
            usage: BufferUsages::STORAGE,
        })
    } else {
        // Buffer vide si pas de nourriture
        render_device.create_buffer(&BufferDescriptor {
            label: Some("Food Buffer"),
            size: std::mem::size_of::<GpuFood>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    };

    // Buffer pour le nombre de nourritures
    let food_count = extracted_data.food_positions.len() as u32;
    let food_count_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Food Count Buffer"),
        contents: bytemuck::bytes_of(&food_count),
        usage: BufferUsages::UNIFORM,
    });

    // Créer le bind group
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

    commands.insert_resource(ParticleBuffers {
        particle_buffer_in,
        particle_buffer_out,
        params_buffer,
        genome_buffer,
        food_buffer,
        food_count_buffer,
        bind_group,
        particle_count: gpu_particles.len(),
    });

    info!("ParticleBuffers created successfully");
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

        let bind_group_layout = render_device.create_bind_group_layout(
            "Particle Compute Bind Group Layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // Particles in
                    storage_buffer_read_only::<GpuParticle>(false),
                    // Particles out
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

/// Nœud du graphe de rendu pour exécuter le compute shader
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
            warn!("No ParticleBuffers found in ParticleComputeNode");
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ParticleComputePipeline>();

        let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
            warn!("Compute pipeline not ready");
            return Ok(());
        };

        let extracted_data = world.resource::<ExtractedParticleData>();
        if !extracted_data.enabled || extracted_data.particles.is_empty() {
            warn!("No particles to compute or compute disabled");
            return Ok(());
        }

        info!("Running compute shader for {} particles", buffers.particle_count);

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, &buffers.bind_group, &[]);
        pass.set_pipeline(compute_pipeline);

        // Calculer le nombre de workgroups nécessaires
        let num_workgroups = (buffers.particle_count as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        pass.dispatch_workgroups(num_workgroups, 1, 1);

        Ok(())
    }
}

/// Système pour copier les résultats dans un buffer accessible
fn write_compute_results(
    buffers: Option<Res<ParticleBuffers>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut results_buffer: ResMut<ComputeResultsBuffer>,
    extracted_data: Res<ExtractedParticleData>,
    mut synced_results: ResMut<SyncedComputeResults>,
) {
    let Some(buffers) = buffers else { return; };

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

/// Système pour appliquer les résultats du compute shader aux entités
pub fn apply_compute_results(
    mut particles: Query<(&mut Transform, &mut Velocity), With<Particle>>,
    results: Res<SyncedComputeResults>,
    compute_enabled: Res<ComputeEnabled>,
) {
    if !compute_enabled.0 {
        info!("Compute disabled in apply_compute_results");
        return;
    }

    if results.data.is_empty() {
        info!("No compute results to apply");
        return;
    }

    info!("Applying compute results for {} particles", results.data.len());

    // Appliquer les résultats aux entités
    let mut applied_count = 0;
    for (entity, gpu_particle) in results.data.iter() {
        if let Ok((mut transform, mut velocity)) = particles.get_mut(*entity) {
            transform.translation = Vec3::from_array(gpu_particle.position);
            velocity.0 = Vec3::from_array(gpu_particle.velocity);
            applied_count += 1;
        }
    }

    info!("Applied results to {} particles", applied_count);
}