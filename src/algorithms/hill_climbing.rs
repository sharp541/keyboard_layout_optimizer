use rand::prelude::*;
use std::collections::HashMap;

use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::LogicalNGram;
pub struct HillClimbing {}

impl HillClimbing {
    pub fn optimize(
        layout: &mut LogicalLayout,
        tri_grams: &HashMap<LogicalNGram<3>, f32>,
        iterations: usize,
    ) {
        let mut rng = thread_rng();
        let mut best_cost = 10e10;
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
