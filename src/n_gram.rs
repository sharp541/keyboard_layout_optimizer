use rusqlite::{params, Connection, Result};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::path::Path;

#[derive(Debug, Eq, Hash, PartialEq)]
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

#[derive(Debug, Eq, Hash, PartialEq)]
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

fn generate_n_grams(text: &str, n: usize) -> Vec<&str> {
    text.as_bytes()
        .windows(n)
        .map(|w| std::str::from_utf8(w).unwrap())
        .collect()
}

pub struct NGramDB {
    conn: Connection,
}

impl NGramDB {
    pub fn new<P: AsRef<Path>>(source_paths: &[P], db_path: P) -> Result<Self> {
        let mut conn = Connection::open(db_path).expect("Failed to open database");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS n_grams (
                      id INTEGER PRIMARY KEY,
                      n INTEGER NOT NULL,
                      n_gram TEXT NOT NULL,
                      count INTEGER NOT NULL
                      )",
            [],
        )
        .expect("Failed to create table");

        let tx = conn.transaction().expect("Failed to create transaction");

        let mut n_gram_counts: HashMap<String, usize> = HashMap::new();

        let n = 3;
        for source_path in source_paths {
            let text = fs::read_to_string(source_path).expect("Failed to read file");
            let n_grams = generate_n_grams(&text, n);
            for n_gram in &n_grams {
                *n_gram_counts.entry(n_gram.to_string()).or_insert(0) += 1;
            }
        }

        for (n_gram_str, count) in n_gram_counts {
            tx.execute(
                "INSERT INTO n_grams (n, n_gram, count) VALUES (?1, ?2, ?3)",
                params![n as u8, n_gram_str, count],
            )
            .expect("Failed to insert n-gram");
        }

        tx.commit().expect("Failed to commit transaction");

        Ok(NGramDB { conn })
    }

    pub fn load<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path).expect("Failed to open database");
        Ok(NGramDB { conn })
    }

    pub fn get_mono_grams(&self) -> Result<HashMap<LogicalNGram<1>, f32>> {
        let mut stmt = self
            .conn
            .prepare("SELECT n_gram, count FROM n_grams WHERE n = ?1")
            .expect("Failed to prepare statement");
        let n_grams_iter = stmt
            .query_map(params![1 as i32], |row| {
                let n_gram: String = row.get(0).expect("Failed to get n-gram");
                let count: u32 = row.get(1).expect("Failed to get count");
                Ok((
                    LogicalNGram::new(n_gram.chars().collect::<Vec<char>>().try_into().unwrap()),
                    count as f32,
                ))
            })
            .expect("Failed to get n-grams");

        let mut n_gram_map: HashMap<LogicalNGram<1>, f32> = HashMap::new();
        let mut total_count: f32 = 0.0;
        for n_gram in n_grams_iter {
            let (n_gram_str, count) = n_gram.expect("Failed to get n-gram");
            total_count += count;
            n_gram_map.insert(n_gram_str, count);
        }

        for count in n_gram_map.values_mut() {
            *count /= total_count;
        }

        Ok(n_gram_map)
    }

    pub fn get_tri_grams(&self, usable_chars: &HashSet<char>) -> Result<HashMap<LogicalNGram<3>, f32>> {
        let mut stmt = self
            .conn
            .prepare("SELECT n_gram, count FROM n_grams WHERE n = ?1")
            .expect("Failed to prepare statement");
        let n_grams_iter = stmt
            .query_map(params![3 as i32], |row| {
                let n_gram: String = row.get(0).expect("Failed to get n-gram");
                let count: u32 = row.get(1).expect("Failed to get frequency");
                Ok((
                    LogicalNGram::new(n_gram.chars().collect::<Vec<char>>().try_into().unwrap()),
                    count as f32,
                ))
            })
            .expect("Failed to get n-grams");

        let mut n_gram_map: HashMap<LogicalNGram<3>, f32> = HashMap::new();
        let mut total_count: f32 = 0.0;
        for n_gram in n_grams_iter {
            let (n_gram_str, count) = n_gram.expect("Failed to get n-gram");
            if n_gram_str.0.iter().all(|&c| usable_chars.contains(&c)) {
                total_count += count;
                n_gram_map.insert(n_gram_str, count);
            }
        }

        for count in n_gram_map.values_mut() {
            *count /= total_count;
        }

        Ok(n_gram_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_n_grams() {
        let text = "abcde";

        // 1-gram
        let mono_grams = generate_n_grams(text, 1);
        assert_eq!(mono_grams.len(), 5);
        assert_eq!(mono_grams, vec!["a", "b", "c", "d", "e"]);

        // 2-gram
        let two_grams = generate_n_grams(text, 2);
        assert_eq!(two_grams.len(), 4);
        assert_eq!(two_grams, vec!["ab", "bc", "cd", "de"]);
    }

    #[test]
    fn test_ngramdb() {
        let file_path = "test_text.txt";
        let db_path = "test_text.db";

        // テスト用のテキストファイルを作成
        fs::write(file_path, "abcabc").expect("Failed to write test file");

        // NGramDBを新規作成
        let n_gram_db = NGramDB::new(&[file_path], db_path).expect("Failed to create NGramDB");

        // 1-gramを取得して確認
        let mono_grams = n_gram_db.get_mono_grams().expect("Failed to get 1-grams");
        println!("{:?}", mono_grams);
        assert_eq!(mono_grams.len(), 3);
        assert!(mono_grams.contains_key(&LogicalNGram::new(['a'])));
        assert!(mono_grams.contains_key(&LogicalNGram::new(['b'])));
        assert!(mono_grams.contains_key(&LogicalNGram::new(['c'])));

        // 3-gramを取得して確認
        let usable_chars: HashSet<char> = ['a', 'b', 'c'].iter().cloned().collect();
        let tri_grams = n_gram_db
            .get_tri_grams(&usable_chars)
            .expect("Failed to get 3-grams");
        println!("{:?}", tri_grams);
        assert_eq!(tri_grams.len(), 3);
        assert!(tri_grams.contains_key(&LogicalNGram::new(['a', 'b', 'c'])));
        assert!(tri_grams.contains_key(&LogicalNGram::new(['b', 'c', 'a'])));
        assert!(tri_grams.contains_key(&LogicalNGram::new(['c', 'a', 'b'])));

        // NGramDBをロード
        let n_gram_db = NGramDB::load(db_path).expect("Failed to load NGramDB");

        // 1-gramを取得して確認
        let mono_grams = n_gram_db.get_mono_grams().expect("Failed to get 1-grams");
        assert_eq!(mono_grams.len(), 3);
        assert!(mono_grams.contains_key(&LogicalNGram::new(['a'])));
        assert!(mono_grams.contains_key(&LogicalNGram::new(['b'])));
        assert!(mono_grams.contains_key(&LogicalNGram::new(['c'])));

        // 3-gramを取得して確認
        let tri_grams = n_gram_db
            .get_tri_grams(&usable_chars)
            .expect("Failed to get 3-grams");
        assert_eq!(tri_grams.len(), 3);
        assert!(tri_grams.contains_key(&LogicalNGram::new(['a', 'b', 'c'])));
        assert!(tri_grams.contains_key(&LogicalNGram::new(['b', 'c', 'a'])));
        assert!(tri_grams.contains_key(&LogicalNGram::new(['c', 'a', 'b'])));

        // テスト用のファイルを削除
        fs::remove_file(file_path).expect("Failed to remove test file");
        fs::remove_file(db_path).expect("Failed to remove test database");
    }
}
