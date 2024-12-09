use rand::prelude::*;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::iter;

use crate::keyboard_layout::{LogicalLayout, PhysicalLayout};
use crate::n_gram::LogicalNGram;

pub struct Genetic {
    population_size: usize,
}

impl Genetic {
    pub fn new(population_size: usize) -> Self {
        if population_size < 3 {
            panic!("population_size must be greater than 2");
        }
        Self { population_size }
    }

    pub fn optimize(
        &self,
        physical_layout: &PhysicalLayout,
        usable_chars: &[char],
        tri_grams: &HashMap<LogicalNGram<3>, f32>,
        iterations: usize,
    ) {
        let mut rng = thread_rng();
        let initial_layout =
            LogicalLayout::from_usable_chars(physical_layout, usable_chars.to_vec());
        let mut layout = initial_layout.clone().output();
        let mut population = Vec::with_capacity(self.population_size);
        for _ in 0..self.population_size {
            layout.shuffle(&mut rng);
            let copy = LogicalLayout::from_usable_chars(physical_layout, layout.clone());
            let individual = Individual::new(copy);
            population.push(individual);
        }

        let mut best_layout = Individual::new(initial_layout);
        best_layout.evaluate(tri_grams);
        for i in 0..iterations {
            println!("iteration: {}", i);
            let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);

            population.par_iter_mut().for_each(|i| {
                i.evaluate(&tri_grams);
            });

            // Sort population by score
            population.sort_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .expect("Failed to compare scores")
            });
            println!("best score: {}", best_layout.score);

            // Keep elite individuals
            let elite_num = self.population_size / 8;
            new_population.extend(population.iter().take(elite_num).cloned());

            let weights: Vec<f32> = population.iter().map(|ind| 1.0 / ind.score).collect();
            // Generate new individuals
            let mut children: Vec<Individual> = (0..self.population_size - elite_num)
                .into_par_iter()
                .map(|_| {
                    let mut rng = thread_rng();
                    let parents: Vec<_> = (0..population.len())
                        .collect::<Vec<_>>()
                        .choose_multiple_weighted(&mut rng, 2, |i| weights[*i])
                        .expect("Failed to choose parents")
                        .map(|i| &population[*i])
                        .collect();

                    let (child1, child2) = parents[0].cycle_crossover(parents[1], &mut rng);
                    vec![child1, child2]
                })
                .flatten()
                .collect();

            children.par_iter_mut().for_each(|i| {
                i.mutate(&mut thread_rng());
            });
            new_population.append(&mut children);

            // Generate new individuals
            population = new_population;
            if population[0].score < best_layout.score {
                best_layout = population[0].clone();
            }
        }

        println!("best score: {}", best_layout.score);
        physical_layout.print(&best_layout.layout());
    }
}

#[derive(Debug, Clone)]
struct Individual<'a> {
    layout: LogicalLayout<'a>,
    score: f32,
}

impl<'a> Individual<'a> {
    fn new(layout: LogicalLayout<'a>) -> Self {
        Self { layout, score: 0.0 }
    }

    fn evaluate(&mut self, tri_grams: &HashMap<LogicalNGram<3>, f32>) {
        self.score = self.layout.evaluate(tri_grams);
    }

    fn cycle_crossover(&self, other: &Self, rng: &mut ThreadRng) -> (Self, Self) {
        let mut child1 = self.layout.clone();
        let mut child2 = other.layout.clone();

        let mut cycle: HashSet<usize> = HashSet::new();
        while cycle.len() < 2 {
            cycle.clear();
            let mut start = rng.gen_range(0..child1.len());
            while cycle.insert(start) {
                start = self.layout.get_char_index(other.layout.get(start));
            }
        }

        cycle.iter().for_each(|i| {
            child1.set(*i, other.layout.get(*i).clone());
            child2.set(*i, self.layout.get(*i).clone());
        });

        (Individual::new(child1), Individual::new(child2))
    }

