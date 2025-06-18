use bevy::prelude::*;

use crate::components::{
    particle::Particle,
    food::Food,
    score::Score,
    simulation::Simulation,
};
use crate::globals::*;

/// Détecte les collisions entre particules et nourriture
pub fn detect_food_collision(
    mut commands: Commands,
    time: Res<Time>,
    particles: Query<(&Transform, &Parent), With<Particle>>,
    mut food_query: Query<(Entity, &Transform, &mut Food, &ViewVisibility)>,
    mut simulations: Query<&mut Score, With<Simulation>>,
) {
    // Pour chaque nourriture
    for (food_entity, food_transform, mut food, visibility) in food_query.iter_mut() {
        // Si la nourriture a un timer de respawn actif
        if let Some(ref mut timer) = food.respawn_timer {
            if timer.finished() {
                // La nourriture réapparaît
                timer.reset();
                commands.entity(food_entity).insert(Visibility::Visible);
            } else if !visibility.get() {
                // Timer en cours et nourriture cachée, passer à la suivante
                timer.tick(time.delta());
                continue;
            }
        }

        let food_pos = food_transform.translation;
        let collision_distance = PARTICLE_RADIUS + FOOD_RADIUS;

        // Vérifier collision avec chaque particule
        for (particle_transform, parent) in particles.iter() {
            let distance = (particle_transform.translation - food_pos).length();

            if distance < collision_distance {
                // Collision détectée !
                // Augmenter le score de la simulation parente
                if let Ok(mut score) = simulations.get_mut(parent.get()) {
                    score.add(food.value);
                }

                // Gérer la nourriture
                if food.respawn_timer.is_some() {
                    // Si respawn activé, cacher la nourriture
                    commands.entity(food_entity).insert(Visibility::Hidden);
                    if let Some(ref mut timer) = food.respawn_timer {
                        timer.reset();
                    }
                } else {
                    // Sinon, détruire la nourriture
                    commands.entity(food_entity).despawn();
                }

                // Une seule particule peut manger cette nourriture
                break;
            }
        }
    }
}