use crate::keyboard_layout::LogicalLayout;

pub trait Algorithm<const N: usize> {
    fn optimize(&self, layout: &mut LogicalLayout, text: &str, iterations: usize);
}
