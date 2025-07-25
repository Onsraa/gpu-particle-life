use bevy::prelude::*;
use rand::Rng;

/// Génome simplifié avec forces vectorisées et validation de cohérence
#[derive(Component, Clone, Debug, Default)]
pub struct Genotype {
    pub force_matrix: Vec<f32>,  // Matrice des forces particule-particule
    pub food_forces: Vec<f32>,   // Forces de nourriture par type
    pub type_count: usize,
    pub fitness_history: Vec<f32>, // Historique des performances pour validation
    pub strategy_coherence: f32,   // Score de cohérence de la stratégie [0.0-1.0]
}

#[derive(Clone, Copy, Debug)]
pub enum CrossoverStrategy {
    Uniform,              // Crossover original (aléatoire)
    SymmetricRelations,   // Préserve les relations symétriques
    TypeBlocks,           // Crossover par type complet
    AdaptiveHybrid,       // Combine les stratégies selon la situation
}

impl Genotype {
    pub fn new(type_count: usize) -> Self {
        let matrix_size = type_count * type_count;
        Self {
            force_matrix: vec![0.0; matrix_size],
            food_forces: vec![0.0; type_count],
            type_count,
            fitness_history: Vec::new(),
            strategy_coherence: 1.0,
        }
    }

    /// Génère un génome aléatoire avec validation de cohérence
    pub fn random(type_count: usize) -> Self {
        let mut rng = rand::rng();
        let mut genotype = Self::new(type_count);

        // Générer plusieurs candidats et choisir le plus cohérent
        let mut best_genotype = genotype.clone();
        let mut best_coherence = 0.0;

        for _ in 0..5 { // Tester 5 candidats
            genotype = Self::generate_candidate(type_count, &mut rng);
            let coherence = genotype.calculate_strategy_coherence();

            if coherence > best_coherence {
                best_coherence = coherence;
                best_genotype = genotype.clone();
            }
        }

        best_genotype.strategy_coherence = best_coherence;
        best_genotype
    }

