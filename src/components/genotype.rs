use bevy::prelude::*;
use crate::globals::FORCE_SCALE_FACTOR;
use rand::Rng;

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

    /// Décode la force d'interaction entre deux types (entre -1.0 et 1.0)
    pub fn decode_force(&self, type_a: usize, type_b: usize) -> f32 {
        let interactions = self.type_count * self.type_count;
        // Avec 5 types max, on a 25 interactions, donc 64/25 = 2.56 bits par interaction
        let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8); // Entre 2 et 8 bits

        // Calcule l'index dans la matrice d'interaction
        let index = type_a * self.type_count + type_b;
        let bit_start = index * bits_per_interaction;

        // Protéger contre le dépassement
        if bit_start >= 64 || bit_start + bits_per_interaction > 64 {
            return 0.0;
        }

        // Extrait les bits correspondants
        let mask = (1u64 << bits_per_interaction) - 1;
        let raw_value = (self.genome >> bit_start) & mask;

        // Avec n bits, on a 2^n valeurs possibles
        let max_value = (1u64 << bits_per_interaction) - 1;

        // Normalisation linéaire entre -1 et 1
        let normalized = (raw_value as f32 / max_value as f32) * 2.0 - 1.0;

        // Pour avoir plus de variété, on utilise une distribution non-linéaire
        // qui favorise les valeurs moyennes tout en permettant des extrêmes
        let shaped = normalized.signum() * normalized.abs().powf(0.7);

        // Arrondir à 3 décimales
        (shaped * 1000.0).round() / 1000.0
    }

    /// Décode la force exercée par la nourriture sur un type de particule (entre -1.0 et 1.0)
    pub fn decode_food_force(&self, particle_type: usize) -> f32 {
        // Avec max 5 types, on a au moins 3 bits par type (16/5 = 3.2)
        let bits_per_type = (16 / self.type_count.max(1)).max(3).min(8);
        let bit_start = particle_type * bits_per_type;

        if bit_start >= 16 || bit_start + bits_per_type > 16 {
            return 0.0;
        }

        // Extrait les bits
        let mask = (1u16 << bits_per_type) - 1;
        let raw_value = (self.food_force_genome >> bit_start) & mask;

        // Normalise entre -1 et 1
        let max_value = (1u16 << bits_per_type) - 1;
        let normalized = (raw_value as f32 / max_value as f32) * 2.0 - 1.0;

        // Même transformation que pour les forces particule-particule
        let shaped = normalized.signum() * normalized.abs().powf(0.7);

        // Arrondir à 3 décimales
        (shaped * 1000.0).round() / 1000.0
    }

    /// Retourne la force avec le facteur d'échelle appliqué (pour le calcul physique)
    pub fn get_scaled_force(&self, type_a: usize, type_b: usize) -> f32 {
        self.decode_force(type_a, type_b) * FORCE_SCALE_FACTOR
    }

    /// Retourne la force de nourriture avec le facteur d'échelle appliqué
    pub fn get_scaled_food_force(&self, particle_type: usize) -> f32 {
        self.decode_food_force(particle_type) * FORCE_SCALE_FACTOR
    }

    /// Retourne une matrice de toutes les forces d'interaction (normalisées)
    pub fn get_force_matrix(&self) -> Vec<Vec<f32>> {
        let mut matrix = vec![vec![0.0; self.type_count]; self.type_count];

        for i in 0..self.type_count {
            for j in 0..self.type_count {
                matrix[i][j] = self.decode_force(i, j);
            }
        }

        matrix
    }

    /// Retourne un vecteur des forces de nourriture pour chaque type (normalisées)
    pub fn get_food_forces(&self) -> Vec<f32> {
        (0..self.type_count)
            .map(|i| self.decode_food_force(i))
            .collect()
    }

    /// Effectue un crossover entre deux génomes
    pub fn crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        // Crossover uniforme : chaque bit a 50% de chance de venir de chaque parent
        let mut new_genome = 0u64;
        for i in 0..64 {
            let bit = if rng.random_bool(0.5) {
                (self.genome >> i) & 1
            } else {
                (other.genome >> i) & 1
            };
            new_genome |= bit << i;
        }

        // Crossover pour le génome de nourriture
        let mut new_food_genome = 0u16;
        for i in 0..16 {
            let bit = if rng.random_bool(0.5) {
                (self.food_force_genome >> i) & 1
            } else {
                (other.food_force_genome >> i) & 1
            };
            new_food_genome |= bit << i;
        }

        Self::new(new_genome, self.type_count, new_food_genome)
    }

    /// Applique une mutation au génome
    pub fn mutate(&mut self, mutation_rate: f32, rng: &mut impl Rng) {
        // Mutation du génome principal
        let interactions = self.type_count * self.type_count;
        let bits_per_interaction = (64 / interactions.max(1)).max(2).min(8);

        // Pour chaque interaction, chance de mutation
        for i in 0..interactions {
            if rng.random::<f32>() < mutation_rate {
                let bit_start = i * bits_per_interaction;
                if bit_start < 64 && bit_start + bits_per_interaction <= 64 {
                    // Mutation : inverser 1 ou 2 bits aléatoires
                    let bits_to_flip = rng.random_range(1..=2.min(bits_per_interaction));
                    for _ in 0..bits_to_flip {
                        let bit_offset = rng.random_range(0..bits_per_interaction);
                        let bit_position = bit_start + bit_offset;
                        if bit_position < 64 {
                            self.genome ^= 1u64 << bit_position;
                        }
                    }
                }
            }
        }

        // Mutation du génome de nourriture (avec taux réduit)
        if rng.random::<f32>() < mutation_rate * 0.5 {
            let bits_per_type = (16 / self.type_count.max(1)).max(3).min(8);
            let type_to_mutate = rng.random_range(0..self.type_count);
            let bit_start = type_to_mutate * bits_per_type;

            if bit_start < 16 && bit_start + bits_per_type <= 16 {
                let bit_offset = rng.random_range(0..bits_per_type);
                let bit_position = bit_start + bit_offset;
                if bit_position < 16 {
                    self.food_force_genome ^= 1u16 << bit_position;
                }
            }
        }
    }
}