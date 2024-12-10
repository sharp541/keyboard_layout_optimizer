pub const NUM_ROWS: usize = 3;
pub const NUM_COLS: usize = 10;

use std::cmp::max;

use super::hand_model::Hand;
use crate::n_gram::PhysicalNGram;

#[derive(Debug)]
pub struct PhysicalLayout {
    cost_matrix: [[f32; NUM_COLS]; NUM_ROWS],
    mapping: [(usize, usize); NUM_COLS * NUM_ROWS],
}

impl PhysicalLayout {
    pub fn new(cost_matrix: [[f32; NUM_COLS]; NUM_ROWS]) -> Result<Self, &'static str> {
        let mut mapping = [(0, 0); NUM_COLS * NUM_ROWS];
        for i in 0..NUM_ROWS {
            for j in 0..NUM_COLS {
                mapping[i * NUM_COLS + j] = (i, j);
            }
        }

        Ok(PhysicalLayout {
            cost_matrix,
            mapping,
        })
    }

    fn position_cost(&self, idx: usize) -> f32 {
        match self.mapping.get(idx) {
            Some((row, col)) => {
                return self.cost_matrix[*row][*col];
            }
            None => return 10.0, // 未知の文字
        };
    }

    fn row_cost(&self, key1: usize, key2: usize) -> f32 {
        let (row1, col1) = match self.coord(key1) {
            Some(coord) => coord,
            None => return 10.0,
        };
        let (row2, col2) = match self.coord(key2) {
            Some(coord) => coord,
            None => return 10.0,
        };
        let same_column: i32 = if col1 == col2 { 2 } else { 0 };
        let col_diff = max(0, (col1 as i32 - col2 as i32).abs() - 2);
        let row_diff = max(0, (row1 as i32 - row2 as i32).abs() - 1);
        (row_diff + same_column + col_diff).abs() as f32
    }

    pub fn stroke_cost(&self, n_gram: PhysicalNGram<3>) -> f32 {
        let mut stroke_cost = 0.0;
        let position_cost = self.position_cost(n_gram.get(0));
        for i in 0..2 {
            let key1 = n_gram.get(i);
            let key2 = n_gram.get(i + 1);
            let hand1 = self.hand(key1);
            let hand2 = self.hand(key2);
            if hand1.same(hand2) {
                stroke_cost += self.row_cost(key1, key2);
            } else {
                stroke_cost += 1.0;
            }
        }
        position_cost.log2() + (1.0 + stroke_cost).log2()
    }

    pub fn len(&self) -> usize {
        self.mapping.len()
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
