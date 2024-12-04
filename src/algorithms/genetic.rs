use super::Algorithm;
use crate::keyboard_layout::LogicalLayout;
use crate::n_gram::NGramDB;

pub struct Genetic {}

impl Algorithm for Genetic {
    fn optimize(&self, layout: &mut LogicalLayout, n_gram_db: &NGramDB, iterations: usize) {}
}
