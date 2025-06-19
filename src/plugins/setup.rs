use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use crate::resources::{
    grid::GridParameters,
    simulation::SimulationParameters,
    particle_types::ParticleTypesConfig,
    food::FoodParameters,
};
use crate::resources::boundary::BoundaryMode;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app
            // Initialiser toutes les ressources
            .init_resource::<GridParameters>()
            .init_resource::<SimulationParameters>()
            .init_resource::<ParticleTypesConfig>()
            .init_resource::<FoodParameters>()
            .init_resource::<BoundaryMode>()

            // Systèmes de setup
            .add_systems(Startup, (
                setup_grid_visualization,
            ));
    }
}

fn setup_grid_visualization(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid: Res<GridParameters>,
) {
    // Créer un cube wireframe pour visualiser les limites
    let grid_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.3, 0.3, 0.1),
        emissive: LinearRgba::rgb(0.3, 0.3, 0.3),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(grid.width, grid.height, grid.depth))),
        MeshMaterial3d(grid_material),
        Transform::from_translation(Vec3::ZERO),
        // Layer 0 pour être visible par toutes les caméras
        RenderLayers::layer(0),
    ));
}