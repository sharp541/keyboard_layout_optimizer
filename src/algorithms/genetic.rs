use fastrand;
use rand::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;

use crate::azik_extension::{
    AzikExtensionToken, AZIK_EXTENSION_TOKENS, AZIK_EXTENSION_TOKEN_COUNT,
};
use crate::keyboard_layout::{
    LayoutLookup, LogicalLayout, PhysicalLayout, NUM_COLS, NUM_LAYERS, NUM_ROWS,
};
use crate::n_gram::{LogicalNGram, NGramDB};

const TOTAL_LOGICAL_KEYS: usize = NUM_COLS * NUM_ROWS * NUM_LAYERS;

pub struct Genetic {
    population_size: usize,
    island_size: usize,
}

pub struct OptimizeConfig {
    pub iterations: usize,
    pub shuffle: bool,
    pub early_stop_count: usize,
    pub ja_weight: f32,
    pub en_weight: f32,
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
        config: OptimizeConfig,
    ) {
        let OptimizeConfig {
            iterations,
            shuffle,
            early_stop_count,
            ja_weight,
            en_weight,
        } = config;
        let (ja_weight, en_weight) = normalize_language_weights(ja_weight, en_weight);
        let mut initial_layout = LogicalLayout::from_usable_chars(usable_chars);
        initial_layout.assign_default_azik_extensions();
        let mut best_layout = Individual::new(initial_layout.clone());
        let split_tri_grams = ngram_db
            .get_split_tri_grams_for_layout(|c| initial_layout.resolve_char_index(c).is_some())
            .expect("Failed to get tri grams");
        let char_to_id: HashMap<char, usize> = usable_chars
            .iter()
            .enumerate()
            .map(|(i, &c)| (c, i))
            .collect();
        let ja_tri_grams_ids = tri_grams_to_ids(&split_tri_grams.japanese, &char_to_id);
        let en_tri_grams_ids = tri_grams_to_ids(&split_tri_grams.english, &char_to_id);
        best_layout.score = evaluate_weighted_ids(
            &best_layout.layout,
            physical_layout,
            &ja_tri_grams_ids,
            &en_tri_grams_ids,
            ja_weight,
            en_weight,
        );

        let mut rng_fast = fastrand::Rng::new();
        let mut islands = Vec::with_capacity(self.island_size);
        for _ in 0..self.island_size {
            let mut population = Vec::with_capacity(self.population_size);
            for _ in 0..self.population_size {
                let mut individual = Individual::new(initial_layout.clone());
                if shuffle {
                    let initial_mutation_steps = 3 + rng_fast.usize(0..4);
                    individual.mutate_steps(&mut rng_fast, initial_mutation_steps);
                }
                individual.score = evaluate_weighted_ids(
                    &individual.layout,
                    physical_layout,
                    &ja_tri_grams_ids,
                    &en_tri_grams_ids,
                    ja_weight,
                    en_weight,
                );
                population.push(individual);
            }
            islands.push(population);
        }

        let elite_num = if self.population_size % 2 == 0 { 2 } else { 1 };
        let mut count = 0;
        for i in 0..iterations {
            let stagnation = count;
            // 並列化：島ごとに独立処理
            islands.par_iter_mut().for_each(|population| {
                // スレッドローカルの軽量RNG
                let mut local_rng = fastrand::Rng::new();
                let mutation_steps = mutation_steps_for_stagnation(stagnation);
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
                    let mut child = if local_rng.u8(0..2) == 0 {
                        parent1.base_crossover(parent2, &mut rng)
                    } else {
                        parent1.extension_crossover(parent2, &mut local_rng)
                    };
                    child.mutate_steps(&mut local_rng, mutation_steps);
                    children.push(child);
                }
                new_population.append(&mut children);

                *population = new_population;

                // Evaluate population
                for i in population.iter_mut() {
                    i.score = evaluate_weighted_ids(
                        &i.layout,
                        physical_layout,
                        &ja_tri_grams_ids,
                        &en_tri_grams_ids,
                        ja_weight,
                        en_weight,
                    );
                }

                // 完全ソートを避ける（次反復のエリート抽出は部分選択で行う）
            });

            if stagnation > 0 && stagnation % 500 == 0 {
                for population in &mut islands {
                    diversify_population(
                        population,
                        elite_num,
                        physical_layout,
                        &ja_tri_grams_ids,
                        &en_tri_grams_ids,
                        ja_weight,
                        en_weight,
                    );
                }
            }

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
            let current_best_layout = islands
                .iter()
                .filter_map(|population| {
                    population.iter().min_by(|a, b| {
                        a.score
                            .partial_cmp(&b.score)
                            .expect("Failed to compare scores")
                    })
                })
                .min_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .expect("Failed to compare scores")
                });
            if let Some(current_best_layout) = current_best_layout {
                if current_best_layout.score < best_layout.score {
                    best_layout = current_best_layout.clone();
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

fn normalize_language_weights(ja_weight: f32, en_weight: f32) -> (f32, f32) {
    let ja = ja_weight.max(0.0);
    let en = en_weight.max(0.0);
    let sum = ja + en;
    if sum <= f32::EPSILON {
        (0.5, 0.5)
    } else {
        (ja / sum, en / sum)
    }
}

fn tri_grams_to_ids(
    tri_grams: &HashMap<LogicalNGram<3>, f32>,
    char_to_id: &HashMap<char, usize>,
) -> Vec<([LayoutLookup; 3], f32)> {
    tri_grams
        .iter()
        .map(|(ng, score)| {
            let lookup = |c| {
                if let Some(token) = AzikExtensionToken::from_char(c) {
                    return LayoutLookup::AzikExtension(token);
                }
                char_to_id
                    .get(&c)
                    .copied()
                    .map(LayoutLookup::CharId)
                    .unwrap_or(LayoutLookup::Char(c))
            };
            (
                [lookup(ng.get(0)), lookup(ng.get(1)), lookup(ng.get(2))],
                *score,
            )
        })
        .collect()
}

fn evaluate_weighted_ids(
    layout: &LogicalLayout,
    physical_layout: &PhysicalLayout,
    ja_tri_grams_ids: &[([LayoutLookup; 3], f32)],
    en_tri_grams_ids: &[([LayoutLookup; 3], f32)],
    ja_weight: f32,
    en_weight: f32,
) -> f32 {
    let (ja_weight, en_weight) = normalize_language_weights(ja_weight, en_weight);
    let ja_score = layout.evaluate_ids(physical_layout, ja_tri_grams_ids);
    let en_score = layout.evaluate_ids(physical_layout, en_tri_grams_ids);
    ja_weight * ja_score + en_weight * en_score
}

#[derive(Debug, Clone)]
struct Individual {
    layout: LogicalLayout,
    score: f32,
}

impl Individual {
    fn new(mut layout: LogicalLayout) -> Self {
        if layout.extension_assignments().is_empty() {
            layout.assign_default_azik_extensions();
        }
        Self { layout, score: 0.0 }
    }

    fn base_crossover<R: Rng + ?Sized>(&self, other: &Self, rng: &mut R) -> Self {
        // Cyclic crossover for permutations:
        // Start from a random index, follow the cycle of positions defined
        // by mapping parent1's value into the index where that value appears in parent2.
        // Positions in the cycle take values from parent1; others remain from parent2.
        let n = self.layout.len();
        let mut child_layout = other.layout.clone();

        let start = rng.gen_range(0..n);
        let mut idx = start;
        let mut visited = [false; TOTAL_LOGICAL_KEYS];

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

        let mut child = Self::new(child_layout);
        let mut repair_rng = fastrand::Rng::new();
        child.repair_extensions(&mut repair_rng);
        child
    }

    fn extension_crossover(&self, other: &Self, rng: &mut fastrand::Rng) -> Self {
        let mut child = self.clone();
        let assignments = crossover_extension_assignments(&child.layout, self, other, rng);
        child.rebuild_extensions(&assignments);
        child.repair_extensions(rng);
        child
    }

    fn base_mutation(&mut self, rng: &mut fastrand::Rng) {
        let Some((a, b)) = self.pick_base_mutation_indices(rng) else {
            return;
        };
        self.layout.swap(a, b);
        self.repair_extensions(rng);
    }

    fn extension_mutation(&mut self, rng: &mut fastrand::Rng) {
        let Some(mut assignments) =
            complete_extension_parent_indices(self.layout.extension_parent_indices())
        else {
            self.repair_extensions(rng);
            return;
        };

        let mut occupied = [false; TOTAL_LOGICAL_KEYS];
        for index in assignments {
            occupied[index] = true;
        }

        let mut available_indices = [0usize; TOTAL_LOGICAL_KEYS];
        let mut available_count = 0usize;
        for (index, slot) in occupied.iter().enumerate().take(self.layout.len()) {
            if self.layout.can_host_extension(index) && !*slot {
                available_indices[available_count] = index;
                available_count += 1;
            }
        }

        if available_count > 0 {
            let move_from = rng.usize(0..AZIK_EXTENSION_TOKEN_COUNT);
            let move_to = available_indices[rng.usize(0..available_count)];
            assignments[move_from] = move_to;
            self.rebuild_extensions(&assignments);
            return;
        }

        if AZIK_EXTENSION_TOKEN_COUNT < 2 {
            return;
        }

        let a = rng.usize(0..AZIK_EXTENSION_TOKEN_COUNT);
        let mut b = rng.usize(0..AZIK_EXTENSION_TOKEN_COUNT);
        while a == b {
            b = rng.usize(0..AZIK_EXTENSION_TOKEN_COUNT);
        }
        assignments.swap(a, b);
        self.rebuild_extensions(&assignments);
    }

    fn rebuild_extensions(&mut self, assignments: &[usize; AZIK_EXTENSION_TOKEN_COUNT]) {
        self.layout.clear_extensions();
        for (token_index, &index) in assignments.iter().enumerate() {
            let token = AZIK_EXTENSION_TOKENS[token_index];
            self.layout
                .assign_extension(index, token)
                .expect("extension assignments should stay valid after rebuild");
        }
    }

    fn pick_base_mutation_indices(&self, rng: &mut fastrand::Rng) -> Option<(usize, usize)> {
        if self.layout.len() < 2 {
            return None;
        }

        let first = rng.usize(0..self.layout.len());
        let mut second = rng.usize(0..self.layout.len());
        while first == second {
            second = rng.usize(0..self.layout.len());
        }

        Some((first, second))
    }

    fn repair_extensions(&mut self, rng: &mut fastrand::Rng) {
        let mut assignments = [None; AZIK_EXTENSION_TOKEN_COUNT];
        let mut occupied = [false; TOTAL_LOGICAL_KEYS];
        let mut hostable_count = 0usize;

        for index in 0..self.layout.len() {
            if !self.layout.can_host_extension(index) {
                continue;
            }
            hostable_count += 1;
        }
        assert!(
            hostable_count >= AZIK_EXTENSION_TOKENS.len(),
            "not enough consonant keys to host all AZIK extensions"
        );

        for (token_index, parent_index) in self
            .layout
            .extension_parent_indices()
            .iter()
            .copied()
            .enumerate()
        {
            if let Some(index) = parent_index {
                if self.layout.can_host_extension(index) && !occupied[index] {
                    assignments[token_index] = Some(index);
                    occupied[index] = true;
                }
            }
        }

        let mut missing_tokens = [0usize; AZIK_EXTENSION_TOKEN_COUNT];
        let mut missing_count = 0usize;
        for (token_index, assignment) in assignments.iter().enumerate() {
            if assignment.is_none() {
                missing_tokens[missing_count] = token_index;
                missing_count += 1;
            }
        }

        let mut available_indices = [0usize; TOTAL_LOGICAL_KEYS];
        let mut available_count = 0usize;
        for (index, slot) in occupied.iter().enumerate().take(self.layout.len()) {
            if self.layout.can_host_extension(index) && !*slot {
                available_indices[available_count] = index;
                available_count += 1;
            }
        }

        shuffle_slice(&mut missing_tokens[..missing_count], rng);
        shuffle_slice(&mut available_indices[..available_count], rng);

        for offset in 0..missing_count {
            assignments[missing_tokens[offset]] = Some(available_indices[offset]);
        }

        let assignments = complete_extension_parent_indices(&assignments)
            .expect("repair should restore every AZIK extension");
        self.rebuild_extensions(&assignments);
    }

    fn mutate(&mut self, rng: &mut fastrand::Rng) {
        let mutation_type = rng.u8(0..5);
        match mutation_type {
            0 => (),
            1 | 2 => self.base_mutation(rng),
            _ => self.extension_mutation(rng),
        }
    }

    fn mutate_steps(&mut self, rng: &mut fastrand::Rng, steps: usize) {
        for _ in 0..steps.max(1) {
            self.mutate(rng);
        }
    }
}

fn mutation_steps_for_stagnation(stagnation: usize) -> usize {
    match stagnation {
        0..=499 => 1,
        500..=1499 => 2,
        1500..=2999 => 3,
        _ => 4,
    }
}

fn diversify_population(
    population: &mut [Individual],
    elite_num: usize,
    physical_layout: &PhysicalLayout,
    ja_tri_grams_ids: &[([LayoutLookup; 3], f32)],
    en_tri_grams_ids: &[([LayoutLookup; 3], f32)],
    ja_weight: f32,
    en_weight: f32,
) {
    if population.len() <= elite_num {
        return;
    }

    population.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .expect("Failed to compare scores")
    });

    let mut rng = fastrand::Rng::new();
    let elite_templates = population[..elite_num].to_vec();
    let replace_count = ((population.len() - elite_num) / 4).max(1);

    for slot in population.iter_mut().rev().take(replace_count) {
        let template = &elite_templates[rng.usize(0..elite_templates.len())];
        let mut diversified = template.clone();
        let diversification_steps = 6 + rng.usize(0..6);
        diversified.mutate_steps(&mut rng, diversification_steps);
        diversified.score = evaluate_weighted_ids(
            &diversified.layout,
            physical_layout,
            ja_tri_grams_ids,
            en_tri_grams_ids,
            ja_weight,
            en_weight,
        );
        *slot = diversified;
    }
}

