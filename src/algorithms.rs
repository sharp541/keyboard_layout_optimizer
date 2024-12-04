pub mod annealing;
pub mod genetic;
pub mod hill_climbing;

use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::NGramDB;
pub use annealing::*;
pub use genetic::*;
pub use hill_climbing::*;

pub trait Algorithm {
    fn optimize(&self, layout: &mut LogicalLayout, n_gram_db: &NGramDB, iterations: usize);
}
