use bevy::prelude::*;
use crate::globals::*;

#[derive(Resource)]
pub struct ParticleTypesConfig {
    pub type_count: usize,
    pub colors: Vec<(Color, LinearRgba)>, // (base_color, emissive)
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

    /// Génère des couleurs distinctes pour chaque type avec émissive
    fn generate_colors(count: usize) -> Vec<(Color, LinearRgba)> {
        (0..count)
            .map(|i| {
                let hue = (i as f32 / count as f32) * 360.0;
                let base_color = Color::hsl(hue, 0.8, 0.6);
                let emissive = base_color.to_linear() * 0.5; // Émission modérée
                (base_color, emissive)
            })
            .collect()
    }

    pub fn get_color_for_type(&self, type_index: usize) -> (Color, LinearRgba) {
        self.colors[type_index % self.colors.len()]
    }
}