fn crossover_extension_assignments(
    layout: &LogicalLayout,
    left: &Individual,
    right: &Individual,
    rng: &mut fastrand::Rng,
) -> [usize; AZIK_EXTENSION_TOKEN_COUNT] {
    let left_assignments =
        complete_extension_parent_indices(left.layout.extension_parent_indices())
            .expect("left parent should have complete AZIK assignments");
    let right_assignments =
        complete_extension_parent_indices(right.layout.extension_parent_indices())
            .expect("right parent should have complete AZIK assignments");
    let mut occupied_indices = [false; TOTAL_LOGICAL_KEYS];
    let mut assignments = [None; AZIK_EXTENSION_TOKEN_COUNT];
    let mut tokens = AZIK_EXTENSION_TOKENS;
    shuffle_slice(&mut tokens, rng);

    for token in tokens {
        let token_index = token.as_usize();
        let pick_left_first = rng.u8(0..2) == 0;
        let candidate_indices = if pick_left_first {
            [
                left_assignments[token_index],
                right_assignments[token_index],
            ]
        } else {
            [
                right_assignments[token_index],
                left_assignments[token_index],
            ]
        };

        if let Some(index) = candidate_indices
            .into_iter()
            .find(|&index| layout.can_host_extension(index) && !occupied_indices[index])
        {
            occupied_indices[index] = true;
            assignments[token_index] = Some(index);
        }
    }

    let mut available_indices = [0usize; TOTAL_LOGICAL_KEYS];
    let mut available_count = 0usize;
    for (index, slot) in occupied_indices.iter().enumerate().take(layout.len()) {
        if layout.can_host_extension(index) && !*slot {
            available_indices[available_count] = index;
            available_count += 1;
        }
    }
    shuffle_slice(&mut available_indices[..available_count], rng);

    let mut next_available = 0usize;
    for assignment in assignments.iter_mut().take(AZIK_EXTENSION_TOKEN_COUNT) {
        if assignment.is_none() {
            let index = available_indices[next_available];
            next_available += 1;
            *assignment = Some(index);
        }
    }

    complete_extension_parent_indices(&assignments)
        .expect("crossover should assign every AZIK extension")
}

