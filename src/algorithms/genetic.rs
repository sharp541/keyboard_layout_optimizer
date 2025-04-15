use fastrand;
use plotters::prelude::*;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand::seq::IteratorRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::keyboard_layout::{LogicalLayout, PhysicalLayout};
use crate::n_gram::{LogicalNGram, NGramDB};

pub struct Genetic {
    population_size: usize,
    island_size: usize
}

impl Genetic {
    pub fn new(population_size: usize, island_size: usize) -> Self {
        if population_size < 3 {
            panic!("population_size must be greater than 2");
        }
        Self { population_size, island_size }
    }

    pub fn optimize(
        &self,
        physical_layout: &PhysicalLayout,
        usable_chars: &[char],
        ngram_db: &NGramDB,
        iterations: usize,
        shuffle: bool,
    ) {
        let initial_layout =
            LogicalLayout::from_usable_chars(physical_layout, usable_chars.to_vec());
        let mut layout = initial_layout.clone().output();
        let mut best_layout = Individual::new(initial_layout);
        let usable_chars: HashSet<char> = usable_chars.iter().cloned().collect();
        let tri_grams = ngram_db
            .get_tri_grams(&usable_chars)
            .expect("Failed to get tri grams");
        best_layout.evaluate(physical_layout, &tri_grams);

        // initialize
        let mut islands = Vec::with_capacity(self.island_size);
        for _ in 0..self.island_size {
            let mut population = Vec::with_capacity(self.population_size);
            for _ in 0..self.population_size {
                if shuffle {
                    fastrand::shuffle(&mut layout);
                }
                let copy = LogicalLayout::from_usable_chars(physical_layout, layout.clone());
                let mut individual = Individual::new(copy);
                individual.evaluate(physical_layout, &tri_grams);
                population.push(individual);
            }
            islands.push(population);
        }

        let elite_num = if self.population_size % 2 == 0 { 2 } else { 1 };
        for i in 0..iterations {
            islands.par_chunks_mut(1).for_each(|chunk| {
                let population = &mut chunk[0];

                let sum = population.iter().map(|ind| ind.score).sum::<f32>();
                let weights: Vec<f32> = population.iter().map(|ind| ind.score / sum).collect();
                let dist = WeightedIndex::new(weights).unwrap();

                // Keep elite individuals
                let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);
                new_population.extend(population.iter().take(elite_num).cloned());

                // Crossover
                let mut children: Vec<Individual> = (0..self.population_size - elite_num)
                    .into_par_iter()
                    .map(|_| {
                        let mut rng = thread_rng();
                        let mut rng_fast = fastrand::Rng::new();
                        let parent1_index = dist.sample(&mut rng);
                        let parent2_index = dist.sample(&mut rng);
                        if parent1_index == parent2_index {
                            return population[parent1_index].clone();
                        }
                        let parent1 = &population[parent1_index];
                        let parent2 = &population[parent2_index];
                        let mut child = parent1.cyclic_crossover(parent2, &usable_chars, &mut rng);
                        child.mutate(&mut rng_fast);
                        child
                    })
                    .collect();
                new_population.append(&mut children);

                *population = new_population;

                // Evaluate population
                population.par_iter_mut().for_each(|i| {
                    i.evaluate(physical_layout, &tri_grams);
                });

                // Sort population by score
                population.sort_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .expect("Failed to compare scores")
                });
            });

            // migrate best individuals
            for idx in 0..islands.len() {
                let best_individual = islands[idx][0].clone();
                let next_idx = (idx + 1) % islands.len();
                let next_population = &mut islands[next_idx];
                next_population[-1] = best_individual;
            };

            // update best layout
            let current_best_layout = islands.iter().min_by(|a, b| {
                a[0].score
                    .partial_cmp(&b[0].score)
                    .expect("Failed to compare scores")
            });
            if let Some(current_best_layout) = current_best_layout {
                if current_best_layout[0].score < best_layout.score {
                    best_layout = current_best_layout[0].clone();
                }
            }

            if i % (iterations / 10) == 0 {
                println!("iteration: {} / {}", i, iterations);
                println!("best score: {}", best_layout.score);
            }

        }

        println!("best score: {}", best_layout.score);
        physical_layout.print(&best_layout.layout());
    }
}

#[derive(Debug, Clone)]
struct Individual {
    layout: LogicalLayout,
    score: f32,
}

impl<'a> Individual {
    fn new(layout: LogicalLayout) -> Self {
        Self { layout, score: 0.0 }
    }

    fn evaluate(
        &mut self,
        physical_layout: &PhysicalLayout,
        tri_grams: &HashMap<LogicalNGram<3>, f32>,
    ) {
        self.score = self.layout.evaluate(physical_layout, tri_grams);
    }

    fn cyclic_crossover(&self, other: &Self, usable_chars: &HashSet<char>, rng: &mut ThreadRng) -> Self {
        let mut new_layout = self.layout.clone();

        let mut usable_chars = usable_chars.clone();
        while !usable_chars.is_empty() {
            let start_char = rand_pop(&mut usable_chars, rng);
            let mut self_char = start_char;
            loop {
                let self_char_index = self.layout.get_char_index(self_char);
                let other_char = other.layout.get(self_char_index);
                if start_char == other_char {
                    break;
                }
                usable_chars.remove(&other_char);
                self_char = other_char;
            }
            if usable_chars.is_empty() {
                break;
            }

            let start_char = rand_pop(&mut usable_chars, rng);
            let mut other_char = start_char;
            loop {
                let other_char_index = other.layout.get_char_index(other_char);
                let self_char = self.layout.get(other_char_index);
                new_layout.set(other_char_index, other_char);
                if start_char == self_char {
                    break;
                }
                usable_chars.remove(&self_char);
                other_char = self_char;
            }
        }

        Self::new(new_layout)
    }

    fn random_mutation(&mut self, rng: &mut fastrand::Rng) {
        let a = rng.usize(0..self.layout.len());
        let b = rng.usize(0..self.layout.len());
        self.layout.swap(a, b);
    }

    fn mutate(&mut self, rng: &mut fastrand::Rng) {
        let mutation_type = rng.u8(0..4);
        match mutation_type {
            0 => (),
            _ => self.random_mutation(rng),
        }
    }

    fn layout(&self) -> Vec<char> {
        self.layout.clone().output()
    }
}

impl<'a> PartialEq for Individual {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

fn rand_pop(set: &mut HashSet<char>, rng: &mut ThreadRng) -> char {
    let char = *set.iter().choose(rng).expect("No usable chars found");
    set.remove(&char);
    char
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::keyboard_layout::PhysicalLayout;

//     #[test]
//     fn test_shift_mutation() {
//         let cost_table = [
//             [3.7, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.7], // 上段
//             [3.0, 1.3, 1.1, 1.0, 1.6, 1.6, 1.0, 1.1, 1.3, 3.0], // 中段（ホームポジション）
//             [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
//         ];
//         let physical = PhysicalLayout::new(cost_table).expect("Invalid cost table");
//         let usable_chars = vec![
//             'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
//             'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ',', '.',
//         ];
//         let logical = LogicalLayout::from_usable_chars(&physical, usable_chars);
//         let individual = Individual::new(logical);
//         let mut rng = fastrand::Rng::new();

//         let original_layout = individual.layout();
//         let mut test_individual = individual.clone();

//         assert_ne!(test_individual.layout(), original_layout);
//     }
// }
