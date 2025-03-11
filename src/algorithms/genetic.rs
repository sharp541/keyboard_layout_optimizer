use fastrand;
use plotters::prelude::*;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;

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
        shuffle: bool,
    ) {
        let initial_layout =
            LogicalLayout::from_usable_chars(physical_layout, usable_chars.to_vec());
        let mut layout = initial_layout.clone().output();
        let mut population = Vec::with_capacity(self.population_size);
        for _ in 0..self.population_size {
            if shuffle {
                fastrand::shuffle(&mut layout);
            }
            let copy = LogicalLayout::from_usable_chars(physical_layout, layout.clone());
            let individual = Individual::new(copy);
            population.push(individual);
        }

        let mut best_layout = Individual::new(initial_layout);
        best_layout.evaluate(physical_layout, tri_grams);
        let elite_num = if self.population_size % 2 == 0 { 2 } else { 1 };

        let mut best_scores: Vec<f32> = Vec::with_capacity(iterations);

        for i in 0..iterations {
            let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);

            population.par_iter_mut().for_each(|i| {
                i.evaluate(physical_layout, tri_grams);
            });

            // Sort population by score
            population.sort_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .expect("Failed to compare scores")
            });

            // Keep elite individuals
            new_population.extend(population.iter().take(elite_num).cloned());

            let sum = population.iter().map(|ind| ind.score).sum::<f32>();
            let weights: Vec<f32> = population.iter().map(|ind| ind.score / sum).collect();
            let dist = WeightedIndex::new(weights).unwrap();
            let mut rng = thread_rng();
            let mut children: Vec<Individual> = (0..self.population_size - elite_num)
                .into_iter()
                .map(|_| population[dist.sample(&mut rng)].clone())
                .collect();

            children.par_iter_mut().for_each(|i| {
                i.mutate(&mut fastrand::Rng::new());
            });
            new_population.append(&mut children);

            population = new_population;
            if population[0].score < best_layout.score {
                best_layout = population[0].clone();
            }
            if i % (iterations / 10) == 0 {
                println!("iteration: {} / {}", i, iterations);
                println!("best score: {}", best_layout.score);
            }
            best_scores.push(best_layout.score);
        }

        println!("best score: {}", best_layout.score);
        physical_layout.print(&best_layout.layout());

        // Plot the best scores
        let file_name = format!(
            "graphs/best_scores_{}_{}.png",
            self.population_size, iterations
        );
        let max_score = *best_scores
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let min_score = *best_scores
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let root = BitMapBackend::new(&file_name, (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("Best Scores Over Iterations", ("sans-serif", 50))
            .margin(10)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0..iterations, min_score..max_score)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                best_scores.iter().enumerate().map(|(x, y)| (x, *y)),
                &RED,
            ))
            .unwrap()
            .label("Best Score")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .draw()
            .unwrap();
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

    fn reverse_mutation(&mut self, rng: &mut fastrand::Rng) {
        let a = rng.usize(0..self.layout.len());
        let b = rng.usize(0..self.layout.len());
        let (start, end) = if a < b { (a, b) } else { (b, a) };
        let median = (start + end + 1) / 2;
        let left = start..median;
        let right = median..=end;
        left.zip(right.rev()).for_each(|(i, j)| {
            self.layout.swap(i, j);
        });
    }

    fn shift_mutation(&mut self, rng: &mut fastrand::Rng) {
        let a = rng.usize(0..self.layout.len());
        let b = rng.usize(0..self.layout.len());
        let (start, end) = if a < b { (a, b) } else { (b, a) };
        for i in start..end {
            self.layout.swap(i, i + 1);
        }
    }

    fn random_mutation(&mut self, rng: &mut fastrand::Rng) {
        let mutation_num = rng.usize(0..3);
        for _ in 0..mutation_num {
            let a = rng.usize(0..self.layout.len());
            let b = rng.usize(0..self.layout.len());
            self.layout.swap(a, b);
        }
    }

    fn mutate(&mut self, rng: &mut fastrand::Rng) {
        let mutation_type = rng.u8(0..6);
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

impl<'a> PartialEq for Individual {
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
        let mut rng = fastrand::Rng::new();

        let original_layout = individual.layout();
        let mut test_individual = individual.clone();
        test_individual.reverse_mutation(&mut rng);

        assert_ne!(test_individual.layout(), original_layout);
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
        let mut rng = fastrand::Rng::new();

        let original_layout = individual.layout();
        let mut test_individual = individual.clone();
        test_individual.shift_mutation(&mut rng);

        assert_ne!(test_individual.layout(), original_layout);
    }
}
