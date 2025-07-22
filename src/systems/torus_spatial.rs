use bevy::prelude::*;
use bevy_spatial::{SpatialAccess, AutomaticUpdate, TransformMode};
use std::collections::HashMap;
use bevy_spatial::kdtree::KDTree3;
use crate::components::{
    particle::{Particle, ParticleType},
    simulation::{Simulation, SimulationId},
};
use crate::resources::{
    grid::GridParameters,
    boundary::BoundaryMode,
};

/// Marqueur pour les particules suivies par le KDTree
#[derive(Component, Default)]
pub struct TrackedParticle;

/// Alias pour le KDTree 3D utilisé dans notre simulation
pub type ParticleTree = KDTree3<TrackedParticle>;

/// Ressource pour stocker les interactions par simulation dans un espace torus
#[derive(Resource, Default)]
pub struct TorusNeighborCache {
    /// Cache des voisins par simulation et particule
    pub neighbors: HashMap<(usize, Entity), Vec<(Entity, Vec3, usize)>>,
    /// Paramètres de la grille pour les calculs de torus
    pub grid_bounds: Option<(f32, f32, f32)>, // (width, height, depth)
    /// Distance maximale pour la recherche de voisins
    pub max_search_distance: f32,
}

impl TorusNeighborCache {
    /// Met à jour les paramètres de grille
    pub fn update_grid_bounds(&mut self, width: f32, height: f32, depth: f32) {
        self.grid_bounds = Some((width, height, depth));
        // Vider le cache car les distances changent
        self.neighbors.clear();
        info!("🌐 Cache spatial mis à jour pour grille {}x{}x{}", width, height, depth);
    }

    /// Calcule la distance minimale dans un espace torus 3D
    pub fn torus_distance(&self, pos1: Vec3, pos2: Vec3) -> f32 {
        let Some((width, height, depth)) = self.grid_bounds else {
            // Fallback sur distance euclidienne normale si pas de grille
            return pos1.distance(pos2);
        };

        // Calcul de la distance minimale sur chaque axe (torus 1D)
        let dx = (pos2.x - pos1.x).abs();
        let min_dx = dx.min(width - dx);

        let dy = (pos2.y - pos1.y).abs();
        let min_dy = dy.min(height - dy);

        let dz = (pos2.z - pos1.z).abs();
        let min_dz = dz.min(depth - dz);

        // Distance euclidienne 3D avec les distances minimales
        (min_dx.powi(2) + min_dy.powi(2) + min_dz.powi(2)).sqrt()
    }

    /// Calcule le vecteur de direction minimal dans un espace torus 3D
    pub fn torus_direction_vector(&self, from: Vec3, to: Vec3) -> Vec3 {
        let Some((width, height, depth)) = self.grid_bounds else {
            return to - from;
        };

        let mut direction = Vec3::ZERO;

        // Axe X
        let dx = to.x - from.x;
        if dx.abs() <= width / 2.0 {
            direction.x = dx;
        } else {
            // Plus court de passer par l'autre côté
            direction.x = if dx > 0.0 { dx - width } else { dx + width };
        }

        // Axe Y
        let dy = to.y - from.y;
        if dy.abs() <= height / 2.0 {
            direction.y = dy;
        } else {
            direction.y = if dy > 0.0 { dy - height } else { dy + height };
        }

        // Axe Z
        let dz = to.z - from.z;
        if dz.abs() <= depth / 2.0 {
            direction.z = dz;
        } else {
            direction.z = if dz > 0.0 { dz - depth } else { dz + depth };
        }

        direction
    }

    /// Génère les positions "fantômes" d'une particule dans l'espace torus
    /// Retourne jusqu'à 27 positions (la particule + ses 26 images miroirs)
    fn generate_torus_positions(&self, position: Vec3) -> Vec<Vec3> {
        let Some((width, height, depth)) = self.grid_bounds else {
            return vec![position];
        };

        let mut positions = Vec::new();

        // Générer toutes les combinaisons d'offsets (-1, 0, +1) pour chaque dimension
        for x_offset in -1..=1 {
            for y_offset in -1..=1 {
                for z_offset in -1..=1 {
                    let ghost_pos = Vec3::new(
                        position.x + (x_offset as f32) * width,
                        position.y + (y_offset as f32) * height,
                        position.z + (z_offset as f32) * depth,
                    );
                    positions.push(ghost_pos);
                }
            }
        }

        positions
    }
}

/// Plugin pour gérer le système spatial avec torus
pub struct TorusSpatialPlugin;

impl Plugin for TorusSpatialPlugin {
    fn build(&self, app: &mut App) {
        app
            // Ressources
            .init_resource::<TorusNeighborCache>()

            .add_plugins(
                AutomaticUpdate::<TrackedParticle>::new()
                    .with_frequency(std::time::Duration::from_millis(50))
                    .with_transform(TransformMode::GlobalTransform)
            )

            // Systèmes
            .add_systems(
                Update,
                (
                    assign_tracked_component,
                    update_torus_cache,
                ).chain()
            );
    }
}