    fn generate_candidate(type_count: usize, rng: &mut impl Rng) -> Self {
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
            fitness_history: Vec::new(),
            strategy_coherence: 1.0,
        }
    }

    /// Calcule un score de cohérence stratégique
    pub fn calculate_strategy_coherence(&self) -> f32 {
        let mut coherence_score = 0.0;
        let mut total_checks = 0;

        // 1. Vérifier la symétrie logique des relations
        for i in 0..self.type_count {
            for j in 0..self.type_count {
                if i != j {
                    let force_ij = self.get_force(i, j);
                    let force_ji = self.get_force(j, i);

                    // Relations réciproques : si A attire B fortement, B devrait avoir une réaction
                    let reciprocity = 1.0 - (force_ij + force_ji).abs() / 4.0; // Normaliser sur [-2, +2]
                    coherence_score += reciprocity.max(0.0);
                    total_checks += 1;
                }
            }
        }

        // 2. Vérifier l'équilibre des forces de nourriture
        let food_balance = {
            let positive_food = self.food_forces.iter().filter(|&&f| f > 0.0).count();
            let negative_food = self.food_forces.iter().filter(|&&f| f < 0.0).count();

            // Idéalement, un mélange équilibré
            let balance_ratio = (positive_food.min(negative_food) as f32) / (self.type_count as f32);
            balance_ratio
        };

        // 3. Vérifier l'absence d'oscillations potentielles
        let stability_score = self.check_oscillation_risk();

        // Score final pondéré
        let final_score = if total_checks > 0 {
            (coherence_score / total_checks as f32) * 0.5 + food_balance * 0.3 + stability_score * 0.2
        } else {
            0.5
        };

        final_score.clamp(0.0, 1.0)
    }

    fn check_oscillation_risk(&self) -> f32 {
        // Détecter les cycles de forces qui pourraient créer des oscillations
        let mut cycles_detected = 0;
        let mut total_cycles_checked = 0;

        for i in 0..self.type_count {
            for j in 0..self.type_count {
                if i != j {
                    for k in 0..self.type_count {
                        if k != i && k != j {
                            // Vérifier le cycle i -> j -> k -> i
                            let force_ij = self.get_force(i, j);
                            let force_jk = self.get_force(j, k);
                            let force_ki = self.get_force(k, i);

                            // Un cycle stable a des forces cohérentes
                            let cycle_product = force_ij * force_jk * force_ki;
                            if cycle_product.abs() > 0.1 && cycle_product > 0.0 {
                                cycles_detected += 1;
                            }
                            total_cycles_checked += 1;
                        }
                    }
                }
            }
        }

        if total_cycles_checked > 0 {
            1.0 - (cycles_detected as f32) / (total_cycles_checked as f32)
        } else {
            1.0
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

    /// Crossover amélioré avec plusieurs stratégies
    pub fn crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        // Choisir la stratégie de crossover selon la cohérence des parents
        let strategy = if self.strategy_coherence > 0.7 && other.strategy_coherence > 0.7 {
            CrossoverStrategy::SymmetricRelations
        } else if self.strategy_coherence > 0.5 || other.strategy_coherence > 0.5 {
            CrossoverStrategy::TypeBlocks
        } else {
            CrossoverStrategy::AdaptiveHybrid
        };

        match strategy {
            CrossoverStrategy::SymmetricRelations => self.symmetric_crossover(other, rng),
            CrossoverStrategy::TypeBlocks => self.type_block_crossover(other, rng),
            CrossoverStrategy::AdaptiveHybrid => self.adaptive_crossover(other, rng),
            CrossoverStrategy::Uniform => self.uniform_crossover(other, rng),
        }
    }

    /// Crossover par relations symétriques (préserve les interactions cohérentes)
    fn symmetric_crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        let mut new_genotype = Genotype::new(self.type_count);

        // Pour chaque paire de types, choisir la relation complète depuis un parent
        for i in 0..self.type_count {
            for j in i+1..self.type_count { // Traiter seulement la moitié supérieure
                let use_parent1 = rng.random_bool(0.5);

                if use_parent1 {
                    // Copier les deux directions depuis parent1
                    new_genotype.set_force(i, j, self.get_force(i, j));
                    new_genotype.set_force(j, i, self.get_force(j, i));
                } else {
                    // Copier les deux directions depuis parent2
                    new_genotype.set_force(i, j, other.get_force(i, j));
                    new_genotype.set_force(j, i, other.get_force(j, i));
                }
            }
        }

        // Auto-interactions (diagonale)
        for i in 0..self.type_count {
            if rng.random_bool(0.5) {
                new_genotype.set_force(i, i, self.get_force(i, i));
            } else {
                new_genotype.set_force(i, i, other.get_force(i, i));
            }
        }

        // Crossover des forces de nourriture par bloc
        if rng.random_bool(0.5) {
            new_genotype.food_forces = self.food_forces.clone();
        } else {
            new_genotype.food_forces = other.food_forces.clone();
        }

        new_genotype.strategy_coherence = new_genotype.calculate_strategy_coherence();
        new_genotype
    }

    /// Crossover par blocs de types complets
    fn type_block_crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        let mut new_genotype = Genotype::new(self.type_count);

        // Pour chaque type, choisir toutes ses interactions depuis le même parent
        for type_i in 0..self.type_count {
            let use_parent1 = rng.random_bool(0.5);

            for type_j in 0..self.type_count {
                if use_parent1 {
                    new_genotype.set_force(type_i, type_j, self.get_force(type_i, type_j));
                } else {
                    new_genotype.set_force(type_i, type_j, other.get_force(type_i, type_j));
                }
            }

            // Force de nourriture pour ce type
            if use_parent1 {
                new_genotype.food_forces[type_i] = self.food_forces[type_i];
            } else {
                new_genotype.food_forces[type_i] = other.food_forces[type_i];
            }
        }

        new_genotype.strategy_coherence = new_genotype.calculate_strategy_coherence();
        new_genotype
    }

    /// Crossover adaptatif qui combine les stratégies
    fn adaptive_crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        // Utiliser principalement le parent le plus cohérent, avec quelques éléments de l'autre
        let (primary, secondary) = if self.strategy_coherence > other.strategy_coherence {
            (self, other)
        } else {
            (other, self)
        };

        let mut new_genotype = primary.clone();

        // Incorporer 20% d'éléments du parent secondaire
        let incorporation_rate = 0.2;

        for i in 0..self.force_matrix.len() {
            if rng.random::<f32>() < incorporation_rate {
                new_genotype.force_matrix[i] = secondary.force_matrix[i];
            }
        }

        for i in 0..self.food_forces.len() {
            if rng.random::<f32>() < incorporation_rate * 0.5 { // Moins fréquent pour la nourriture
                new_genotype.food_forces[i] = secondary.food_forces[i];
            }
        }

        new_genotype.strategy_coherence = new_genotype.calculate_strategy_coherence();
        new_genotype
    }

    /// Crossover uniforme original (pour comparaison)
    fn uniform_crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        let mut new_genotype = Genotype::new(self.type_count);

        for i in 0..self.force_matrix.len() {
            if rng.random_bool(0.5) {
                new_genotype.force_matrix[i] = self.force_matrix[i];
            } else {
                new_genotype.force_matrix[i] = other.force_matrix[i];
            }
        }

        for i in 0..self.food_forces.len() {
            if rng.random_bool(0.5) {
                new_genotype.food_forces[i] = self.food_forces[i];
            } else {
                new_genotype.food_forces[i] = other.food_forces[i];
            }
        }

        new_genotype.strategy_coherence = new_genotype.calculate_strategy_coherence();
        new_genotype
    }

    /// Mutation améliorée et plus douce
    pub fn mutate(&mut self, mutation_rate: f32, rng: &mut impl Rng) {
        let coherence_factor = self.strategy_coherence;

        // Taux de mutation adaptatif : moins de mutation si la stratégie est cohérente
        let effective_rate = mutation_rate * (1.5 - coherence_factor);

        // Amplitude de mutation adaptative
        let mutation_amplitude = if coherence_factor > 0.7 {
            0.1 // Mutations douces pour préserver les bonnes stratégies
        } else {
            0.2 // Mutations plus importantes pour améliorer les stratégies incohérentes
        };

        // Mutation de la matrice des forces avec préservation des relations importantes
        for i in 0..self.force_matrix.len() {
            if rng.random::<f32>() < effective_rate {
                let old_value = self.force_matrix[i];
                let mutation = rng.random_range(-mutation_amplitude..=mutation_amplitude);

                // Mutation plus conservative pour les forces importantes
                let conservative_factor = if old_value.abs() > 0.5 { 0.5 } else { 1.0 };

                self.force_matrix[i] = (old_value + mutation * conservative_factor).clamp(-2.0, 2.0);
            }
        }

        // Mutation des forces de nourriture (moins fréquente)
        for force in &mut self.food_forces {
            if rng.random::<f32>() < effective_rate * 0.3 {
                let mutation = rng.random_range(-mutation_amplitude..=mutation_amplitude);
                *force = (*force + mutation).clamp(-2.0, 2.0);
            }
        }

        // Recalculer la cohérence après mutation
        self.strategy_coherence = self.calculate_strategy_coherence();
    }

    /// Met à jour l'historique de fitness
    pub fn update_fitness_history(&mut self, fitness: f32) {
        self.fitness_history.push(fitness);
        // Garder seulement les 10 dernières valeurs
        if self.fitness_history.len() > 10 {
            self.fitness_history.remove(0);
        }
    }

    /// Calcule la tendance de fitness (amélioration/dégradation)
    pub fn get_fitness_trend(&self) -> f32 {
        if self.fitness_history.len() < 3 {
            return 0.0;
        }

        let recent_avg = self.fitness_history.iter().rev().take(3).sum::<f32>() / 3.0;
        let older_avg = self.fitness_history.iter().take(3).sum::<f32>() / 3.0;

        recent_avg - older_avg
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

    /// Génère des forces intéressantes prédéfinies avec validation
    pub fn set_interesting_forces(&mut self) {
        self.force_matrix.fill(0.0);
        self.food_forces.fill(0.0);

        match self.type_count {
            3 => {
                // Configuration rock-paper-scissors améliorée
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

                // Forces de nourriture équilibrées
                self.food_forces = vec![0.8, -0.3, 0.5];
            },
            4 => {
                // Configuration plus complexe avec validation
                self.set_force(0, 1, 1.5);   // Rouge attire fort Vert
                self.set_force(1, 2, 0.8);   // Vert attire Bleu
                self.set_force(2, 3, 1.2);   // Bleu attire fort Jaune
                self.set_force(3, 0, 0.6);   // Jaune attire Rouge

                // Répulsions croisées équilibrées
                self.set_force(0, 2, -1.0);
                self.set_force(1, 3, -0.8);
                self.set_force(2, 0, -0.6);
                self.set_force(3, 1, -1.2);

                // Auto-répulsion
                for i in 0..4 {
                    self.set_force(i, i, -0.4);
                }

                self.food_forces = vec![0.6, -0.4, 0.8, -0.2];
            },
            _ => {
                // Configuration aléatoire validée
                let mut rng = rand::rng();
                for _ in 0..5 { // Essayer 5 configurations
                    let candidate = Self::generate_candidate(self.type_count, &mut rng);
                    if candidate.calculate_strategy_coherence() > 0.5 {
                        self.force_matrix = candidate.force_matrix;
                        self.food_forces = candidate.food_forces;
                        break;
                    }
                }
            }
        }

        self.strategy_coherence = self.calculate_strategy_coherence();
    }
}