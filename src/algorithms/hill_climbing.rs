use rand::prelude::*;

use crate::interfaces::Algorithm;
use crate::keyboard_layout::LogicalLayout;

pub struct HillClimbing {}

impl<const N: usize> Algorithm<N> for HillClimbing {
    fn optimize(&self, layout: &mut LogicalLayout, text: &str, iterations: usize) {
        let mut rng = thread_rng();
        let mut best_cost = 10e10;
        for _ in 0..iterations {
            let a = rng.gen_range(0..layout.len());
            let b = rng.gen_range(0..layout.len());
            layout.swap(a, b);
            let new_cost = layout.evaluate::<N>(text);
            if new_cost < best_cost {
                best_cost = new_cost;
            } else {
                layout.swap(a, b);
            }
        }
    }
}
