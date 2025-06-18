use bevy::prelude::*;
use crate::globals::*;

#[derive(Resource)]
pub struct FoodParameters {
    pub food_count: usize,
    pub respawn_enabled: bool,
    pub respawn_cooldown: f32,
    pub food_value: f32,
}

impl Default for FoodParameters {
    fn default() -> Self {
        Self {
            food_count: DEFAULT_FOOD_COUNT,
            respawn_enabled: true,
            respawn_cooldown: DEFAULT_FOOD_RESPAWN_TIME,
            food_value: DEFAULT_FOOD_VALUE,
        }
    }
}