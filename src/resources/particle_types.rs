use bevy::prelude::*;
use crate::globals::*;

#[derive(Resource)]
pub struct ParticleTypesConfig {
    pub type_count: usize,
    pub colors: Vec<Color>, // Couleur par type
}

impl Default for ParticleTypesConfig {
    fn default() -> Self {
        Self {
            type_count: DEFAULT_PARTICLE_TYPES,
            colors: Self::generate_colors(DEFAULT_PARTICLE_TYPES),
        }
    }
}

impl ParticleTypesConfig {
    pub fn new(type_count: usize) -> Self {
        Self {
            type_count,
            colors: Self::generate_colors(type_count),
        }
    }

    /// Génère des couleurs distinctes pour chaque type
    fn generate_colors(count: usize) -> Vec<Color> {
        (0..count)
            .map(|i| {
                // Utilise l'espace HSL pour des couleurs bien distinctes
                let hue = (i as f32 / count as f32) * 360.0;
                Color::hsl(hue, 0.8, 0.6)
            })
            .collect()
    }
}