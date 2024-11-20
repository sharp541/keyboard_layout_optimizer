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
    use crate::keyboard_layout::*;

    #[test]
    fn test_logical_layout() {
        let cost_matrix: [[f32; NUM_COLS]; NUM_ROWS] = [
            [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0], // 上段
            [1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2], // 下段
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix).unwrap();
        let mut logical_layout = LogicalLayout::new(&physical_layout, vec!['a', 'b', 'c']);
        assert_eq!(logical_layout.len(), 30);
        assert_eq!(logical_layout.usable_chars().len(), 3);
        assert_eq!(logical_layout.evaluate::<1>(&LogicalNGram::new(['a'])), 3.0);
        assert_eq!(
            logical_layout.evaluate::<2>(&LogicalNGram::new(['a', 'b'])),
            5.4
        );
        logical_layout.swap(0, 10);
        assert_eq!(logical_layout.evaluate::<1>(&LogicalNGram::new(['a'])), 1.6);
    }
}