/// Assigne le composant TrackedParticle aux particules qui n'en ont pas
fn assign_tracked_component(
    mut commands: Commands,
    particles: Query<Entity, (With<Particle>, Without<TrackedParticle>)>,
) {
    for entity in particles.iter() {
        commands.entity(entity).insert(TrackedParticle);
    }
}

/// Met à jour le cache des voisins pour l'espace torus
fn update_torus_cache(
    mut neighbor_cache: ResMut<TorusNeighborCache>,
    grid_params: Res<GridParameters>,
    boundary_mode: Res<BoundaryMode>,
    tree: Res<ParticleTree>,
    simulations: Query<&SimulationId>, 
    particles: Query<(Entity, &Transform, &ParticleType, &ChildOf), With<Particle>>,
) {
    // Mettre à jour les paramètres de grille si changés
    if grid_params.is_changed() {
        neighbor_cache.update_grid_bounds(
            grid_params.width,
            grid_params.height,
            grid_params.depth
        );
    }

    // Ne traiter le torus qu'en mode Teleport
    if *boundary_mode != BoundaryMode::Teleport {
        // En mode Bounce, utiliser la recherche normale
        neighbor_cache.neighbors.clear();
        return;
    }

    neighbor_cache.neighbors.clear();

    let max_search_radius = neighbor_cache.max_search_distance;

    // Pour chaque particule, trouver ses voisins dans l'espace torus
    for (entity, transform, particle_type, parent) in particles.iter() {
        let Ok(sim_id) = simulations.get(parent.parent()) else { continue; }; // CORRECTION

        let mut torus_neighbors = Vec::new();
        let position = transform.translation;

        // Générer toutes les positions fantômes de cette particule
        let ghost_positions = neighbor_cache.generate_torus_positions(position);

        // Pour chaque position fantôme, chercher les voisins dans le KDTree
        for ghost_pos in ghost_positions {
            for (neighbor_pos, neighbor_entity) in tree.within_distance(ghost_pos, max_search_radius) {
                let Some(neighbor_entity) = neighbor_entity else { continue; };

                // Éviter l'auto-interaction
                if neighbor_entity == entity {
                    continue;
                }

                // Vérifier que le voisin est dans la même simulation
                if let Ok((_, neighbor_transform, neighbor_type, neighbor_parent)) =
                    particles.get(neighbor_entity) {

                    if let Ok(neighbor_sim_id) = simulations.get(neighbor_parent.parent()) { // CORRECTION
                        if neighbor_sim_id.0 != sim_id.0 {
                            continue;
                        }

                        // Calculer la vraie distance torus
                        let torus_distance = neighbor_cache.torus_distance(
                            position,
                            neighbor_transform.translation
                        );

                        // Ajouter seulement si dans la portée et pas déjà ajouté
                        if torus_distance <= max_search_radius &&
                            !torus_neighbors.iter().any(|(e, _, _)| *e == neighbor_entity) {
                            torus_neighbors.push((
                                neighbor_entity,
                                neighbor_transform.translation,
                                neighbor_type.0
                            ));
                        }
                    }
                }
            }
        }

        // Stocker dans le cache
        neighbor_cache.neighbors.insert((sim_id.0, entity), torus_neighbors);
    }
}

/// Fonction utilitaire pour récupérer les voisins d'une particule dans l'espace torus
pub fn get_torus_neighbors(
    cache: &TorusNeighborCache,
    tree: &ParticleTree,
    sim_id: usize,
    entity: Entity,
    position: Vec3,
    max_distance: f32,
    boundary_mode: BoundaryMode,
) -> Vec<(Entity, Vec3, usize)> {
    match boundary_mode {
        BoundaryMode::Teleport => {
            // Utiliser le cache torus si disponible
            if let Some(cached_neighbors) = cache.neighbors.get(&(sim_id, entity)) {
                cached_neighbors.clone()
            } else {
                // Fallback sur recherche normale
                tree.within_distance(position, max_distance)
                    .into_iter()
                    .filter_map(|(pos, ent_opt)| {
                        if let Some(ent) = ent_opt {
                            if ent != entity {
                                Some((ent, pos, 0)) // Type sera récupéré ailleurs
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        }
        BoundaryMode::Bounce => {
            // Mode bounce : recherche normale dans le KDTree
            tree.within_distance(position, max_distance)
                .into_iter()
                .filter_map(|(pos, ent_opt)| {
                    if let Some(ent) = ent_opt {
                        if ent != entity {
                            Some((ent, pos, 0)) // Type sera récupéré ailleurs
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        }
    }
}