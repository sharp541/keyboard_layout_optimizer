use fastrand;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand::thread_rng;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::keyboard_layout::{LogicalLayout, PhysicalLayout};
use crate::n_gram::{LogicalNGram, NGramDB};

pub struct Genetic {
    population_size: usize,
    island_size: usize,
}

impl Genetic {
    pub fn new(population_size: usize, island_size: usize) -> Self {
        if population_size < 3 {
            panic!("population_size must be greater than 2");
        }
        Self {
            population_size,
            island_size,
        }
    }

    pub fn optimize(
        &self,
        physical_layout: &PhysicalLayout,
        usable_chars: &[char],
        ngram_db: &NGramDB,
        iterations: usize,
        shuffle: bool,
        early_stop_count: usize,
    ) {
        let initial_layout =
            LogicalLayout::from_usable_chars(&usable_chars.to_vec());
        let mut best_layout = Individual::new(initial_layout.clone());
        let usable_chars_set: HashSet<char> = usable_chars.iter().cloned().collect();
        let tri_grams = ngram_db
            .get_tri_grams(&usable_chars_set)
            .expect("Failed to get tri grams");
        best_layout.evaluate(physical_layout, &tri_grams);

        let mut rng_fast = fastrand::Rng::new();
        // initialize
        let mut islands = Vec::with_capacity(self.island_size);
        for _ in 0..self.island_size {
            let mut population = Vec::with_capacity(self.population_size);
            for _ in 0..self.population_size {
                let mut individual = Individual::new(initial_layout.clone());
                if shuffle {
                    individual.mutate(&mut rng_fast);
                }
                individual.evaluate(physical_layout, &tri_grams);
                population.push(individual);
            }
            islands.push(population);
        }

        let elite_num = if self.population_size % 2 == 0 { 2 } else { 1 };
        let mut count = 0;
        for i in 0..iterations {
            islands.chunks_mut(1).for_each(|chunk| {
                let population = &mut chunk[0];

                let sum = population.iter().map(|ind| ind.score).sum::<f32>();
                let weights: Vec<f32> = population.iter().map(|ind| ind.score / sum).collect();
                let dist = WeightedIndex::new(weights).unwrap();

                // Keep elite individuals
                let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);
                new_population.extend(population.iter().take(elite_num).cloned());

                // Crossover
                let mut children: Vec<Individual> = (0..self.population_size - elite_num)
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
                        let mut child = parent1.cyclic_crossover(parent2, &mut rng);
                        child.layout.print();
                        child.mutate(&mut rng_fast);
                        child
                    })
                    .collect();
                new_population.append(&mut children);

                *population = new_population;

                // Evaluate population
                population.iter_mut().for_each(|i| {
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
            if i % 10 == 0 {
                for idx in 0..islands.len() {
                    let best_individual = islands[idx][0].clone();
                    let next_idx = (idx + 1) % islands.len();
                    let next_population = &mut islands[next_idx];
                    next_population[self.population_size - 1] = best_individual;
                }
            }

            // update best layout
            let current_best_layout = islands.iter().min_by(|a, b| {
                a[0].score
                    .partial_cmp(&b[0].score)
                    .expect("Failed to compare scores")
            });
            if let Some(current_best_layout) = current_best_layout {
                if current_best_layout[0].score < best_layout.score {
                    best_layout = current_best_layout[0].clone();
                    count = 0;
                }
            }

            if i % 100 == 0 {
                println!("iteration: {} / {}", i, iterations);
                println!("best score: {}", best_layout.score);
            }

            count += 1;
            if count > early_stop_count {
                println!("No improvement for {} iterations, stopping...", early_stop_count);
                break;
            }
        }

        println!("best score: {}", best_layout.score);
        best_layout.layout.print();
    }
}

#[derive(Debug, Clone)]
struct Individual {
    layout: LogicalLayout,
    score: f32,
}

impl Individual {
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

    fn cyclic_crossover(
        &self,
        other: &Self,
        rng: &mut ThreadRng,
    ) -> Self {
        let mut new_layout = self.layout.clone();

        let n = self.layout.len();
        let mut visited = vec![false; n];
        let mut remaining = n;

        while remaining > 0 {
            let idx = loop {
                let i = rng.gen_range(0..n);
                if !visited[i] {
                    break i;
                }
            };
            visited[idx] = true;
            remaining -= 1;
            let start_char = self.layout.get(idx);
            let mut self_char = start_char;
            loop {
                let self_char_index = self.layout.get_char_index(self_char);
                let other_char = other.layout.get(self_char_index);
                if start_char == other_char {
                    break;
                }
                visited[idx] = true;
                remaining -= 1;
                self_char = other_char;
            }
            if remaining == 0 {
                break;
            }

            let idx = loop {
                let i = rng.gen_range(0..n);
                if !visited[i] {
                    break i;
                }
            };
            visited[idx] = true;
            remaining -= 1;
            let mut other_char = start_char;
            loop {
                let other_char_index = other.layout.get_char_index(other_char);
                let self_char = self.layout.get(other_char_index);
                new_layout.set(other_char_index, other_char);
                if start_char == self_char {
                    break;
                }
                visited[idx] = true;
                remaining -= 1;
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
}

impl PartialEq for Individual {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}
