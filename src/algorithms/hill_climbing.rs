use rand::prelude::*;

use crate::interfaces::Algorithm;
use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::generate_n_grams;

pub struct HillClimbing {}

impl<const N: usize> Algorithm<N> for HillClimbing {
    fn optimize(&self, layout: &mut LogicalLayout, text: &str) {
        let mut rng = thread_rng();
        let mut best_cost = 10e10;
        generate_n_grams::<N>(text).iter().for_each(|n_gram| {
            let a = rng.gen_range(0..layout.len());
            let b = rng.gen_range(0..layout.len());
            layout.swap(a, b);
            let new_cost = layout.evaluate(n_gram);
            if new_cost < best_cost {
                best_cost = new_cost;
            } else {
                layout.swap(a, b);
            }
        });
    }
}
