pub const NUM_ROWS: usize = 3;
pub const NUM_COLS: usize = 10;

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

    pub fn cost<const N: usize>(&self, n_gram: PhysicalNGram<N>) -> f32 {
        let mut cost = 0.0;
        for i in 0..N {
            let key = &n_gram.get(i);
            match self.mapping.get(*key) {
                Some((row, col)) => {
                    cost += self.cost_matrix[*row][*col];
                }
                None => cost += 10.0, // 未知の文字
            };
        }
        cost
    }

    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    pub fn coord(&self, index: usize) -> (usize, usize) {
        self.mapping[index]
    }

    pub fn print(&self, layout: Vec<Option<char>>) {
        for (i, row) in layout.chunks(self.cost_matrix[0].len()).enumerate() {
            for (j, key) in row.iter().enumerate() {
                if j == NUM_COLS / 2 {
                    print!("| ");
                }
                match key {
                    Some(c) => print!("{} ", c),
                    None => print!("  "),
                }
                if (i + 1) * (j + 1) == self.cost_matrix[0].len() * self.cost_matrix.len() {
                    println!("\n");
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
        assert_eq!(physical_layout.cost(PhysicalNGram::new([0, 1, 2])), 7.4);
        assert_eq!(physical_layout.cost(PhysicalNGram::new([48, 49, 50])), 30.0);
    }
}