fn complete_extension_parent_indices(
    assignments: &[Option<usize>; AZIK_EXTENSION_TOKEN_COUNT],
) -> Option<[usize; AZIK_EXTENSION_TOKEN_COUNT]> {
    let mut completed = [0usize; AZIK_EXTENSION_TOKEN_COUNT];
    for (i, index) in assignments.iter().copied().enumerate() {
        completed[i] = index?;
    }
    Some(completed)
}

fn shuffle_slice<T>(slice: &mut [T], rng: &mut fastrand::Rng) {
    for i in (1..slice.len()).rev() {
        let j = rng.usize(..=i);
        slice.swap(i, j);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::azik_extension::AzikExtensionToken;
    use crate::keyboard_layout::Finger;
    use std::collections::HashSet;

    fn layout_for_tests() -> LogicalLayout {
        LogicalLayout::from_usable_chars(&[
            'k', 's', 't', 'n', 'm', 'r', 'w', 'z', 'd', 'g', 'b', 'p', 'f', 'h', 'y',
        ])
    }

    fn mixed_layout_for_tests() -> LogicalLayout {
        LogicalLayout::from_usable_chars(&[
            'k', 'a', 's', 'i', 't', 'u', 'n', 'e', 'm', 'o', 'r', 'w', 'z', 'd', 'h', 'y',
        ])
    }

    fn weighted_physical_layout() -> PhysicalLayout {
        let cost_matrix = std::array::from_fn(|index| index as f32 + 1.0);
        let finger_matrix = std::array::from_fn(|_| Finger::I);
        let mut physical_layout =
            PhysicalLayout::new(cost_matrix, finger_matrix).expect("layout should be valid");
        physical_layout.calculate_tri_gram_cost();
        physical_layout
    }

    fn assert_valid_extension_assignments(layout: &LogicalLayout) {
        let assignments = layout.extension_assignments();
        let assigned_indices: HashSet<usize> =
            assignments.iter().map(|(index, _)| *index).collect();
        let assigned_tokens: HashSet<AzikExtensionToken> =
            assignments.iter().map(|(_, token)| *token).collect();

        assert_eq!(assignments.len(), AZIK_EXTENSION_TOKENS.len());
        assert_eq!(assigned_indices.len(), AZIK_EXTENSION_TOKENS.len());
        assert_eq!(
            assigned_tokens,
            AZIK_EXTENSION_TOKENS.iter().copied().collect()
        );

        for token in AZIK_EXTENSION_TOKENS {
            let index = layout
                .get_extension_parent_index(token)
                .expect("every AZIK token should be assigned exactly once");
            assert!(layout.can_host_extension(index));
        }
    }

    #[test]
    fn new_individual_initializes_all_azik_extensions() {
        let individual = Individual::new(layout_for_tests());

        assert_valid_extension_assignments(&individual.layout);
    }

    #[test]
    fn extension_mutation_preserves_base_layout_and_changes_extension_assignment() {
        let mut individual = Individual::new(layout_for_tests());
        let base_layout = individual.layout.output();
        let before = individual.layout.extension_assignments();

        individual.extension_mutation(&mut fastrand::Rng::with_seed(7));

        assert_eq!(individual.layout.output(), base_layout);
        let after = individual.layout.extension_assignments();
        assert_ne!(before, after);
        assert_valid_extension_assignments(&individual.layout);
    }

    #[test]
    fn base_mutation_repairs_extensions_after_base_layout_changes() {
        let mut saw_base_layout_change = false;
        let mut saw_cross_bucket_swap = false;

        for seed in 0..64 {
            let mut individual = Individual::new(mixed_layout_for_tests());
            let before_layout = individual.layout.output();
            let before_extensions = individual.layout.extension_assignments();
            let before_hostable = (0..individual.layout.len())
                .map(|index| individual.layout.can_host_extension(index))
                .collect::<Vec<_>>();

            individual.base_mutation(&mut fastrand::Rng::with_seed(seed));

            saw_base_layout_change |= individual.layout.output() != before_layout;
            let after_hostable = (0..individual.layout.len())
                .map(|index| individual.layout.can_host_extension(index))
                .collect::<Vec<_>>();
            saw_cross_bucket_swap |= before_hostable != after_hostable;

            assert_eq!(
                individual.layout.extension_assignments().len(),
                before_extensions.len(),
                "base mutation should preserve the number of assigned extensions"
            );
            assert_valid_extension_assignments(&individual.layout);
        }

        assert!(
            saw_base_layout_change,
            "test should exercise at least one real base-layout mutation"
        );
        assert!(
            saw_cross_bucket_swap,
            "test should exercise at least one vowel/consonant swap across extension hostability"
        );
    }

    #[test]
    fn extension_assignment_changes_evaluation_score() {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let physical_layout = weighted_physical_layout();
                let ann = AzikExtensionToken::Ann.as_char();
                let tri_grams = HashMap::from([(LogicalNGram::new([ann, 'k', 'k']), 1.0)]);

                let mut left = layout_for_tests();
                left.clear_extensions();
                left.assign_extension(0, AzikExtensionToken::Ann)
                    .expect("index 0 should host ann");
                left.assign_extension(1, AzikExtensionToken::Inn)
                    .expect("index 1 should host inn");
                left.assign_extension(2, AzikExtensionToken::Unn)
                    .expect("index 2 should host unn");
                left.assign_extension(3, AzikExtensionToken::Enn)
                    .expect("index 3 should host enn");
                left.assign_extension(4, AzikExtensionToken::Onn)
                    .expect("index 4 should host onn");
                left.assign_extension(5, AzikExtensionToken::Ai)
                    .expect("index 5 should host ai");
                left.assign_extension(6, AzikExtensionToken::Uu)
                    .expect("index 6 should host uu");
                left.assign_extension(7, AzikExtensionToken::Ei)
                    .expect("index 7 should host ei");
                left.assign_extension(8, AzikExtensionToken::Ou)
                    .expect("index 8 should host ou");

                let mut right = layout_for_tests();
                right.clear_extensions();
                right
                    .assign_extension(9, AzikExtensionToken::Ann)
                    .expect("index 9 should host ann");
                right
                    .assign_extension(1, AzikExtensionToken::Inn)
                    .expect("index 1 should host inn");
                right
                    .assign_extension(2, AzikExtensionToken::Unn)
                    .expect("index 2 should host unn");
                right
                    .assign_extension(3, AzikExtensionToken::Enn)
                    .expect("index 3 should host enn");
                right
                    .assign_extension(4, AzikExtensionToken::Onn)
                    .expect("index 4 should host onn");
                right
                    .assign_extension(5, AzikExtensionToken::Ai)
                    .expect("index 5 should host ai");
                right
                    .assign_extension(6, AzikExtensionToken::Uu)
                    .expect("index 6 should host uu");
                right
                    .assign_extension(7, AzikExtensionToken::Ei)
                    .expect("index 7 should host ei");
                right
                    .assign_extension(8, AzikExtensionToken::Ou)
                    .expect("index 8 should host ou");

                let left_score = left.evaluate(&physical_layout, &tri_grams);
                let right_score = right.evaluate(&physical_layout, &tri_grams);

                assert_ne!(left_score, right_score);
            })
            .expect("thread should spawn")
            .join()
            .expect("thread should finish");
    }

    #[test]
    fn base_crossover_changes_only_base_layout_when_hostability_is_stable() {
        let left = Individual::new(layout_for_tests());
        let mut right_layout = LogicalLayout::from_usable_chars(&[
            'p', 'f', 'b', 'g', 'd', 'z', 'w', 'r', 'm', 'n', 't', 's', 'k', 'h', 'y',
        ]);
        right_layout.clear_extensions();
        right_layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("index 0 should host ann");
        right_layout
            .assign_extension(1, AzikExtensionToken::Inn)
            .expect("index 1 should host inn");
        right_layout
            .assign_extension(2, AzikExtensionToken::Unn)
            .expect("index 2 should host unn");
        right_layout
            .assign_extension(3, AzikExtensionToken::Enn)
            .expect("index 3 should host enn");
        right_layout
            .assign_extension(4, AzikExtensionToken::Onn)
            .expect("index 4 should host onn");
        right_layout
            .assign_extension(5, AzikExtensionToken::Ai)
            .expect("index 5 should host ai");
        right_layout
            .assign_extension(6, AzikExtensionToken::Uu)
            .expect("index 6 should host uu");
        right_layout
            .assign_extension(7, AzikExtensionToken::Ei)
            .expect("index 7 should host ei");
        right_layout
            .assign_extension(8, AzikExtensionToken::Ou)
            .expect("index 8 should host ou");
        let right = Individual::new(right_layout);

        let before_base = right.layout.output();
        let before_extensions = right.layout.extension_assignments();
        let mut saw_base_change = false;

        for seed in 0..32 {
            let mut rng = StdRng::seed_from_u64(seed);
            let child = left.base_crossover(&right, &mut rng);
            saw_base_change |= child.layout.output() != before_base;
            assert_eq!(child.layout.extension_assignments(), before_extensions);
            assert_valid_extension_assignments(&child.layout);
        }

        assert!(
            saw_base_change,
            "base crossover should change the base layout for at least one start position"
        );
    }

    #[test]
    fn extension_crossover_changes_only_extension_assignments() {
        let left = Individual::new(layout_for_tests());
        let mut right_layout = layout_for_tests();
        right_layout.clear_extensions();
        right_layout
            .assign_extension(9, AzikExtensionToken::Ann)
            .expect("index 9 should host ann");
        right_layout
            .assign_extension(1, AzikExtensionToken::Inn)
            .expect("index 1 should host inn");
        right_layout
            .assign_extension(2, AzikExtensionToken::Unn)
            .expect("index 2 should host unn");
        right_layout
            .assign_extension(3, AzikExtensionToken::Enn)
            .expect("index 3 should host enn");
        right_layout
            .assign_extension(4, AzikExtensionToken::Onn)
            .expect("index 4 should host onn");
        right_layout
            .assign_extension(5, AzikExtensionToken::Ai)
            .expect("index 5 should host ai");
        right_layout
            .assign_extension(6, AzikExtensionToken::Uu)
            .expect("index 6 should host uu");
        right_layout
            .assign_extension(7, AzikExtensionToken::Ei)
            .expect("index 7 should host ei");
        right_layout
            .assign_extension(8, AzikExtensionToken::Ou)
            .expect("index 8 should host ou");
        let right = Individual::new(right_layout);

        let before_base = left.layout.output();
        let before_extensions = left.layout.extension_assignments();
        let child = left.extension_crossover(&right, &mut fastrand::Rng::with_seed(3));

        assert_eq!(child.layout.output(), before_base);
        assert_ne!(child.layout.extension_assignments(), before_extensions);
        assert_valid_extension_assignments(&child.layout);
    }

    #[test]
    fn repair_extensions_restores_all_tokens_after_invalidating_a_host_key() {
        let mut individual = Individual::new(layout_for_tests());
        let ann_index = individual
            .layout
            .get_extension_parent_index(AzikExtensionToken::Ann)
            .expect("ann should be assigned");
        individual.layout.set(ann_index, 'a');

        assert_eq!(
            individual
                .layout
                .get_extension_parent_index(AzikExtensionToken::Ann),
            None
        );

        individual.repair_extensions(&mut fastrand::Rng::with_seed(11));

        assert_valid_extension_assignments(&individual.layout);
    }

    #[test]
    fn weighted_evaluation_normalizes_language_weights() {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let physical_layout = weighted_physical_layout();
                let layout = layout_for_tests();
                let ja_tri_grams_ids = vec![(
                    [
                        LayoutLookup::Char('k'),
                        LayoutLookup::Char('s'),
                        LayoutLookup::Char('t'),
                    ],
                    1.0,
                )];
                let en_tri_grams_ids = vec![(
                    [
                        LayoutLookup::Char('n'),
                        LayoutLookup::Char('h'),
                        LayoutLookup::Char('m'),
                    ],
                    1.0,
                )];

                let normalized = evaluate_weighted_ids(
                    &layout,
                    &physical_layout,
                    &ja_tri_grams_ids,
                    &en_tri_grams_ids,
                    0.5,
                    0.5,
                );
                let unnormalized_same_ratio = evaluate_weighted_ids(
                    &layout,
                    &physical_layout,
                    &ja_tri_grams_ids,
                    &en_tri_grams_ids,
                    1.0,
                    1.0,
                );

                assert_eq!(normalized, unnormalized_same_ratio);
            })
            .expect("thread should spawn")
            .join()
            .expect("thread should finish");
    }
}
