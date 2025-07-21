pub const DEFAULT_PARTICLE_COUNT: usize = 500;
pub const DEFAULT_PARTICLE_TYPES: usize = 3;
pub const DEFAULT_SIMULATION_COUNT: usize = 4;
pub const DEFAULT_EPOCH_DURATION: f32 = 60.0; // secondes
pub const DEFAULT_PARTICLES_PER_TYPE: usize = DEFAULT_PARTICLE_COUNT / DEFAULT_PARTICLE_TYPES;

// Paramètres de la grille
pub const DEFAULT_GRID_WIDTH: f32 = 400.0;
pub const DEFAULT_GRID_HEIGHT: f32 = 400.0;
pub const DEFAULT_GRID_DEPTH: f32 = 400.0;

// Paramètres de la nourriture
pub const DEFAULT_FOOD_COUNT: usize = 50;
pub const DEFAULT_FOOD_RESPAWN_TIME: f32 = 5.0; // secondes
pub const DEFAULT_FOOD_VALUE: f32 = 1.0;
pub const FOOD_RADIUS: f32 = 1.0;

// Paramètres des particules
pub const PARTICLE_RADIUS: f32 = 2.5;
pub const PARTICLE_MASS: f32 = 1.0;
pub const MAX_VELOCITY: f32 = 200.0;
pub const COLLISION_DAMPING: f32 = 0.5;

// Paramètres des forces
pub const DEFAULT_MAX_FORCE_RANGE: f32 = 100.0;

pub const FORCE_SCALE_FACTOR: f32 = 80.0;

pub const MIN_DISTANCE: f32 = 0.001;
pub const PARTICLE_REPULSION_STRENGTH: f32 = 100.0;

// Paramètres génétiques
pub const DEFAULT_ELITE_RATIO: f32 = 0.1; // 10% des génomes gardés
pub const DEFAULT_MUTATION_RATE: f32 = 0.1; // 10% de chance de mutation
pub const DEFAULT_CROSSOVER_RATE: f32 = 0.7; // 70% de crossover

// Paramètres de rendu
pub const PARTICLE_SUBDIVISIONS: u32 = 8;