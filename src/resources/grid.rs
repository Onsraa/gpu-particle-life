use bevy::prelude::*;
use crate::globals::*;

#[derive(Resource)]
pub struct GridParameters {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

impl Default for GridParameters {
    fn default() -> Self {
        Self {
            width: DEFAULT_GRID_WIDTH,
            height: DEFAULT_GRID_HEIGHT,
            depth: DEFAULT_GRID_DEPTH,
        }
    }
}

impl GridParameters {
    /// Vérifie si une position est dans les limites de la grille
    pub fn is_in_bounds(&self, position: Vec3) -> bool {
        let half_width = self.width / 2.0;
        let half_height = self.height / 2.0;
        let half_depth = self.depth / 2.0;

        position.x.abs() <= half_width &&
            position.y.abs() <= half_height &&
            position.z.abs() <= half_depth
    }

    /// Applique les rebonds sur les murs si nécessaire
    pub fn apply_bounds(&self, position: &mut Vec3, velocity: &mut Vec3) {
        let half_width = self.width / 2.0;
        let half_height = self.height / 2.0;
        let half_depth = self.depth / 2.0;

        // Rebond sur les murs X
        if position.x.abs() > half_width - PARTICLE_RADIUS {
            position.x = position.x.signum() * (half_width - PARTICLE_RADIUS);
            velocity.x *= -COLLISION_DAMPING;
        }

        // Rebond sur les murs Y
        if position.y.abs() > half_height - PARTICLE_RADIUS {
            position.y = position.y.signum() * (half_height - PARTICLE_RADIUS);
            velocity.y *= -COLLISION_DAMPING;
        }

        // Rebond sur les murs Z
        if position.z.abs() > half_depth - PARTICLE_RADIUS {
            position.z = position.z.signum() * (half_depth - PARTICLE_RADIUS);
            velocity.z *= -COLLISION_DAMPING;
        }
    }
}   