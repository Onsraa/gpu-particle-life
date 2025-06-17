use bevy::prelude::*;
const FOOD_SCORE: f32 = 1.0;

#[derive(Component)]
struct Food(f32);

impl Default for Food {
    fn default() -> Self {
        Self(FOOD_SCORE)
    }
}
