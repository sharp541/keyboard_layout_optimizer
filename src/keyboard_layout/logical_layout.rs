use std::collections::{HashMap, HashSet};
use std::iter;

use super::physical_layout::PhysicalLayout;
use crate::n_gram::{LogicalNGram, PhysicalNGram};

#[derive(Debug)]
pub struct LogicalLayout<'a> {
    layout: Vec<Option<char>>,
    usable_chars: HashMap<char, usize>,
    physical_layout: &'a PhysicalLayout,
}

impl<'a> LogicalLayout<'a> {
    pub fn new(physical_layout: &'a PhysicalLayout, usable_chars: Vec<char>) -> Self {
        let mut layout: Vec<Option<char>> = Vec::new();
        for key in &usable_chars {
            layout.push(Some(key.clone()));
        }
        if layout.len() < physical_layout.len() {
            layout.extend(iter::repeat(None).take(physical_layout.len() - layout.len()));
        }
        let usable_chars = usable_chars
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c, i))
            .collect();
        LogicalLayout {
            layout,
            usable_chars,
            physical_layout,
        }
    }

    pub fn evaluate<const N: usize>(&self, n_gram: &LogicalNGram<N>) -> f32 {
        let mut physical_n_gram = PhysicalNGram::new([0; N]);
        for i in 0..N {
            let key = &n_gram.get(i);
            match self.usable_chars.get(key) {
                Some(idx) => {
                    physical_n_gram.set(i, *idx);
                }
                None => physical_n_gram.set(i, self.usable_chars.len()),
            };
        }
        self.physical_layout.cost(physical_n_gram)
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if let Some(a_char) = self.layout[a] {
            self.usable_chars.insert(a_char, b);
        }
        if let Some(b_char) = self.layout[b] {
            self.usable_chars.insert(b_char, a);
        }
        self.layout.swap(a, b);
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }

    pub fn usable_chars(&self) -> HashSet<char> {
        self.usable_chars.keys().cloned().collect()
    }
}

impl<'a> std::fmt::Display for LogicalLayout<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\n")?;
        self.physical_layout.print(self.layout.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logical_layout() {
        let cost_matrix = vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.0, 3.0],
            vec![2.0, 3.0, 0.0],
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix).unwrap();
        let mut logical_layout = LogicalLayout::new(&physical_layout, vec!['a', 'b', 'c']);
        assert_eq!(logical_layout.len(), 9);
        assert_eq!(logical_layout.usable_chars().len(), 3);
        assert_eq!(
            logical_layout.evaluate(&LogicalNGram::new(['a', 'b', 'c'])),
            3.0
        );
        logical_layout.swap(0, 7);
        assert_eq!(
            logical_layout.evaluate(&LogicalNGram::new(['b', 'a', 'c'])),
            6.0
        );
    }
}
