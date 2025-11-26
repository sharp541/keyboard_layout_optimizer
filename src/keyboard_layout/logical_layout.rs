use rayon::prelude::*;
use std::collections::HashMap;

use super::physical_layout::{PhysicalLayout, NUM_COLS, NUM_LAYERS, NUM_ROWS};
use crate::n_gram::{LogicalNGram, PhysicalNGram};

#[derive(Debug, Clone, Copy)]
pub struct Key {
    layers: [Option<char>; NUM_LAYERS],
}
impl Key {
    pub fn new() -> Self {
        Key {
            layers: [None; NUM_LAYERS],
        }
    }

    pub fn set(&mut self, layer: usize, c: char) {
        self.layers[layer] = Some(c);
    }

    pub fn get(&self, layer: usize) -> Option<char> {
        self.layers[layer]
    }
}

#[derive(Debug, Clone, Copy)]
struct KeyIndex {
    idx: usize,
    layer: usize,
}

impl KeyIndex {
    pub fn new(n: usize) -> Self {
        let idx = n % (NUM_COLS * NUM_ROWS);
        let layer = n / (NUM_COLS * NUM_ROWS);
        KeyIndex { idx, layer }
    }
}

#[derive(Debug, Clone)]
pub struct LogicalLayout {
    layout: [Key; NUM_COLS * NUM_ROWS],
    char_map: HashMap<char, KeyIndex>,
}

impl LogicalLayout {
    pub fn from_usable_chars(usable_chars: Vec<char>) -> Self {
        if usable_chars.len() > NUM_COLS * NUM_ROWS * NUM_LAYERS {
            panic!("Too many usable characters: {}", usable_chars.len());
        }

        let mut layout: [Key; NUM_COLS * NUM_ROWS] = [Key::new(); NUM_COLS * NUM_ROWS];
        let mut char_map = HashMap::new();
        for (i, c) in usable_chars.into_iter().enumerate() {
            let key_index = KeyIndex::new(i);
            layout[key_index.idx].set(key_index.layer, c);
            char_map.insert(c, key_index);
        }
        LogicalLayout { layout, char_map }
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
                    self.get_char_index(n_gram.get(0)).idx,
                    self.get_char_index(n_gram.get(1)).idx,
                    self.get_char_index(n_gram.get(2)).idx,
                ]);
                *score * physical_layout.get_tri_gram_cost(&physical_n_gram)
            })
            .sum();
        cost
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        let a_key_index = KeyIndex::new(a);
        let b_key_index = KeyIndex::new(b);
        let a_key_char = self.layout[a_key_index.idx]
            .get(a_key_index.layer)
            .expect("Invalid key index");
        let b_key_char = self.layout[b_key_index.idx]
            .get(b_key_index.layer)
            .expect("Invalid key index");
        self.layout[a_key_index.idx].set(a_key_index.layer, b_key_char);
        self.layout[b_key_index.idx].set(b_key_index.layer, a_key_char);
        self.char_map.insert(a_key_char, b_key_index);
        self.char_map.insert(b_key_char, a_key_index);
    }

    pub fn get_char_index(&self, c: char) -> KeyIndex {
        *self
            .char_map
            .get(&c)
            .expect(&format!("Character {} not found", c))
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }

    pub fn char_nums(&self) -> usize {
        self.char_map.len()
    }

    pub fn output(self) -> [Key; NUM_COLS * NUM_ROWS] {
        self.layout
    }
}
