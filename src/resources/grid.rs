use bevy::prelude::*;

const GRID_WIDTH: f32 = 400.0;
const GRID_HEIGHT: f32 = 400.0;
const GRID_DEPTH: f32 = 400.0;
#[derive(Resource)]
pub struct GridParameters {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

impl Default for GridParameters {
    fn default() -> Self {
        Self {
            width: GRID_WIDTH,
            height: GRID_HEIGHT,
            depth: GRID_DEPTH,
        }
    }
}