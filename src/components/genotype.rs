use bevy::prelude::*;
use rand::Rng;

/// Génome simplifié avec forces vectorisées
#[derive(Component, Clone, Debug, Default)]
pub struct Genotype {
    pub force_matrix: Vec<f32>,  // Matrice des forces particule-particule
    pub food_forces: Vec<f32>,   // Forces de nourriture par type
    pub type_count: usize,
}

impl Genotype {
    pub fn new(type_count: usize) -> Self {
        let matrix_size = type_count * type_count;
        Self {
            force_matrix: vec![0.0; matrix_size],
            food_forces: vec![0.0; type_count],
            type_count,
        }
    }

    /// Génère un génome aléatoire
    pub fn random(type_count: usize) -> Self {
        let mut rng = rand::rng();
        let matrix_size = type_count * type_count;

        let force_matrix = (0..matrix_size)
            .map(|i| {
                let type_a = i / type_count;
                let type_b = i % type_count;

                if type_a == type_b {
                    // Auto-répulsion pour éviter l'agglomération
                    rng.random_range(-1.0..=-0.1)
                } else {
                    // Forces variées entre types différents
                    rng.random_range(-1.0..=1.0)
                }
            })
            .collect();

        let food_forces = (0..type_count)
            .map(|_| rng.random_range(-1.0..=1.0))
            .collect();

        Self {
            force_matrix,
            food_forces,
            type_count,
        }
    }

    /// Obtient la force entre deux types
    pub fn get_force(&self, type_a: usize, type_b: usize) -> f32 {
        let index = type_a * self.type_count + type_b;
        self.force_matrix.get(index).copied().unwrap_or(0.0)
    }

    /// Définit la force entre deux types
    pub fn set_force(&mut self, type_a: usize, type_b: usize, force: f32) {
        let index = type_a * self.type_count + type_b;
        if index < self.force_matrix.len() {
            self.force_matrix[index] = force;
        }
    }

    /// Obtient la force de nourriture pour un type
    pub fn get_food_force(&self, particle_type: usize) -> f32 {
        self.food_forces.get(particle_type).copied().unwrap_or(0.0)
    }

    /// Crossover avec un autre génome
    pub fn crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        let mut new_force_matrix = Vec::with_capacity(self.force_matrix.len());
        let mut new_food_forces = Vec::with_capacity(self.food_forces.len());

        // Crossover uniforme pour la matrice des forces
        for i in 0..self.force_matrix.len() {
            if rng.random_bool(0.5) {
                new_force_matrix.push(self.force_matrix[i]);
            } else {
                new_force_matrix.push(other.force_matrix[i]);
            }
        }

        // Crossover uniforme pour les forces de nourriture
        for i in 0..self.food_forces.len() {
            if rng.random_bool(0.5) {
                new_food_forces.push(self.food_forces[i]);
            } else {
                new_food_forces.push(other.food_forces[i]);
            }
        }

        Self {
            force_matrix: new_force_matrix,
            food_forces: new_food_forces,
            type_count: self.type_count,
        }
    }

    /// Applique une mutation
    pub fn mutate(&mut self, mutation_rate: f32, rng: &mut impl Rng) {
        // Mutation de la matrice des forces
        for force in &mut self.force_matrix {
            if rng.random::<f32>() < mutation_rate {
                *force += rng.random_range(-0.2..=0.2);
                *force = force.clamp(-2.0, 2.0);
            }
        }

        // Mutation des forces de nourriture
        for force in &mut self.food_forces {
            if rng.random::<f32>() < mutation_rate * 0.5 {
                *force += rng.random_range(-0.2..=0.2);
                *force = force.clamp(-2.0, 2.0);
            }
        }
    }

    /// Retourne une matrice de toutes les forces d'interaction
    pub fn get_force_matrix(&self) -> Vec<Vec<f32>> {
        let mut matrix = vec![vec![0.0; self.type_count]; self.type_count];

        for i in 0..self.type_count {
            for j in 0..self.type_count {
                matrix[i][j] = self.get_force(i, j);
            }
        }

        matrix
    }

    /// Génère des forces intéressantes prédéfinies
    pub fn set_interesting_forces(&mut self) {
        // Efface les forces actuelles
        self.force_matrix.fill(0.0);
        self.food_forces.fill(0.0);

        match self.type_count {
            3 => {
                // Configuration rock-paper-scissors
                self.set_force(0, 1, 1.0);   // Rouge attire Vert
                self.set_force(1, 2, 1.0);   // Vert attire Bleu
                self.set_force(2, 0, 1.0);   // Bleu attire Rouge
                self.set_force(1, 0, -0.5);  // Vert repousse Rouge
                self.set_force(2, 1, -0.5);  // Bleu repousse Vert
                self.set_force(0, 2, -0.5);  // Rouge repousse Bleu

                // Auto-répulsion
                for i in 0..3 {
                    self.set_force(i, i, -0.3);
                }

                // Forces de nourriture variées
                self.food_forces = vec![0.8, -0.3, 0.5];
            },
            4 => {
                // Configuration plus complexe
                self.set_force(0, 1, 1.5);   // Rouge attire fort Vert
                self.set_force(1, 2, 0.8);   // Vert attire Bleu
                self.set_force(2, 3, 1.2);   // Bleu attire fort Jaune
                self.set_force(3, 0, 0.6);   // Jaune attire Rouge

                // Répulsions croisées
                self.set_force(0, 2, -1.0);  // Rouge repousse Bleu
                self.set_force(1, 3, -0.8);  // Vert repousse Jaune
                self.set_force(2, 0, -0.6);  // Bleu repousse Rouge
                self.set_force(3, 1, -1.2);  // Jaune repousse fort Vert

                // Auto-répulsion
                for i in 0..4 {
                    self.set_force(i, i, -0.4);
                }

                // Forces de nourriture équilibrées
                self.food_forces = vec![0.6, -0.4, 0.8, -0.2];
            },
            _ => {
                // Configuration aléatoire pour autres nombres de types
                let mut rng = rand::rng();
                for i in 0..self.type_count {
                    for j in 0..self.type_count {
                        let force = if i == j {
                            rng.random_range(-0.5..=-0.1)
                        } else {
                            rng.random_range(-1.0..=1.0)
                        };
                        self.set_force(i, j, force);
                    }
                    self.food_forces[i] = rng.random_range(-1.0..=1.0);
                }
            }
        }
    }
}