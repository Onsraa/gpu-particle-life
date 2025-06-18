use bevy::prelude::*;

#[derive(Component, Default, Debug, Clone)]
pub struct Score(pub f32);

impl Score {
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    pub fn add(&mut self, value: f32) {
        self.0 += value;
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}