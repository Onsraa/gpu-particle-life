use bevy::prelude::*;
use crate::globals::FORCE_SCALE_FACTOR;

/// Génome encodé dans un entier
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Genotype {
    pub genome: u64, 
    pub type_count: usize,
}

impl Genotype {
    pub fn new(genome: u64, type_count: usize) -> Self {
        Self { genome, type_count }
    }

    /// Génère un génome aléatoire
    pub fn random(type_count: usize) -> Self {
        let genome = rand::random::<u64>();
        Self { genome, type_count }
    }

    /// Décode la force d'interaction entre deux types
    /// Retourne une valeur entre -1.0 et 1.0 multipliée par FORCE_SCALE_FACTOR
    pub fn decode_force(&self, type_a: usize, type_b: usize) -> f32 {
        let interactions = self.type_count * self.type_count;
        let bits_per_interaction = 64 / interactions.max(1);

        // Calcule l'index dans la matrice d'interaction
        let index = type_a * self.type_count + type_b;
        let bit_start = index * bits_per_interaction;

        // Protéger contre le dépassement
        if bit_start >= 64 {
            return 0.0;
        }

        // Extrait les bits correspondants
        let mask = (1u64 << bits_per_interaction) - 1;
        let raw_value = (self.genome >> bit_start) & mask;

        // Convertit en force normalisée entre -1 et 1
        let max_value = (1u64 << bits_per_interaction) - 1;
        let normalized = (raw_value as f32 / max_value as f32) * 2.0 - 1.0;

        normalized * FORCE_SCALE_FACTOR
    }

    /// Retourne une matrice de toutes les forces d'interaction
    pub fn get_force_matrix(&self) -> Vec<Vec<f32>> {
        let mut matrix = vec![vec![0.0; self.type_count]; self.type_count];

        for i in 0..self.type_count {
            for j in 0..self.type_count {
                matrix[i][j] = self.decode_force(i, j);
            }
        }

        matrix
    }
}