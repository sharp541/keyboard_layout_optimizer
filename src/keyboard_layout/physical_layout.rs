use crate::n_gram::PhysicalNGram;

#[derive(Debug)]
pub struct PhysicalLayout {
    cost_matrix: Vec<Vec<f32>>,
    mapping: Vec<(usize, usize)>,
}

impl PhysicalLayout {
    pub fn new(cost_matrix: Vec<Vec<f32>>) -> Result<Self, &'static str> {
        if cost_matrix.is_empty() {
            return Err("Matrix cannot be empty");
        }

        let row_length = cost_matrix[0].len();
        for row in &cost_matrix {
            if row.len() != row_length {
                return Err("All rows must have the same length");
            }
        }

        let mut mapping = Vec::new();
        for (i, row) in cost_matrix.iter().enumerate() {
            for (j, _) in row.iter().enumerate() {
                mapping.push((i, j));
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
                match key {
                    Some(c) => print!("{} ", c),
                    None => print!("  "),
                }
                if (i + 1) * (j + 1) == self.cost_matrix[0].len() * self.cost_matrix.len() {
                    println!("\n-------------------");
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
        let cost_matrix = vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.0, 3.0],
            vec![2.0, 3.0, 0.0],
        ];
        let physical_layout = PhysicalLayout::new(cost_matrix).unwrap();
        assert_eq!(physical_layout.cost(PhysicalNGram::new([0, 1, 2])), 3.0);
        assert_eq!(physical_layout.cost(PhysicalNGram::new([3, 4, 5])), 4.0);
        assert_eq!(physical_layout.cost(PhysicalNGram::new([6, 7, 8])), 5.0);
        assert_eq!(physical_layout.cost(PhysicalNGram::new([9, 10, 11])), 30.0);
    }
}
