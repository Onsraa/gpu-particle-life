pub const DEFAULT_PARTICLE_COUNT: usize = 100;
pub const DEFAULT_PARTICLE_TYPES: usize = 3;
pub const DEFAULT_SIMULATION_COUNT: usize = 6;
pub const DEFAULT_EPOCH_DURATION: f32 = 60.0; // secondes
pub const DEFAULT_PARTICLES_PER_TYPE: usize = DEFAULT_PARTICLE_COUNT / DEFAULT_PARTICLE_TYPES;

/// Timestep fixe pour la physique (60 FPS) - indépendant de la vitesse de simulation
pub const PHYSICS_TIMESTEP: f32 = 0.008;

// Paramètres de la grille
pub const DEFAULT_GRID_WIDTH: f32 = 800.0;
pub const DEFAULT_GRID_HEIGHT: f32 = 800.0;
pub const DEFAULT_GRID_DEPTH: f32 = 800.0;

// Paramètres de la nourriture
pub const DEFAULT_FOOD_COUNT: usize = 50;
pub const DEFAULT_FOOD_RESPAWN_TIME: f32 = 5.0; // secondes
pub const DEFAULT_FOOD_VALUE: f32 = 1.0;
pub const FOOD_RADIUS: f32 = 2.0;

// Paramètres des particules
pub const PARTICLE_RADIUS: f32 = 4.0;
pub const PARTICLE_MASS: f32 = 1.0;
pub const MAX_VELOCITY: f32 = 200.0;
pub const COLLISION_DAMPING: f32 = 0.5;

// Paramètres des forces
pub const DEFAULT_MAX_FORCE_RANGE: f32 = 300.0;

pub const FORCE_SCALE_FACTOR: f32 = 80.0;

pub const MIN_DISTANCE: f32 = 0.001;
pub const PARTICLE_REPULSION_STRENGTH: f32 = 100.0;

// Paramètres génétiques OPTIMISÉS pour préserver les stratégies cohérentes
pub const DEFAULT_ELITE_RATIO: f32 = 0.3; // 30% des génomes gardés (vs 10% avant)
pub const DEFAULT_MUTATION_RATE: f32 = 0.15; // 15% de chance de mutation (vs 10% avant)
pub const DEFAULT_CROSSOVER_RATE: f32 = 0.25; // 25% de crossover (vs 70% avant)

// Nouveaux paramètres pour la validation de cohérence
pub const MIN_STRATEGY_COHERENCE: f32 = 0.3; // Seuil minimum de cohérence acceptable
pub const DIVERSITY_INJECTION_RATE: f32 = 0.1; // 10% d'injection si faible diversité
pub const LIGHT_ELITE_MUTATION: f32 = 0.01; // Mutation très légère des élites

// Paramètres de rendu
pub const PARTICLE_SUBDIVISIONS: u32 = 8;