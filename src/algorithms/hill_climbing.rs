use rand::prelude::*;

use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::NGramDB;
pub struct HillClimbing {}

impl HillClimbing {
    pub fn optimize(layout: &mut LogicalLayout, n_gram_db: &NGramDB, iterations: usize) {
        let mut rng = thread_rng();
        let mut best_cost = 10e10;
        let tri_grams = n_gram_db.get_tri_grams().expect("Failed to get 3-grams");
        for _ in 0..iterations {
            let a = rng.gen_range(0..layout.len());
            let b = rng.gen_range(0..layout.len());
            layout.swap(a, b);
            let new_cost = layout.evaluate(&tri_grams);
            if new_cost < best_cost {
                best_cost = new_cost;
            } else {
                layout.swap(a, b);
            }
        }
    }
}
