use rusqlite::{params, Connection, OptionalExtension, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

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

fn normalize_weights(ja_weight: f32, en_weight: f32) -> (f32, f32) {
    let ja = if ja_weight.is_sign_negative() {
        0.0
    } else {
        ja_weight
    };
    let en = if en_weight.is_sign_negative() {
        0.0
    } else {
        en_weight
    };
    let sum = ja + en;
    if sum <= f32::EPSILON {
        (0.5, 0.5)
    } else {
        (ja / sum, en / sum)
    }
}

fn table_exists(conn: &Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        params![table_name],
        |_| Ok(()),
    )
    .optional()
    .expect("Failed to check table existence")
    .is_some()
}

fn source_name_from_index(index: usize) -> String {
    match index {
        0 => "ja".to_string(),
        1 => "en".to_string(),
        _ => format!("source_{index}"),
    }
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
        .expect("Failed to create n_grams table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS source_stats (
                      source_id INTEGER PRIMARY KEY,
                      source_name TEXT NOT NULL UNIQUE,
                      char_count INTEGER NOT NULL,
                      tri_gram_count INTEGER NOT NULL,
                      char_ratio REAL NOT NULL
                      )",
            [],
        )
        .expect("Failed to create source_stats table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS n_grams_by_source (
                      id INTEGER PRIMARY KEY,
                      source_id INTEGER NOT NULL,
                      n INTEGER NOT NULL,
                      n_gram TEXT NOT NULL,
                      count INTEGER NOT NULL,
                      UNIQUE(source_id, n, n_gram)
                      )",
            [],
        )
        .expect("Failed to create n_grams_by_source table");

        let tx = conn.transaction().expect("Failed to create transaction");

        let mut n_gram_counts: HashMap<(u8, String), usize> = HashMap::new();
        let mut source_n_gram_counts: Vec<HashMap<(u8, String), usize>> =
            vec![HashMap::new(); source_paths.len()];
        let mut source_char_counts: Vec<usize> = vec![0; source_paths.len()];
        let mut source_tri_gram_counts: Vec<usize> = vec![0; source_paths.len()];

        for (source_id, source_path) in source_paths.iter().enumerate() {
            let text = fs::read_to_string(source_path).expect("Failed to read file");
            source_char_counts[source_id] = text.chars().count();

            for &n in &[1_usize, 3_usize] {
                let n_grams = generate_n_grams(&text, n);
                if n == 3 {
                    source_tri_gram_counts[source_id] = n_grams.len();
                }
                for n_gram in &n_grams {
                    let key = (n as u8, n_gram.to_string());
                    *n_gram_counts.entry(key.clone()).or_insert(0) += 1;
                    *source_n_gram_counts[source_id].entry(key).or_insert(0) += 1;
                }
            }
        }

        tx.execute("DELETE FROM n_grams", [])
            .expect("Failed to clear n_grams");
        tx.execute("DELETE FROM source_stats", [])
            .expect("Failed to clear source_stats");
        tx.execute("DELETE FROM n_grams_by_source", [])
            .expect("Failed to clear n_grams_by_source");

        for ((n, n_gram_str), count) in n_gram_counts {
            tx.execute(
                "INSERT INTO n_grams (n, n_gram, count) VALUES (?1, ?2, ?3)",
                params![n, n_gram_str, count],
            )
            .expect("Failed to insert n-gram");
        }

        let total_char_count: usize = source_char_counts.iter().sum();
        for source_id in 0..source_paths.len() {
            let char_ratio = if total_char_count == 0 {
                0.0
            } else {
                source_char_counts[source_id] as f32 / total_char_count as f32
            };
            let source_name = source_name_from_index(source_id);
            tx.execute(
                "INSERT INTO source_stats (source_id, source_name, char_count, tri_gram_count, char_ratio)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    source_id as i64,
                    source_name,
                    source_char_counts[source_id] as i64,
                    source_tri_gram_counts[source_id] as i64,
                    char_ratio
                ],
            )
            .expect("Failed to insert source stats");

            for ((n, n_gram_str), count) in &source_n_gram_counts[source_id] {
                tx.execute(
                    "INSERT INTO n_grams_by_source (source_id, n, n_gram, count)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![source_id as i64, *n, n_gram_str, *count as i64],
                )
                .expect("Failed to insert n-gram by source");
            }
        }

        tx.commit().expect("Failed to commit transaction");

        Ok(NGramDB { conn })
    }

    pub fn load<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path).expect("Failed to open database");
        Ok(NGramDB { conn })
    }

    pub fn get_source_size_ratios(&self) -> Result<Option<(f32, f32)>> {
        if !table_exists(&self.conn, "source_stats") {
            return Ok(None);
        }

        let ja_ratio = self
            .conn
            .query_row(
                "SELECT char_ratio FROM source_stats WHERE source_name = 'ja' LIMIT 1",
                [],
                |row| row.get::<_, f32>(0),
            )
            .optional()?;

        let en_ratio = self
            .conn
            .query_row(
                "SELECT char_ratio FROM source_stats WHERE source_name = 'en' LIMIT 1",
                [],
                |row| row.get::<_, f32>(0),
            )
            .optional()?;

        Ok(match (ja_ratio, en_ratio) {
            (Some(ja), Some(en)) => Some((ja, en)),
            _ => None,
        })
    }

    pub fn get_mono_grams(&self) -> Result<HashMap<LogicalNGram<1>, f32>> {
        let mut stmt = self
            .conn
            .prepare("SELECT n_gram, count FROM n_grams WHERE n = ?1")
            .expect("Failed to prepare statement");
        let n_grams_iter = stmt
            .query_map(params![1_i32], |row| {
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

        if total_count > f32::EPSILON {
            for count in n_gram_map.values_mut() {
                *count /= total_count;
            }
        }

        Ok(n_gram_map)
    }

    pub fn get_tri_grams(
        &self,
        usable_chars: &HashSet<char>,
    ) -> Result<HashMap<LogicalNGram<3>, f32>> {
        let mut stmt = self
            .conn
            .prepare("SELECT n_gram, count FROM n_grams WHERE n = ?1")
            .expect("Failed to prepare statement");
        let n_grams_iter = stmt
            .query_map(params![3_i32], |row| {
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

        if total_count > f32::EPSILON {
            for count in n_gram_map.values_mut() {
                *count /= total_count;
            }
        }

        Ok(n_gram_map)
    }

    pub fn get_tri_grams_weighted(
        &self,
        usable_chars: &HashSet<char>,
        ja_weight: f32,
        en_weight: f32,
    ) -> Result<HashMap<LogicalNGram<3>, f32>> {
        if !table_exists(&self.conn, "n_grams_by_source")
            || !table_exists(&self.conn, "source_stats")
        {
            return self.get_tri_grams(usable_chars);
        }

        let (ja_weight, en_weight) = normalize_weights(ja_weight, en_weight);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.source_name, n.n_gram, n.count
                 FROM n_grams_by_source n
                 JOIN source_stats s ON s.source_id = n.source_id
                 WHERE n.n = ?1",
            )
            .expect("Failed to prepare weighted n-gram query");

        let rows = stmt
            .query_map(params![3_i32], |row| {
                let source_name: String = row.get(0)?;
                let n_gram: String = row.get(1)?;
                let count: u32 = row.get(2)?;
                Ok((source_name, n_gram, count as f32))
            })
            .expect("Failed to query weighted n-grams");

        let mut ja_counts: HashMap<LogicalNGram<3>, f32> = HashMap::new();
        let mut en_counts: HashMap<LogicalNGram<3>, f32> = HashMap::new();
        let mut ja_total = 0.0_f32;
        let mut en_total = 0.0_f32;

        for row in rows {
            let (source_name, n_gram, count) = row.expect("Failed to read weighted n-gram row");
            let n_gram_chars: [char; 3] = n_gram.chars().collect::<Vec<char>>().try_into().unwrap();
            if !n_gram_chars.iter().all(|&c| usable_chars.contains(&c)) {
                continue;
            }
            let key = LogicalNGram::new(n_gram_chars);
            match source_name.as_str() {
                "ja" => {
                    ja_total += count;
                    *ja_counts.entry(key).or_insert(0.0) += count;
                }
                "en" => {
                    en_total += count;
                    *en_counts.entry(key).or_insert(0.0) += count;
                }
                _ => {}
            }
        }

        if ja_total <= f32::EPSILON || en_total <= f32::EPSILON {
            return self.get_tri_grams(usable_chars);
        }

        for count in ja_counts.values_mut() {
            *count /= ja_total;
        }
        for count in en_counts.values_mut() {
            *count /= en_total;
        }

        let mut merged: HashMap<LogicalNGram<3>, f32> = HashMap::new();
        for (n_gram, prob) in ja_counts {
            *merged.entry(n_gram).or_insert(0.0) += ja_weight * prob;
        }
        for (n_gram, prob) in en_counts {
            *merged.entry(n_gram).or_insert(0.0) += en_weight * prob;
        }

        Ok(merged)
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
        assert_eq!(mono_grams.len(), 3);
        assert!(mono_grams.contains_key(&LogicalNGram::new(['a'])));
        assert!(mono_grams.contains_key(&LogicalNGram::new(['b'])));
        assert!(mono_grams.contains_key(&LogicalNGram::new(['c'])));

        // 3-gramを取得して確認
        let usable_chars: HashSet<char> = ['a', 'b', 'c'].iter().cloned().collect();
        let tri_grams = n_gram_db
            .get_tri_grams(&usable_chars)
            .expect("Failed to get 3-grams");
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

    #[test]
    fn test_source_ratios_are_stored() {
        let ja_path = "test_ja.txt";
        let en_path = "test_en.txt";
        let db_path = "test_ja_en.db";

        fs::write(ja_path, "aa").expect("Failed to write JA test file");
        fs::write(en_path, "bbbb").expect("Failed to write EN test file");

        let n_gram_db =
            NGramDB::new(&[ja_path, en_path], db_path).expect("Failed to create NGramDB");
        let ratios = n_gram_db
            .get_source_size_ratios()
            .expect("Failed to get source ratios")
            .expect("Expected JA/EN ratios to exist");

        assert!((ratios.0 - (2.0 / 6.0)).abs() < 1e-6);
        assert!((ratios.1 - (4.0 / 6.0)).abs() < 1e-6);

        fs::remove_file(ja_path).expect("Failed to remove JA test file");
        fs::remove_file(en_path).expect("Failed to remove EN test file");
        fs::remove_file(db_path).expect("Failed to remove test database");
    }

    #[test]
    fn test_weighted_tri_grams_changes_distribution() {
        let ja_path = "test_weight_ja.txt";
        let en_path = "test_weight_en.txt";
        let db_path = "test_weight_ja_en.db";

        fs::write(ja_path, "aaaaaa").expect("Failed to write JA test file");
        fs::write(en_path, "bbbbbb").expect("Failed to write EN test file");

        let n_gram_db =
            NGramDB::new(&[ja_path, en_path], db_path).expect("Failed to create NGramDB");
        let usable_chars: HashSet<char> = ['a', 'b'].iter().cloned().collect();

        let ja_only = n_gram_db
            .get_tri_grams_weighted(&usable_chars, 1.0, 0.0)
            .expect("Failed to get JA-only tri-grams");
        let en_only = n_gram_db
            .get_tri_grams_weighted(&usable_chars, 0.0, 1.0)
            .expect("Failed to get EN-only tri-grams");

        let aaa = LogicalNGram::new(['a', 'a', 'a']);
        let bbb = LogicalNGram::new(['b', 'b', 'b']);

        assert!(ja_only.get(&aaa).copied().unwrap_or(0.0) > 0.9);
        assert!(ja_only.get(&bbb).copied().unwrap_or(0.0) < 0.1);
        assert!(en_only.get(&bbb).copied().unwrap_or(0.0) > 0.9);
        assert!(en_only.get(&aaa).copied().unwrap_or(0.0) < 0.1);

        fs::remove_file(ja_path).expect("Failed to remove JA test file");
        fs::remove_file(en_path).expect("Failed to remove EN test file");
        fs::remove_file(db_path).expect("Failed to remove test database");
    }
}
