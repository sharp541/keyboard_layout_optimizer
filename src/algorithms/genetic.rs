use rand::seq::IteratorRandom;
use rand::seq::SliceRandom;

use crate::keyboard_layout::{LogicalLayout, PhysicalLayout};
use crate::n_gram::NGramDB;

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
        let mut rng = rand::thread_rng();
        let mut layout = vec![None; physical_layout.len()];
        for c in usable_chars {
            layout.push(Some(*c));
        }
        let mut population = Vec::new();
        for _ in 0..self.population_size {
            layout.shuffle(&mut rng);
            population.push(LogicalLayout::from_layout(physical_layout, layout.clone()));
        }
        let tri_grams = n_gram_db.get_tri_grams().expect("Failed to get 3-grams");

        for _ in 0..iterations {
            let mut new_population: Vec<LogicalLayout> = Vec::new();
            let scores: Vec<f32> = population.iter().map(|l| l.evaluate(&tri_grams)).collect();

            let elite_idx = scores
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx)
                .unwrap();
            let elite = population[elite_idx].clone();

            new_population.push(elite);

            population = new_population;
        }
    }
}
