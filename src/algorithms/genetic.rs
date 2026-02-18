use fastrand;
use rand::prelude::*;
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
        ja_weight: f32,
        en_weight: f32,
    ) {
        let initial_layout = LogicalLayout::from_usable_chars(usable_chars);
        let mut best_layout = Individual::new(initial_layout.clone());
        let usable_chars_set: HashSet<char> = usable_chars.iter().cloned().collect();
        let tri_grams = ngram_db
            .get_tri_grams_weighted(&usable_chars_set, ja_weight, en_weight)
            .expect("Failed to get tri grams");
        // 使用文字に連番IDを付与（初期レイアウトと同一順）
        let char_to_id: HashMap<char, usize> = usable_chars
            .iter()
            .enumerate()
            .map(|(i, &c)| (c, i))
            .collect();
        // tri_gramsをID化してVecに前処理
        let tri_grams_ids: Vec<([usize; 3], f32)> = tri_grams
            .iter()
            .map(|(ng, s)| {
                let id0 = *char_to_id.get(&ng.get(0)).expect("char id missing");
                let id1 = *char_to_id.get(&ng.get(1)).expect("char id missing");
                let id2 = *char_to_id.get(&ng.get(2)).expect("char id missing");
                ([id0, id1, id2], *s)
            })
            .collect();
        best_layout.score = best_layout
            .layout
            .evaluate_ids(physical_layout, &tri_grams_ids);

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
            // 並列化：島ごとに独立処理
            islands.par_iter_mut().for_each(|population| {
                // スレッドローカルの軽量RNG
                let mut local_rng = fastrand::Rng::new();
                // エリート抽出：全体ソートを避け、最小スコアの上位elite_numのみ選ぶ
                let mut new_population: Vec<Individual> = Vec::with_capacity(self.population_size);
                for _ in 0..elite_num {
                    if let Some((best_idx, _)) =
                        population.iter().enumerate().min_by(|(_, a), (_, b)| {
                            a.score
                                .partial_cmp(&b.score)
                                .expect("Failed to compare scores")
                        })
                    {
                        new_population.push(population[best_idx].clone());
                        // 重複選出を避けるため、その個体のscoreを一時的に最大化
                        population[best_idx].score = f32::INFINITY;
                    }
                }

                // Crossover
                let mut children: Vec<Individual> =
                    Vec::with_capacity(self.population_size - elite_num);
                // トーナメント選択（最小化）：k=3
                let k = 3usize;
                for _ in 0..(self.population_size - elite_num) {
                    // 親1
                    let p1 = tournament_index(population, k, &mut local_rng);
                    // 親2（同一回避を緩く試みる）
                    let mut p2 = tournament_index(population, k, &mut local_rng);
                    if p1 == p2 {
                        p2 = tournament_index(population, k, &mut local_rng);
                    }
                    let parent1 = &population[p1];
                    // randのThreadRngは一つを共有
                    let mut rng = rand::thread_rng();
                    let parent2 = &population[p2];
                    let mut child = parent1.cyclic_crossover(parent2, &mut rng);
                    child.mutate(&mut local_rng);
                    children.push(child);
                }
                new_population.append(&mut children);

                *population = new_population;

                // Evaluate population
                population.par_iter_mut().for_each(|i| {
                    i.score = i.layout.evaluate_ids(physical_layout, &tri_grams_ids);
                });

                // 完全ソートを避ける（次反復のエリート抽出は部分選択で行う）
            });

            // migrate best individuals
            if i % 10 == 0 {
                for idx in 0..islands.len() {
                    // 各島の最良個体（最小スコア）を取得
                    let best_individual = islands[idx]
                        .iter()
                        .min_by(|a, b| {
                            a.score
                                .partial_cmp(&b.score)
                                .expect("Failed to compare scores")
                        })
                        .cloned()
                        .expect("Island population should not be empty");
                    let next_idx = (idx + 1) % islands.len();
                    let next_population = &mut islands[next_idx];
                    // 次島の最悪個体（最大スコア）を置換
                    if let Some((worst_idx, _)) =
                        next_population.iter().enumerate().max_by(|(_, a), (_, b)| {
                            a.score
                                .partial_cmp(&b.score)
                                .expect("Failed to compare scores")
                        })
                    {
                        next_population[worst_idx] = best_individual;
                    }
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

            if i % 1000 == 0 {
                println!("iteration: {} / {}", i, iterations);
                println!("best score: {}", best_layout.score);
            }

            count += 1;
            if count > early_stop_count {
                println!(
                    "No improvement for {} iterations, stopping...",
                    early_stop_count
                );
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

    fn cyclic_crossover(&self, other: &Self, rng: &mut ThreadRng) -> Self {
        // Cyclic crossover for permutations:
        // Start from a random index, follow the cycle of positions defined
        // by mapping parent1's value into the index where that value appears in parent2.
        // Positions in the cycle take values from parent1; others remain from parent2.
        let n = self.layout.len();
        let mut child_layout = other.layout.clone();

        let start = rng.gen_range(0..n);
        let mut idx = start;
        let mut visited = vec![false; n];

        loop {
            if visited[idx] {
                break;
            }
            visited[idx] = true;

            let v = self.layout.get(idx);
            child_layout.set(idx, v);

            let next_idx = other.layout.get_char_index(v);
            if visited[next_idx] {
                break;
            }
            idx = next_idx;
        }

        Self::new(child_layout)
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

// トーナメント選択（最小化）：k個体から最良（最小スコア）を選ぶ
fn tournament_index(population: &[Individual], k: usize, rng: &mut fastrand::Rng) -> usize {
    let n = population.len();
    let mut best_idx = rng.usize(0..n);
    let mut best_score = population[best_idx].score;
    for _ in 1..k {
        let idx = rng.usize(0..n);
        let s = population[idx].score;
        if s < best_score {
            best_score = s;
            best_idx = idx;
        }
    }
    best_idx
}

impl PartialEq for Individual {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}
