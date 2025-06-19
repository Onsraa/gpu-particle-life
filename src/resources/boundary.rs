use bevy::prelude::*;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMode {
    #[default]
    Bounce,     // Rebondir sur les murs
    Teleport,   // Téléporter de l'autre côté
}