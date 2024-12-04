use rand::prelude::*;

use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::NGramDB;

pub struct Annealing {
    // 初期温度
    initial_temperature: f32,
    // 冷却率
    cooling_rate: f32,
}

impl Annealing {
    pub fn new(initial_temperature: f32, cooling_rate: f32) -> Self {
        Annealing {
            initial_temperature,
            cooling_rate,
        }
    }
}

impl Annealing {
    pub fn optimize(&self, layout: &mut LogicalLayout, n_gram_db: &NGramDB, iterations: usize) {
        let mut rng = thread_rng();
        let mut current_cost = 10e10;
        let tri_grams = n_gram_db.get_tri_grams().expect("Failed to get 3-grams");

        // 現在の温度
        let mut temperature = self.initial_temperature;

        for _ in 0..iterations {
            let a = rng.gen_range(0..layout.len());
            let b = rng.gen_range(0..layout.len());

            layout.swap(a, b);
            let new_cost = layout.evaluate(&tri_grams);

            // コストの差分
            let delta = new_cost - current_cost;

            // 改善する場合は必ず採用
            // 悪化する場合は確率的に採用
            if delta < 0.0 || rng.gen::<f32>() < (-delta / temperature).exp() {
                current_cost = new_cost;
            } else {
                // 採用しない場合は元に戻す
                layout.swap(a, b);
            }

            // 温度を下げる
            temperature *= self.cooling_rate;
        }
    }
}
