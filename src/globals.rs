pub const DEFAULT_PARTICLE_COUNT: usize = 500;
pub const DEFAULT_PARTICLE_TYPES: usize = 3;
pub const DEFAULT_SIMULATION_COUNT: usize = 3;
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
pub const FORCE_SCALE_FACTOR: f32 = 1000.0;
pub const MIN_DISTANCE: f32 = 100.0; // distance minimale pour éviter les singularités
pub const PARTICLE_REPULSION_STRENGTH: f32 = 100.0;

// Paramètres génétiques
pub const MUTATION_RATE: f32 = 0.1;
pub const ELITE_RATIO: f32 = 0.2; // top 20% des génomes gardés
pub const CROSSOVER_RATE: f32 = 0.7;

// Paramètres de rendu
pub const PARTICLE_SUBDIVISIONS: u32 = 8;