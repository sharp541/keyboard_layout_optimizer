use std::fmt::Display;

#[derive(Debug)]
pub struct PhysicalNGram<const N: usize>([usize; N]);

impl<const N: usize> PhysicalNGram<N> {
    pub fn new(n_gram: [usize; N]) -> Self {
        PhysicalNGram(n_gram)
    }

    pub fn get(&self, index: usize) -> usize {
        self.0[index]
    }

    pub fn set(&mut self, index: usize, value: usize) {
        self.0[index] = value;
    }
}

impl<const N: usize> Display for PhysicalNGram<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
pub struct LogicalNGram<const N: usize>([char; N]);

impl<const N: usize> LogicalNGram<N> {
    pub fn new(n_gram: [char; N]) -> Self {
        LogicalNGram(n_gram)
    }

    pub fn get(&self, index: usize) -> char {
        self.0[index]
    }

    pub fn set(&mut self, index: usize, value: char) {
        self.0[index] = value;
    }
}

impl<const N: usize> Display for LogicalNGram<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

pub fn generate_n_grams<const N: usize>(text: &str) -> Vec<LogicalNGram<N>> {
    let mut n_grams = Vec::new();
    for i in 0..text.len() - N + 1 {
        let n_gram: [char; N] = text[i..i + N]
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .expect("Failed to convert logical n-gram");
        n_grams.push(LogicalNGram::new(n_gram));
    }
    n_grams
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physical_n_gram() {
        let n_gram = PhysicalNGram::new([0, 1, 2]);
        assert_eq!(n_gram.get(0), 0);
        assert_eq!(n_gram.get(1), 1);
        assert_eq!(n_gram.get(2), 2);

        let mut n_gram = PhysicalNGram::new([0, 1, 2]);
        n_gram.set(0, 2);
        n_gram.set(1, 1);
        n_gram.set(2, 0);
        assert_eq!(n_gram.get(0), 2);
        assert_eq!(n_gram.get(1), 1);
        assert_eq!(n_gram.get(2), 0);
    }

    #[test]
    fn test_logical_n_gram() {
        let n_gram = LogicalNGram::new(['a', 'b', 'c']);
        assert_eq!(n_gram.get(0), 'a');
        assert_eq!(n_gram.get(1), 'b');
        assert_eq!(n_gram.get(2), 'c');

        let mut n_gram = LogicalNGram::new(['a', 'b', 'c']);
        n_gram.set(0, 'c');
        n_gram.set(1, 'b');
        n_gram.set(2, 'a');
        assert_eq!(n_gram.get(0), 'c');
        assert_eq!(n_gram.get(1), 'b');
        assert_eq!(n_gram.get(2), 'a');
    }

    #[test]
    fn test_generate_n_grams() {
        let n_grams = generate_n_grams::<3>("abcde");
        assert_eq!(n_grams.len(), 3);
        assert_eq!(n_grams[0].get(0), 'a');
        assert_eq!(n_grams[0].get(1), 'b');
        assert_eq!(n_grams[0].get(2), 'c');
        assert_eq!(n_grams[1].get(0), 'b');
        assert_eq!(n_grams[1].get(1), 'c');
        assert_eq!(n_grams[1].get(2), 'd');
        assert_eq!(n_grams[2].get(0), 'c');
        assert_eq!(n_grams[2].get(1), 'd');
        assert_eq!(n_grams[2].get(2), 'e');
    }
}
