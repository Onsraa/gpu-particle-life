use bevy::prelude::*;
use crate::globals::FORCE_SCALE_FACTOR;

/// Génome encodé dans un entier avec forces de nourriture
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Genotype {
    pub genome: u64,
    pub type_count: usize,
    /// Force exercée par la nourriture sur chaque type de particule
    /// Encodé dans les 16 bits de poids fort du génome
    pub food_force_genome: u16,
}

impl Genotype {
    pub fn new(genome: u64, type_count: usize, food_force_genome: u16) -> Self {
        Self { genome, type_count, food_force_genome }
    }

    /// Génère un génome aléatoire avec forces de nourriture
    pub fn random(type_count: usize) -> Self {
        let genome = rand::random::<u64>();
        let food_force_genome = rand::random::<u16>();
        Self { genome, type_count, food_force_genome }
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

    /// Décode la force exercée par la nourriture sur un type de particule
    /// Retourne une valeur entre -1.0 et 1.0 multipliée par FORCE_SCALE_FACTOR
    pub fn decode_food_force(&self, particle_type: usize) -> f32 {
        // On répartit les 16 bits entre les types de particules
        let bits_per_type = 16 / self.type_count.max(1);
        let bit_start = particle_type * bits_per_type;

        if bit_start >= 16 {
            return 0.0;
        }

        // Extrait les bits
        let mask = (1u16 << bits_per_type) - 1;
        let raw_value = (self.food_force_genome >> bit_start) & mask;

        // Normalise entre -1 et 1
        let max_value = (1u16 << bits_per_type) - 1;
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

    /// Retourne un vecteur des forces de nourriture pour chaque type
    pub fn get_food_forces(&self) -> Vec<f32> {
        (0..self.type_count)
            .map(|i| self.decode_food_force(i))
            .collect()
    }
}