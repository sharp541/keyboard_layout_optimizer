pub const NUM_ROWS: usize = 3;
pub const NUM_COLS: usize = 10;
pub const NUM_LAYERS: usize = 2;
const TOTAL_KEYS: usize = NUM_ROWS * NUM_COLS * NUM_LAYERS;

use std::cmp::max;

use super::hand_model::Hand;
use crate::keyboard_layout::Finger;

#[derive(Debug)]
struct KeyLocation {
    row: usize,
    col: usize,
    index: usize,
    layer: usize,
}

impl KeyLocation {
    pub fn new(index: usize) -> Self {
        let layer = index / (NUM_COLS * NUM_ROWS);
        let row = (index % (NUM_COLS * NUM_ROWS)) / NUM_COLS;
        let col = index % NUM_COLS;
        let index = index % (NUM_COLS * NUM_ROWS);
        KeyLocation { row, col, layer, index }
    }

    pub fn hand(&self) -> Hand {
        if self.col < NUM_COLS / 2 {
            Hand::Left
        } else {
            Hand::Right
        }
    }
}

#[derive(Debug)]
pub struct PhysicalLayout {
    cost_matrix: [f32; NUM_COLS * NUM_ROWS],
    finger_matrix: [Finger; NUM_COLS * NUM_ROWS],
    tri_gram_cost: [f32; TOTAL_KEYS * TOTAL_KEYS * TOTAL_KEYS],
}

impl PhysicalLayout {
    pub fn new(
        cost_matrix: [f32; NUM_COLS * NUM_ROWS],
        finger_matrix: [Finger; NUM_COLS * NUM_ROWS],
    ) -> Result<Self, &'static str> {
        let tri_gram_cost = [0.0; TOTAL_KEYS * TOTAL_KEYS * TOTAL_KEYS];

        Ok(PhysicalLayout {
            cost_matrix,
            finger_matrix,
            tri_gram_cost,
        })
    }

    pub fn calculate_tri_gram_cost(&mut self) {
        let num_keys = NUM_COLS * NUM_ROWS * NUM_LAYERS;
        for k1 in 0..num_keys {
            for k2 in 0..num_keys {
                for k3 in 0..num_keys {
                    let cost = self.stroke_cost(k1, k2, k3);
                    let index = self.cost_index(k1, k2, k3);
                    self.tri_gram_cost[index] = cost;
                }
            }
        }
    }

    fn position_cost(&self, key: &KeyLocation) -> f32 {
        let base = self.cost_matrix[key.index];
        if key.layer == 0 {
            base
        } else {
            base + 8.0
        }
    }

    fn finger(&self, key: usize) -> &Finger {
        &self.finger_matrix[key]
    }

    fn finger_cost(&self, key1: &KeyLocation, key2: &KeyLocation) -> f32 {
        let finger_cost = if self.finger(key1.index) == self.finger(key2.index) {
            8
        } else {
            0
        };
        let same_column = if key1.col == key2.col {
            8
        } else {
            0
        };
        let col_diff = max(0, (key1.col as i32 - key2.col as i32).abs() - 2);
        let row_diff = max(0, (key1.row as i32 - key2.row as i32).abs() - 1);
        (row_diff + same_column + col_diff + finger_cost) as f32
    }

    fn roll_cost(&self, keys: &[KeyLocation]) -> f32 {
        let mut ret = 0.0;
        for i in 0..keys.len() - 1 {
            let finger1 = &self.finger_matrix[keys[i].index];
            let finger2 = &self.finger_matrix[keys[i + 1].index];
            if finger1 <= finger2 {
                ret += 8.0;
            }
            if finger1.same(&Finger::P) {
                ret += 8.0;
            }
            if keys[i].layer != keys[i + 1].layer {
                ret += 8.0;
            }
        }
        ret
    }

    fn stroke_cost(&self, key1: usize, key2: usize, key3: usize) -> f32 {
        let kl1 = KeyLocation::new(key1);
        let kl2 = KeyLocation::new(key2);
        let kl3 = KeyLocation::new(key3);
        let first_hand = kl1.hand();
        let pattern = (
            true,
            first_hand == kl2.hand(),
            first_hand == kl3.hand(),
        );
        let cost = match pattern {
            (true, true, true) => {
                let position_cost = self.position_cost(&kl1);
                let finger_cost = self.finger_cost(&kl1, &kl2)
                    + self.finger_cost(&kl2, &kl3)
                    + self.finger_cost(&kl3, &kl1);
                let roll_cost = self.roll_cost(&[kl1, kl2, kl3]);
                position_cost * (roll_cost + finger_cost)
            }
            (true, true, false) => {
                let position_cost = self.position_cost(&kl1);
                let finger_cost = self.finger_cost(&kl1, &kl2);
                let roll_cost = self.roll_cost(&[kl1, kl2]);
                position_cost * (finger_cost + roll_cost) + self.position_cost(&kl3)
            }
            (true, false, true) => {
                let position_cost = self.position_cost(&kl1);
                let finger_cost = self.finger_cost(&kl1, &kl3);
                let roll_cost = self.roll_cost(&[kl1, kl3]);
                position_cost * (finger_cost + roll_cost) + self.position_cost(&kl2)
            }
            (true, false, false) => {
                let position_cost = self.position_cost(&kl2);
                let finger_cost = self.finger_cost(&kl2, &kl3);
                let roll_cost = self.roll_cost(&[kl2, kl3]);
                position_cost * (finger_cost + roll_cost) + self.position_cost(&kl1)
            }
            _ => panic!("Invalid pattern"),
        };
        (1.0 + cost).log2()
    }

    pub fn len(&self) -> usize {
        NUM_COLS * NUM_ROWS
    }

    fn cost_index(&self, k1: usize, k2: usize, k3: usize) -> usize {
        k1 * TOTAL_KEYS * TOTAL_KEYS + k2 * TOTAL_KEYS + k3
    }

    pub fn get_tri_gram_cost(&self, k1: usize, k2: usize, k3: usize) -> f32 {
        let index = self.cost_index(k1, k2, k3);
        self.tri_gram_cost[index]
    }
}

pub fn get_left_keys() -> Vec<usize> {
    let mut keys = Vec::new();
    for i in 0..NUM_ROWS {
        for j in 0..NUM_COLS / 2 {
            keys.push(i * NUM_COLS + j);
        }
    }
    keys
}

pub fn get_right_keys() -> Vec<usize> {
    let mut keys = Vec::new();
    for i in 0..NUM_ROWS {
        for j in NUM_COLS / 2..NUM_COLS {
            keys.push(i * NUM_COLS + j);
        }
    }
    keys
}
