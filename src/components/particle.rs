use bevy::prelude::*;

/// Type de particule (0, 1, 2, etc.)
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ParticleType(pub usize);

/// Vélocité de la particule
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Velocity(pub Vec3);

/// Marqueur pour identifier une particule
#[derive(Component)]
#[require(ParticleType, Velocity, Transform, Mesh3d, MeshMaterial3d<StandardMaterial>)]
pub struct Particle;