use bevy::prelude::*;
use std::{f32::consts::FRAC_PI_2, ops::Range};

#[derive(Debug, Resource)]
pub struct CameraSettings {
    pub orbit_distance: f32,
    pub pitch_speed: f32,
    pub pitch_range: Range<f32>,
    pub roll_speed: f32,
    pub yaw_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        let pitch_limit = FRAC_PI_2 - 0.01;
        Self {
            // CHANGEMENT : Distance par défaut adaptée aux grilles plus grandes
            // Cette valeur sera mise à jour automatiquement selon la taille de la grille
            orbit_distance: 800.0, // Augmenté de 1000 à 800 pour les grilles par défaut de 400

            // Vitesses d'orbite optimisées pour les nouvelles distances
            pitch_speed: 0.003,
            pitch_range: -pitch_limit..pitch_limit,
            roll_speed: 1.0,
            yaw_speed: 0.003, // Légèrement réduit pour plus de fluidité
        }
    }
}

impl CameraSettings {
    pub fn update_for_grid(&mut self, grid_width: f32, grid_height: f32, grid_depth: f32) {
        // Calculer la diagonale 3D de la grille
        let diagonal_3d = (grid_width.powi(2) + grid_height.powi(2) + grid_depth.powi(2)).sqrt();

        // Distance optimale pour orbiter autour de la grille
        self.orbit_distance = diagonal_3d * 0.85;

        // Ajuster légèrement les vitesses selon la distance
        let distance_factor = (self.orbit_distance / 800.0).clamp(0.5, 2.0);
        self.pitch_speed = 0.003 / distance_factor.sqrt();
        self.yaw_speed = 0.003 / distance_factor.sqrt();

        info!("📐 Paramètres caméra adaptés : distance={:.0}, vitesses={:.4}",
              self.orbit_distance, self.pitch_speed);
    }
}