pub mod hill_climbing;

pub use hill_climbing::*;

use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::NGramDB;
pub trait Algorithm {
    fn optimize(&self, layout: &mut LogicalLayout, n_gram_db: &NGramDB, iterations: usize);
}
