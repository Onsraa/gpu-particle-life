use bevy::prelude::*;
use crate::components::{
    particle::{Particle, Velocity},
};

/// Système de debug pour vérifier que les particules bougent
pub fn debug_particle_movement(
    time: Res<Time>,
    mut timer: Local<Timer>,
    particles: Query<&Velocity, With<Particle>>,
) {
    // Initialiser le timer
    if timer.duration() == std::time::Duration::ZERO {
        *timer = Timer::from_seconds(5.0, TimerMode::Repeating);
    }

    timer.tick(time.delta());

    if timer.just_finished() {
        let velocities: Vec<f32> = particles.iter()
            .map(|v| v.0.length())
            .collect();

        if velocities.is_empty() {
            warn!("Aucune particule trouvée!");
            return;
        }

        let avg_velocity = velocities.iter().sum::<f32>() / velocities.len() as f32;
        let max_velocity = velocities.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0);
        let moving_count = velocities.iter().filter(|&&v| v > 0.1).count();

        info!("=== Debug Particules ===");
        info!("Particules en mouvement: {}/{}", moving_count, velocities.len());
        info!("Vitesse moyenne: {:.2}", avg_velocity);
        info!("Vitesse max: {:.2}", max_velocity);
    }
}