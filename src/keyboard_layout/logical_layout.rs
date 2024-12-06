use std::collections::{HashMap, HashSet};
use std::iter;

use super::physical_layout::PhysicalLayout;
use crate::n_gram::{LogicalNGram, PhysicalNGram};

#[derive(Debug, Clone)]
pub struct LogicalLayout<'a> {
    layout: Vec<Option<char>>,
    usable_chars: HashMap<char, usize>,
    unassigned_chars: HashSet<usize>,
    physical_layout: &'a PhysicalLayout,
}

impl<'a> LogicalLayout<'a> {
    pub fn from_layout(physical_layout: &'a PhysicalLayout, mut layout: Vec<Option<char>>) -> Self {
        let mut unassigned_chars = HashSet::new();
        let mut usable_chars: HashMap<char, usize> = HashMap::new();
        for (i, c) in layout.iter().enumerate() {
            match c {
                Some(c) => {
                    usable_chars.insert(*c, i);
                }
                None => {
                    unassigned_chars.insert(i);
                }
            }
        }
        if layout.len() < physical_layout.len() {
            layout.extend(iter::repeat(None).take(physical_layout.len() - layout.len()));
            unassigned_chars.extend(layout.len()..physical_layout.len());
        }
        LogicalLayout {
            layout,
            usable_chars,
            unassigned_chars,
            physical_layout,
        }
    }

    pub fn from_usable_chars(physical_layout: &'a PhysicalLayout, usable_chars: Vec<char>) -> Self {
        let mut layout: Vec<Option<char>> = usable_chars.iter().map(|c| Some(*c)).collect();
        let mut unassigned_chars = HashSet::new();
        if layout.len() < physical_layout.len() {
            layout.extend(iter::repeat(None).take(physical_layout.len() - layout.len()));
            unassigned_chars.extend(layout.len()..physical_layout.len());
        }
        let usable_chars = usable_chars
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c, i))
            .collect();
        LogicalLayout {
            layout,
            usable_chars,
            unassigned_chars,
            physical_layout,
        }
    }

    pub fn evaluate(&self, tri_grams: &HashMap<LogicalNGram<3>, f32>) -> f32 {
        let mut cost = 0.0;

        for (n_gram, freq) in tri_grams {
            let physical_n_gram = PhysicalNGram::new([
                *self
                    .usable_chars
                    .get(&n_gram.get(0))
                    .unwrap_or(&self.layout.len()),
                *self
                    .usable_chars
                    .get(&n_gram.get(1))
                    .unwrap_or(&self.layout.len()),
                *self
                    .usable_chars
                    .get(&n_gram.get(2))
                    .unwrap_or(&self.layout.len()),
            ]);
            cost += self.physical_layout.stroke_cost(physical_n_gram) * freq;
        }
        cost
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

    pub fn get_char_index(&self, c: Option<char>) -> usize {
        match c {
            Some(c) => *self.usable_chars.get(&c).unwrap_or(&self.layout.len()),
            None => {
                if self.unassigned_chars.is_empty() {
                    panic!("No unassigned chars");
                }
                *self.unassigned_chars.iter().next().unwrap()
            }
        }
    }

    pub fn get(&self, index: usize) -> Option<char> {
        self.layout[index]
    }

    pub fn set(&mut self, index: usize, c: Option<char>) {
        match c {
            Some(c) => {
                self.usable_chars.insert(c, index);
            }
            None => {
                self.unassigned_chars.insert(index);
            }
        }
        self.layout[index] = c;
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }

    pub fn char_nums(&self) -> usize {
        self.usable_chars.len()
    }

    pub fn output(self) -> Vec<Option<char>> {
        self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_layout::*;

    #[test]
    fn test_from_usable_chars() {
        let cost_matrix: [[f32; NUM_COLS]; NUM_ROWS] = [
            [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0], // 上段
            [1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2], // 下段
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix).unwrap();
        let logical_layout =
            LogicalLayout::from_usable_chars(&physical_layout, vec!['a', 'b', 'c']);
        assert_eq!(logical_layout.len(), 30);
        assert_eq!(logical_layout.char_nums(), 3);
    }

    #[test]
    fn test_from_layout() {
        let cost_matrix: [[f32; NUM_COLS]; NUM_ROWS] = [
            [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0], // 上段
            [1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2], // 下段
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix).unwrap();
        let layout = vec![Some('a'), Some('b'), Some('c')];
        let logical_layout = LogicalLayout::from_layout(&physical_layout, layout);
        assert_eq!(logical_layout.len(), 30);
        assert_eq!(logical_layout.char_nums(), 3);
    }
}
