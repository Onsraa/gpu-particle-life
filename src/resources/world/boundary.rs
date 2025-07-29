use bevy::prelude::*;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMode {
    #[default]
    Bounce,
    Teleport,
}