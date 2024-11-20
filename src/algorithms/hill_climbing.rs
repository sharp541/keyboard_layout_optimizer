use rand::prelude::*;

use crate::interfaces::Algorithm;
use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::generate_n_grams;
pub struct HillClimbing {}

impl<const N: usize> Algorithm<N> for HillClimbing {
    fn optimize(
        &self,
        layout: &mut LogicalLayout,
        text: &str,
        iterations: usize,
        batch_size: usize,
    ) {
        let mut rng = thread_rng();
        let mut best_cost = 10e10;
        for _ in 0..iterations {
            generate_n_grams::<N>(text)
                .chunks(batch_size)
                .for_each(|n_grams| {
                    let a = rng.gen_range(0..layout.len());
                    let b = rng.gen_range(0..layout.len());
                    layout.swap(a, b);
                    let new_cost = n_grams
                        .iter()
                        .map(|n_gram| layout.evaluate::<N>(n_gram))
                        .sum::<f32>();
                    if new_cost < best_cost {
                        best_cost = new_cost;
                    } else {
                        layout.swap(a, b);
                    }
                });
        }
    }
}