    fn reverse_mutation(&mut self, rng: &mut ThreadRng) {
        let a = rng.gen_range(0..self.layout.len());
        let b = rng.gen_range(0..self.layout.len());
        let (start, end) = if a < b { (a, b) } else { (b, a) };
        let median = (start + end + 1) / 2;
        let left = start..median;
        let right = median..=end;
        left.zip(right.rev()).for_each(|(i, j)| {
            self.layout.swap(i, j);
        });
    }

    fn shift_mutation(&mut self, rng: &mut ThreadRng) {
        let a = rng.gen_range(0..self.layout.len());
        let b = rng.gen_range(0..self.layout.len());
        let (start, end) = if a < b { (a, b) } else { (b, a) };
        for i in start..end {
            self.layout.swap(i, i + 1);
        }
    }

    fn random_mutation(&mut self, rng: &mut ThreadRng) {
        let mutation_num = rng.gen_range(0..self.layout.len() / 4);
        for _ in 0..mutation_num {
            let a = rng.gen_range(0..self.layout.len());
            let b = rng.gen_range(0..self.layout.len());
            self.layout.swap(a, b);
        }
    }

    fn mutate(&mut self, rng: &mut ThreadRng) {
        let mutation_type = rng.gen_range(0..5);
        match mutation_type {
            0 => (),
            1 => self.reverse_mutation(rng),
            2 => self.shift_mutation(rng),
            _ => self.random_mutation(rng),
        }
    }

    fn layout(&self) -> Vec<char> {
        self.layout.clone().output()
    }
}

impl<'a> PartialEq for Individual<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_layout::PhysicalLayout;

    #[test]
    fn test_reverse_mutation() {
        let cost_table = [
            [3.7, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.7], // 上段
            [3.0, 1.3, 1.1, 1.0, 1.6, 1.6, 1.0, 1.1, 1.3, 3.0], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
        ];
        let physical = PhysicalLayout::new(cost_table).expect("Invalid cost table");
        let usable_chars = vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ',', '.',
        ];
        let logical = LogicalLayout::from_usable_chars(&physical, usable_chars);
        let individual = Individual::new(logical);
        let mut rng = thread_rng();

        let original_layout = individual.layout();
        let mut test_individual = individual.clone();
        test_individual.reverse_mutation(&mut rng);

        assert_ne!(test_individual.layout(), original_layout);
    }

    #[test]
    fn test_cycle_crossover() {
        let cost_table = [
            [3.7, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.7], // 上段
            [3.0, 1.3, 1.1, 1.0, 1.6, 1.6, 1.0, 1.1, 1.3, 3.0], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
        ];
        let physical = PhysicalLayout::new(cost_table).expect("Invalid cost table");
        let usable_chars = vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ',', '.',
        ];
        let usable_chars2 = vec![
            'z', ',', '.', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n',
            'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
        ];
        let logical1 = LogicalLayout::from_usable_chars(&physical, usable_chars);
        let logical2 = LogicalLayout::from_usable_chars(&physical, usable_chars2);

        let parent1 = Individual::new(logical1);
        let parent2 = Individual::new(logical2);
        let mut rng = thread_rng();

        let (child1, _) = parent1.cycle_crossover(&parent2, &mut rng);

        assert_ne!(child1.layout(), parent1.layout());
    }

    #[test]
    fn test_shift_mutation() {
        let cost_table = [
            [3.7, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.7], // 上段
            [3.0, 1.3, 1.1, 1.0, 1.6, 1.6, 1.0, 1.1, 1.3, 3.0], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
        ];
        let physical = PhysicalLayout::new(cost_table).expect("Invalid cost table");
        let usable_chars = vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ',', '.',
        ];
        let logical = LogicalLayout::from_usable_chars(&physical, usable_chars);
        let individual = Individual::new(logical);
        let mut rng = thread_rng();

        let original_layout = individual.layout();
        let mut test_individual = individual.clone();
        test_individual.shift_mutation(&mut rng);

        assert_ne!(test_individual.layout(), original_layout);
    }
}
