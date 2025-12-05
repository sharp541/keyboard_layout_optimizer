use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use super::physical_layout::{PhysicalLayout, NUM_COLS, NUM_LAYERS, NUM_ROWS};
use crate::n_gram::LogicalNGram;

#[derive(Debug, Clone)]
pub struct LogicalLayout {
    layout: [char; NUM_COLS * NUM_ROWS * NUM_LAYERS],
    char_map: HashMap<char, usize>,
    dummy_chars: HashSet<char>,
}

impl LogicalLayout {
    pub fn from_usable_chars(usable_chars: &[char]) -> Self {
        if usable_chars.len() > NUM_COLS * NUM_ROWS * NUM_LAYERS {
            panic!("Too many usable characters: {}", usable_chars.len());
        }

        let mut layout: [char; NUM_COLS * NUM_ROWS * NUM_LAYERS] =
            [' '; NUM_COLS * NUM_ROWS * NUM_LAYERS];
        let mut char_map = HashMap::new();
        let mut dummy_chars = HashSet::new();
        // Fill layout with provided usable chars; if not enough, use unique dummy chars
        // Use Unicode Private Use Area starting at U+E000 to avoid collisions
        let mut dummy_counter: u32 = 0;
        for i in 0..NUM_COLS * NUM_ROWS * NUM_LAYERS {
            if i < usable_chars.len() {
                let ch = usable_chars[i];
                layout[i] = ch;
                char_map.insert(ch, i);
            } else {
                // Generate a unique dummy character that doesn't collide with usable_chars
                let dummy_base: u32 = 0xE000; // Private Use Area start
                let mut dummy_ch = std::char::from_u32(dummy_base + dummy_counter)
                    .expect("Failed to create dummy character");
                // Ensure uniqueness and avoid accidental collision with usable chars
                while usable_chars.contains(&dummy_ch) || char_map.contains_key(&dummy_ch) {
                    dummy_counter += 1;
                    dummy_ch = std::char::from_u32(dummy_base + dummy_counter)
                        .expect("Failed to create dummy character");
                }
                layout[i] = dummy_ch;
                char_map.insert(dummy_ch, i);
                dummy_chars.insert(dummy_ch);
                dummy_counter += 1;
            }
        }
        LogicalLayout { layout, char_map, dummy_chars }
    }

    pub fn evaluate(
        &self,
        physical_layout: &PhysicalLayout,
        tri_grams: &HashMap<LogicalNGram<3>, f32>,
    ) -> f32 {
        tri_grams
            .iter()
            .map(|(n_gram, score)| -> f32 {
                let k1 = self.get_char_index(n_gram.get(0));
                let k2 = self.get_char_index(n_gram.get(1));
                let k3 = self.get_char_index(n_gram.get(2));
                *score * physical_layout.get_tri_gram_cost(k1, k2, k3)
            })
            .sum()
    }

    pub fn evaluate_par(
        &self,
        physical_layout: &PhysicalLayout,
        tri_grams_vec: &[(&LogicalNGram<3>, f32)],
    ) -> f32 {
        tri_grams_vec
            .par_iter()
            .map(|(n_gram, score)| -> f32 {
                let k1 = self.get_char_index(n_gram.get(0));
                let k2 = self.get_char_index(n_gram.get(1));
                let k3 = self.get_char_index(n_gram.get(2));
                *score * physical_layout.get_tri_gram_cost(k1, k2, k3)
            })
            .sum()
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.char_map.insert(self.layout[a], b);
        self.char_map.insert(self.layout[b], a);
        self.layout.swap(a, b);
    }

    pub fn get_char_index(&self, c: char) -> usize {
        *self
            .char_map
            .get(&c)
            .unwrap_or_else(|| panic!("Character {} not found", c))
    }

    pub fn get(&self, index: usize) -> char {
        self.layout[index]
    }

    pub fn set(&mut self, index: usize, c: char) {
        self.layout[index] = c;
        self.char_map.insert(c, index);
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
    }

    pub fn char_nums(&self) -> usize {
        self.char_map.len()
    }

    pub fn output(&self) -> [char; NUM_COLS * NUM_ROWS * NUM_LAYERS] {
        self.layout
    }

    pub fn print(&self) {
        println!();
        for layer in 0..NUM_LAYERS {
            println!("Layer {}:", layer);
            for row in 0..NUM_ROWS {
                for col in 0..NUM_COLS {
                    let idx = layer * (NUM_COLS * NUM_ROWS) + row * NUM_COLS + col;
                    let ch = self.layout[idx];
                    if col == NUM_COLS / 2 {
                        print!("| ");
                    }
                    if self.dummy_chars.contains(&ch) {
                        // Do not display dummy characters
                        print!("  ");
                    } else {
                        print!("{} ", ch);
                    }
                }
                println!();
            }
            std::iter::repeat_n("--", NUM_COLS + 1).for_each(|c| {
                print!("{}", c);
            });
            println!();
        }
    }
}
