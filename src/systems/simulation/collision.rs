use bevy::prelude::*;
use crate::components::entities::food::{Food, FoodRespawnTimer, FoodValue};
use crate::components::entities::particle::Particle;
use crate::components::entities::simulation::Simulation;
use crate::components::genetics::score::Score;
use crate::globals::*;

/// Détecte les collisions entre particules et nourriture
pub fn detect_food_collision(
    mut commands: Commands,
    time: Res<Time>,
    particles: Query<(&Transform, &ChildOf), With<Particle>>,
    mut food_query: Query<
        (
            Entity,
            &Transform,
            &FoodValue,
            &mut FoodRespawnTimer,
            &ViewVisibility,
        ),
        With<Food>,
    >,
    mut simulations: Query<&mut Score, With<Simulation>>,
) {
    // Pour chaque nourriture
    for (food_entity, food_transform, food_value, mut respawn_timer, visibility) in
        food_query.iter_mut()
    {
        // Si la nourriture a un timer de respawn actif
        if let Some(ref mut timer) = respawn_timer.0 {
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
                if let Ok(mut score) = simulations.get_mut(parent.parent()) {
                    score.add(food_value.0);
                }

                // Gérer la nourriture
                if respawn_timer.0.is_some() {
                    // Si respawn activé, cacher la nourriture
                    commands.entity(food_entity).insert(Visibility::Hidden);
                    if let Some(ref mut timer) = respawn_timer.0 {
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
