pub const NUM_ROWS: usize = 3;
pub const NUM_COLS: usize = 10;
pub const NUM_LAYERS: usize = 2;

use std::cmp::max;
use std::collections::HashMap;

use super::hand_model::Hand;
use crate::keyboard_layout::Finger;
use crate::n_gram::PhysicalNGram;

#[derive(Debug)]
pub struct PhysicalLayout {
    cost_matrix: [f32; NUM_COLS * NUM_ROWS],
    finger_matrix: [Finger; NUM_COLS * NUM_ROWS],
    tri_gram_cost: HashMap<PhysicalNGram<3>, f32>,
}

impl PhysicalLayout {
    pub fn new(
        cost_matrix: [f32; NUM_COLS * NUM_ROWS],
        finger_matrix: [Finger; NUM_COLS * NUM_ROWS],
    ) -> Result<Self, &'static str> {
        let tri_gram_cost = HashMap::new();

        Ok(PhysicalLayout {
            cost_matrix,
            finger_matrix,
            tri_gram_cost,
        })
    }

    pub fn calculate_tri_gram_cost(&mut self) {
        let num_keys = NUM_COLS * NUM_ROWS;
        for k1 in 0..num_keys {
            for k2 in 0..num_keys {
                for k3 in 0..num_keys {
                    self.tri_gram_cost.insert(
                        PhysicalNGram::new([k1, k2, k3]),
                        self.stroke_cost(PhysicalNGram::new([k1, k2, k3])),
                    );
                }
            }
        }
    }

    fn position_cost(&self, key: usize) -> f32 {
        self.cost_matrix[key]
    }

    fn finger_cost(&self, key1: usize, key2: usize) -> f32 {
        let finger_cost = if self.finger_matrix[key1] == self.finger_matrix[key2] {
            8
        } else {
            0
        };
        let same_column = if key1 % NUM_COLS == key2 % NUM_COLS {
            8
        } else {
            0
        };
        let col_diff = max(0, ((key1 % NUM_COLS - key2 % NUM_COLS) as i32).abs() - 2);
        let row_diff = max(0, ((key1 / NUM_COLS - key2 / NUM_COLS) as i32).abs() - 1);
        (row_diff + same_column + col_diff + finger_cost) as f32
    }

    fn roll_cost(&self, keys: &[usize]) -> f32 {
        for i in 0..keys.len() - 1 {
            let finger1 = &self.finger_matrix[keys[i]];
            let finger2 = &self.finger_matrix[keys[i + 1]];
            if finger1 <= finger2 {
                return 8.0;
            }
        }
        0.0
    }

    fn stroke_cost(&self, n_gram: PhysicalNGram<3>) -> f32 {
        let key1 = n_gram.get(0);
        let key2 = n_gram.get(1);
        let key3 = n_gram.get(2);
        let first_hand = self.hand(key1);
        let pattern = (
            true,
            first_hand == self.hand(key2),
            first_hand == self.hand(key3),
        );
        let cost = match pattern {
            (true, true, true) => {
                let position_cost = self.position_cost(key1);
                let roll_cost = self.roll_cost(&[key1, key2, key3]);
                let finger_cost = self.finger_cost(key1, key2)
                    + self.finger_cost(key2, key3)
                    + self.finger_cost(key3, key1);
                position_cost * (roll_cost + finger_cost)
            }
            (true, true, false) => {
                let position_cost = self.position_cost(key1);
                let finger_cost = self.finger_cost(key1, key2);
                let roll_cost = self.roll_cost(&[key1, key2]);
                position_cost * (finger_cost + roll_cost) + self.position_cost(key3)
            }
            (true, false, true) => {
                let position_cost = self.position_cost(key1);
                let finger_cost = self.finger_cost(key1, key3);
                let roll_cost = self.roll_cost(&[key1, key3]);
                position_cost * (finger_cost + roll_cost) + self.position_cost(key2)
            }
            (true, false, false) => {
                let position_cost = self.position_cost(key2);
                let finger_cost = self.finger_cost(key2, key3);
                let roll_cost = self.roll_cost(&[key2, key3]);
                position_cost * (finger_cost + roll_cost) + self.position_cost(key1)
            }
            _ => panic!("Invalid pattern"),
        };
        (1.0 + cost).log2()
    }

    pub fn len(&self) -> usize {
        NUM_COLS * NUM_ROWS
    }

    pub fn get_tri_gram_cost(&self, n_gram: &PhysicalNGram<3>) -> f32 {
        *self
            .tri_gram_cost
            .get(n_gram)
            .expect("Failed to get tri gram cost")
    }

    fn hand(&self, index: usize) -> Hand {
        let col = index % NUM_COLS;
        if col < NUM_COLS / 2 {
            Hand::Left
        } else {
            Hand::Right
        }
    }

    pub fn print(&self, layout: &[char]) {
        println!();
        for (i, row) in layout.chunks(NUM_COLS).enumerate() {
            for (j, key) in row.iter().enumerate() {
                if j == NUM_COLS / 2 {
                    print!("| ");
                }
                print!("{} ", key);
                if (i + 1) * (j + 1) == NUM_COLS * NUM_ROWS {
                    println!();
                    std::iter::repeat_n("--", NUM_COLS + 1).for_each(|c| {
                        print!("{}", c);
                    });
                }
            }
            println!();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard_layout::hand_model::Finger as F;

    #[test]
    fn test_physical_layout() {
        let cost_matrix = [
            3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0, // 上段
            1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3,
            1.6, // 中段（ホームポジション）
            3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2, // 下段
        ];
        let finger_table = [
            F::R,
            F::R,
            F::M,
            F::M,
            F::I,
            F::I,
            F::M,
            F::M,
            F::R,
            F::R,
            F::P,
            F::R,
            F::M,
            F::I,
            F::I,
            F::I,
            F::I,
            F::M,
            F::R,
            F::P,
            F::P,
            F::R,
            F::M,
            F::I,
            F::I,
            F::I,
            F::I,
            F::M,
            F::R,
            F::P,
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix, finger_table).unwrap();
        assert_eq!(physical_layout.position_cost(0), 3.0);
        assert_eq!(physical_layout.position_cost(48), 100.0);
    }
}
