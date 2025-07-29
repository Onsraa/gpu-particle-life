use bevy::prelude::*;

/// Valeur nutritive de la nourriture
#[derive(Component)]
pub struct FoodValue(pub f32);

impl Default for FoodValue {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Timer de respawn pour la nourriture
#[derive(Component)]
pub struct FoodRespawnTimer(pub Option<Timer>);

impl Default for FoodRespawnTimer {
    fn default() -> Self {
        Self(Some(Timer::from_seconds(5.0, TimerMode::Once)))
    }
}

/// Marqueur pour la nourriture
#[derive(Component)]
#[require(FoodValue, FoodRespawnTimer, Transform, Mesh3d, MeshMaterial3d<StandardMaterial>)]
pub struct Food;
