pub const NUM_ROWS: usize = 3;
pub const NUM_COLS: usize = 10;

use std::cmp::max;
use std::collections::HashMap;

use super::hand_model::Hand;
use crate::n_gram::PhysicalNGram;

#[derive(Debug)]
pub struct PhysicalLayout {
    cost_matrix: [[f32; NUM_COLS]; NUM_ROWS],
    mapping: [(usize, usize); NUM_COLS * NUM_ROWS],
    tri_gram_cost: HashMap<PhysicalNGram<3>, f32>,
}

impl PhysicalLayout {
    pub fn new(cost_matrix: [[f32; NUM_COLS]; NUM_ROWS]) -> Result<Self, &'static str> {
        let mut mapping = [(0, 0); NUM_COLS * NUM_ROWS];
        for i in 0..NUM_ROWS {
            for j in 0..NUM_COLS {
                mapping[i * NUM_COLS + j] = (i, j);
            }
        }
        let tri_gram_cost = HashMap::new();

        Ok(PhysicalLayout {
            cost_matrix,
            mapping,
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

    fn position_cost(&self, idx: usize) -> f32 {
        match self.mapping.get(idx) {
            Some((row, col)) => {
                return self.cost_matrix[*row][*col];
            }
            None => return 5.0, // 未知の文字
        };
    }

    fn relative_cost(&self, key1: usize, key2: usize) -> f32 {
        let (row1, col1) = match self.coord(key1) {
            Some(coord) => coord,
            None => return 5.0,
        };
        let (row2, col2) = match self.coord(key2) {
            Some(coord) => coord,
            None => return 5.0,
        };
        let same_column: i32 = if col1 == col2 { 4 } else { 0 };
        let col_diff = max(0, (col1 as i32 - col2 as i32).abs() - 2);
        let row_diff = max(0, (row1 as i32 - row2 as i32).abs() - 1);
        (row_diff + same_column + col_diff).abs() as f32
    }

    fn roll_cost(&self, key1: usize, key2: usize, key3: usize) -> f32 {
        let (row1, col1) = match self.coord(key1) {
            Some(coord) => coord,
            None => return 5.0,
        };
        let (row2, col2) = match self.coord(key2) {
            Some(coord) => coord,
            None => return 5.0,
        };
        let (row3, col3) = match self.coord(key3) {
            Some(coord) => coord,
            None => return 5.0,
        };

        let same_column: i32 = if col1 == col2 && col2 == col3 { 8 } else { 0 };
        let not_roll_penalty = if (col1 <= col2 && col2 <= col3) && (col1 >= col2 && col2 >= col3) { 0 } else { 8 };
        let row_diff = max(0, (row1 as i32 - row2 as i32).abs() - 1) +
            max(0, (row2 as i32 - row3 as i32).abs() - 1);

        (same_column + not_roll_penalty + row_diff) as f32
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
                let roll_cost = self.roll_cost(key1, key2, key3);
                position_cost * roll_cost
            }
            (true, true, false) => {
                let position_cost = self.position_cost(key1);
                let relative_cost = self.relative_cost(key1, key2);
                position_cost * relative_cost + self.position_cost(key3)
            }
            (true, false, true) => {
                let position_cost = self.position_cost(key1);
                let relative_cost = self.relative_cost(key1, key3);
                position_cost * relative_cost + self.position_cost(key2)
            }
            (true, false, false) => {
                let position_cost = self.position_cost(key2);
                let relative_cost = self.relative_cost(key2, key3);
                position_cost * relative_cost + self.position_cost(key1)
            }
            _ => panic!("Invalid pattern"),
        };
        (1.0 + cost).log2()
    }

    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    pub fn get_tri_gram_cost(&self, n_gram: &PhysicalNGram<3>) -> f32 {
        *self
            .tri_gram_cost
            .get(n_gram)
            .expect("Failed to get tri gram cost")
    }

    fn coord(&self, index: usize) -> Option<(usize, usize)> {
        match self.mapping.get(index) {
            Some(coord) => Some(*coord),
            None => None,
        }
    }

    fn hand(&self, index: usize) -> Hand {
        match self.coord(index) {
            Some((_, col)) => {
                if col < NUM_COLS / 2 {
                    Hand::Left
                } else {
                    Hand::Right
                }
            }
            None => Hand::Other,
        }
    }

    pub fn print(&self, layout: &Vec<char>) {
        println!();
        for (i, row) in layout.chunks(self.cost_matrix[0].len()).enumerate() {
            for (j, key) in row.iter().enumerate() {
                if j == NUM_COLS / 2 {
                    print!("| ");
                }
                print!("{} ", key);
                if (i + 1) * (j + 1) == self.cost_matrix[0].len() * self.cost_matrix.len() {
                    println!();
                    std::iter::repeat("--")
                        .take(self.cost_matrix[0].len() + 1)
                        .for_each(|c| {
                            print!("{}", c);
                        });
                }
            }
            println!();
        }
    }
}

fn get_left_grams() -> Vec<PhysicalNGram<3>> {
    let mut keys = Vec::new();
    for i in 0..NUM_ROWS {
        for j in 0..NUM_COLS / 2 {
            keys.push(i * NUM_COLS + j);
        }
    }
    let mut grams = Vec::new();
    for k1 in keys.iter() {
        for k2 in keys.iter() {
            for k3 in keys.iter() {
                grams.push(PhysicalNGram::new([*k1, *k2, *k3]));
            }
        }
    }
    grams
}

fn get_right_grams() -> Vec<PhysicalNGram<3>> {
    let mut keys = Vec::new();
    for i in 0..NUM_ROWS {
        for j in NUM_COLS / 2..NUM_COLS {
            keys.push(i * NUM_COLS + j);
        }
    }
    let mut grams = Vec::new();
    for k1 in keys.iter() {
        for k2 in keys.iter() {
            for k3 in keys.iter() {
                grams.push(PhysicalNGram::new([*k1, *k2, *k3]));
            }
        }
    }
    grams
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

    #[test]
    fn test_physical_layout() {
        let cost_matrix: [[f32; NUM_COLS]; NUM_ROWS] = [
            [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0], // 上段
            [1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6], // 中段（ホームポジション）
            [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2], // 下段
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix).unwrap();
        assert_eq!(physical_layout.position_cost(0), 3.0);
        assert_eq!(physical_layout.position_cost(48), 100.0);
    }
}
