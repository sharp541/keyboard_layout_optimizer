use rayon::prelude::*;
use std::collections::HashMap;

use super::physical_layout::PhysicalLayout;
use crate::n_gram::{LogicalNGram, PhysicalNGram};
#[derive(Debug, Clone)]
pub struct LogicalLayout {
    layout: Vec<char>,
    usable_chars: HashMap<char, usize>,
}

impl LogicalLayout {
    pub fn from_usable_chars(physical_layout: &PhysicalLayout, usable_chars: Vec<char>) -> Self {
        let mut layout: Vec<char> = usable_chars.to_vec();
        let mut usable_chars: HashMap<char, usize> = usable_chars
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c, i))
            .collect();
        let mut next_char = ' ';

        while layout.len() < physical_layout.len() {
            while usable_chars.contains_key(&next_char) {
                next_char = ((next_char as u8) + 1) as char;
            }
            layout.push(next_char);
            usable_chars.insert(next_char, layout.len() - 1);
        }
        LogicalLayout {
            layout,
            usable_chars,
        }
    }

    pub fn evaluate(
        &self,
        physical_layout: &PhysicalLayout,
        tri_grams: &HashMap<LogicalNGram<3>, f32>,
    ) -> f32 {
        let cost = tri_grams
            .par_iter()
            .map(|(n_gram, score)| -> f32 {
                let physical_n_gram = PhysicalNGram::new([
                    self.get_char_index(n_gram.get(0)),
                    self.get_char_index(n_gram.get(1)),
                    self.get_char_index(n_gram.get(2)),
                ]);
                *score * physical_layout.get_tri_gram_cost(&physical_n_gram)
            })
            .sum();
        cost
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.usable_chars.insert(self.layout[a], b);
        self.usable_chars.insert(self.layout[b], a);
        self.layout.swap(a, b);
    }

    pub fn get_char_index(&self, c: char) -> usize {
        *self.usable_chars.get(&c).unwrap_or(&self.layout.len())
    }

    pub fn get(&self, index: usize) -> char {
        self.layout[index]
    }

    pub fn set(&mut self, index: usize, c: char) {
        self.usable_chars.insert(c, index);
        self.layout[index] = c;
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }

    pub fn char_nums(&self) -> usize {
        self.usable_chars.len()
    }

    pub fn output(self) -> Vec<char> {
        self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_layout::*;
    use crate::keyboard_layout::hand_model::Finger as F;

    #[test]
    fn test_from_usable_chars() {
        let cost_matrix: [[f32; NUM_COLS]; NUM_ROWS] = [
            [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0], // 上段
            [1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2], // 下段
        ];
        let finger_table: [[F; NUM_COLS]; NUM_ROWS] = [
            [F::R, F::R, F::M, F::M, F::I, F::I, F::M, F::M, F::R, F::R],
            [F::P, F::R, F::M, F::I, F::I, F::I, F::I, F::M, F::R, F::P],
            [F::P, F::R, F::M, F::I, F::I, F::I, F::I, F::M, F::R, F::P],
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix, finger_table).unwrap();
        let logical_layout =
            LogicalLayout::from_usable_chars(&physical_layout, vec!['a', 'b', 'c']);
        assert_eq!(logical_layout.len(), 30);
        assert_eq!(logical_layout.char_nums(), 3);
    }
}
