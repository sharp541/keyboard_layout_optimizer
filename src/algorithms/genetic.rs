use rand::prelude::*;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

use crate::keyboard_layout::{LogicalLayout, PhysicalLayout};
use crate::n_gram::{LogicalNGram, NGramDB};

pub struct Genetic {
    population_size: usize,
}

impl Genetic {
    pub fn new(population_size: usize) -> Self {
        Self { population_size }
    }

    pub fn optimize(
        &self,
        physical_layout: &PhysicalLayout,
        usable_chars: &[char],
        n_gram_db: &NGramDB,
        iterations: usize,
    ) {
        let tri_grams = n_gram_db.get_tri_grams().expect("Failed to get 3-grams");
        let mut rng = thread_rng();
        let mut layout = vec![None; physical_layout.len()];
        for c in usable_chars {
            layout.push(Some(*c));
        }
        let mut population = Vec::with_capacity(self.population_size);
        for _ in 0..self.population_size {
            layout.shuffle(&mut rng);
            let copy = LogicalLayout::from_layout(physical_layout, layout.clone());
            let individual = Individual::new(copy);
            population.push(individual);
        }

        for _ in 0..iterations {
            let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);

            // Evaluate population
            population.iter_mut().for_each(|i| {
                i.evaluate(&tri_grams);
            });

            // Sort population by score
            population.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .expect("Failed to compare scores")
            });

            // Keep elite individuals
            let elite_num = if self.population_size % 2 == 0 { 2 } else { 1 };
            new_population.extend(population.iter().take(elite_num).cloned());

            while new_population.len() < self.population_size {
                let weights: Vec<f32> = population.iter().map(|ind| 1.0 / ind.score).collect();
                let [parent1, parent2]: [&Individual; 2] = (0..population.len())
                    .collect::<Vec<_>>()
                    .choose_multiple_weighted(&mut rng, 2, |i| weights[*i])
                    .unwrap()
                    .map(|i| &population[*i])
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                let (mut child1, mut child2) = parent1.cycle_crossover(parent2, &mut rng);
                child1.reverse_mutation(&mut rng);
                child2.shift_mutation(&mut rng);
                new_population.push(child1);
                new_population.push(child2);
            }

            // Generate new individuals
            population = new_population;
        }
    }
}

#[derive(Debug, Clone)]
struct Individual<'a> {
    layout: LogicalLayout<'a>,
    score: f32,
}

impl<'a> Individual<'a> {
    pub fn new(layout: LogicalLayout<'a>) -> Self {
        Self { layout, score: 0.0 }
    }

    pub fn evaluate(&mut self, tri_grams: &HashMap<LogicalNGram<3>, f32>) {
        self.score = self.layout.evaluate(tri_grams);
    }

    pub fn cycle_crossover(&self, other: &Self, rng: &mut ThreadRng) -> (Self, Self) {
        let mut child1 = self.layout.clone();
        let mut child2 = other.layout.clone();

        let mut cycle: HashSet<usize> = HashSet::new();
        let mut start = rng.gen_range(0..child1.len());
        while !cycle.contains(&start) {
            cycle.insert(start);
            start = self.layout.get_char_index(other.layout.get(start)).clone();
        }
        cycle.iter().for_each(|i| {
            child1.set(*i, other.layout.get(*i));
            child2.set(*i, self.layout.get(*i));
        });

        (Individual::new(child1), Individual::new(child2))
    }

    pub fn reverse_mutation(&mut self, rng: &mut ThreadRng) {
        let a = rng.gen_range(0..self.layout.len());
        let b = rng.gen_range(0..self.layout.len());
        let (start, end) = if a < b { (a, b) } else { (b, a) };
        let median = (start + end + 1) / 2;
        let left = start..median;
        let right = (median + 1)..=end;
        left.zip(right.rev()).for_each(|(i, j)| {
            self.layout.swap(i, j);
        });
    }

    pub fn shift_mutation(&mut self, rng: &mut ThreadRng) {
        let a = rng.gen_range(0..self.layout.len());
        let b = rng.gen_range(0..self.layout.len());
        let (start, end) = if a < b { (a, b) } else { (b, a) };
        for i in start..end {
            self.layout.swap(i, i + 1);
        }
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }
}

impl<'a> PartialEq for Individual<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}
