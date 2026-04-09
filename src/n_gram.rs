use rusqlite::{params, Connection, OptionalExtension, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::japanese_preprocessor::preprocess_japanese_romanization;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Japanese,
    English,
    Other,
}

impl SourceKind {
    fn as_db_value(self) -> &'static str {
        match self {
            SourceKind::Japanese => "ja",
            SourceKind::English => "en",
            SourceKind::Other => "other",
        }
    }
}

#[derive(Clone, Debug)]
pub struct NGramSource<P> {
    pub name: String,
    pub kind: SourceKind,
    pub path: P,
}

impl<P> NGramSource<P> {
    pub fn new(name: impl Into<String>, kind: SourceKind, path: P) -> Self {
        Self {
            name: name.into(),
            kind,
            path,
        }
    }
}

fn generate_n_grams(text: &str, n: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(n)
        .map(|window| window.iter().collect())
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

fn table_has_column(conn: &Connection, table_name: &str, column_name: &str) -> bool {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = conn
        .prepare(&pragma)
        .expect("Failed to inspect table schema");
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("Failed to query table schema");

    let mut column_names = Vec::new();
    for column in columns {
        column_names.push(column.expect("Failed to read column name"));
    }

    column_names.into_iter().any(|column| column == column_name)
}

fn preprocess_source_text(source_kind: SourceKind, text: &str) -> String {
    match source_kind {
        SourceKind::Japanese => preprocess_japanese_romanization(text),
        _ => text.to_string(),
    }
}

pub struct NGramDB {
    conn: Connection,
}

impl NGramDB {
    fn get_tri_grams_with_filter<F>(
        &self,
        mut is_usable: F,
    ) -> Result<HashMap<LogicalNGram<3>, f32>>
    where
        F: FnMut(char) -> bool,
    {
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
            if n_gram_str.0.iter().all(|&c| is_usable(c)) {
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

    fn get_tri_grams_weighted_with_filter<F>(
        &self,
        mut is_usable: F,
        ja_weight: f32,
        en_weight: f32,
    ) -> Result<HashMap<LogicalNGram<3>, f32>>
    where
        F: FnMut(char) -> bool,
    {
        if !table_exists(&self.conn, "n_grams_by_source")
            || !table_exists(&self.conn, "source_stats")
        {
            return self.get_tri_grams_with_filter(is_usable);
        }

        let (ja_weight, en_weight) = normalize_weights(ja_weight, en_weight);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.source_kind, n.n_gram, n.count
                 FROM n_grams_by_source n
                 JOIN source_stats s ON s.source_id = n.source_id
                 WHERE n.n = ?1",
            )
            .expect("Failed to prepare weighted n-gram query");

        let rows = stmt
            .query_map(params![3_i32], |row| {
                let source_kind: String = row.get(0)?;
                let n_gram: String = row.get(1)?;
                let count: u32 = row.get(2)?;
                Ok((source_kind, n_gram, count as f32))
            })
            .expect("Failed to query weighted n-grams");

        let mut ja_counts: HashMap<LogicalNGram<3>, f32> = HashMap::new();
        let mut en_counts: HashMap<LogicalNGram<3>, f32> = HashMap::new();
        let mut ja_total = 0.0_f32;
        let mut en_total = 0.0_f32;

        for row in rows {
            let (source_kind, n_gram, count) = row.expect("Failed to read weighted n-gram row");
            let n_gram_chars: [char; 3] = n_gram.chars().collect::<Vec<char>>().try_into().unwrap();
            if !n_gram_chars.iter().all(|&c| is_usable(c)) {
                continue;
            }
            let key = LogicalNGram::new(n_gram_chars);
            match source_kind.as_str() {
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
            return self.get_tri_grams_with_filter(is_usable);
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

    pub fn requires_rebuild<P: AsRef<Path>>(db_path: P) -> Result<bool> {
        if !db_path.as_ref().exists() {
            return Ok(true);
        }

        let conn = Connection::open(db_path)?;
        Ok(!Self::has_compatible_schema(&conn))
    }

    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(
        sources: &[NGramSource<P>],
        db_path: Q,
    ) -> Result<Self> {
        let mut conn = Connection::open(db_path).expect("Failed to open database");

        if table_exists(&conn, "source_stats")
            && !table_has_column(&conn, "source_stats", "source_kind")
        {
            conn.execute("DROP TABLE source_stats", [])
                .expect("Failed to drop legacy source_stats table");
        }
        if table_exists(&conn, "n_grams_by_source")
            && !table_has_column(&conn, "n_grams_by_source", "source_id")
        {
            conn.execute("DROP TABLE n_grams_by_source", [])
                .expect("Failed to drop legacy n_grams_by_source table");
        }

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
                      source_kind TEXT NOT NULL,
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
            vec![HashMap::new(); sources.len()];
        let mut source_char_counts: Vec<usize> = vec![0; sources.len()];
        let mut source_tri_gram_counts: Vec<usize> = vec![0; sources.len()];

        for (source_id, source) in sources.iter().enumerate() {
            let raw_text = fs::read_to_string(source.path.as_ref()).expect("Failed to read file");
            let text = preprocess_source_text(source.kind, &raw_text);
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
        for (source_id, source) in sources.iter().enumerate() {
            let char_ratio = if total_char_count == 0 {
                0.0
            } else {
                source_char_counts[source_id] as f32 / total_char_count as f32
            };
            tx.execute(
                "INSERT INTO source_stats (source_id, source_name, source_kind, char_count, tri_gram_count, char_ratio)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    source_id as i64,
                    &source.name,
                    source.kind.as_db_value(),
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

    fn has_compatible_schema(conn: &Connection) -> bool {
        table_exists(conn, "n_grams")
            && table_exists(conn, "source_stats")
            && table_exists(conn, "n_grams_by_source")
            && table_has_column(conn, "source_stats", "source_kind")
            && table_has_column(conn, "n_grams_by_source", "source_id")
    }

    pub fn get_source_size_ratios(&self) -> Result<Option<(f32, f32)>> {
        if !table_exists(&self.conn, "source_stats") {
            return Ok(None);
        }

        let ja_ratio = self.conn.query_row(
            "SELECT SUM(char_ratio) FROM source_stats WHERE source_kind = 'ja'",
            [],
            |row| row.get::<_, Option<f32>>(0),
        )?;

        let en_ratio = self.conn.query_row(
            "SELECT SUM(char_ratio) FROM source_stats WHERE source_kind = 'en'",
            [],
            |row| row.get::<_, Option<f32>>(0),
        )?;

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
        self.get_tri_grams_with_filter(|c| usable_chars.contains(&c))
    }

    pub fn get_tri_grams_for_layout<F>(
        &self,
        can_resolve_char: F,
    ) -> Result<HashMap<LogicalNGram<3>, f32>>
    where
        F: FnMut(char) -> bool,
    {
        self.get_tri_grams_with_filter(can_resolve_char)
    }

    pub fn get_tri_grams_weighted(
        &self,
        usable_chars: &HashSet<char>,
        ja_weight: f32,
        en_weight: f32,
    ) -> Result<HashMap<LogicalNGram<3>, f32>> {
        self.get_tri_grams_weighted_with_filter(|c| usable_chars.contains(&c), ja_weight, en_weight)
    }

    pub fn get_tri_grams_weighted_for_layout<F>(
        &self,
        can_resolve_char: F,
        ja_weight: f32,
        en_weight: f32,
    ) -> Result<HashMap<LogicalNGram<3>, f32>>
    where
        F: FnMut(char) -> bool,
    {
        self.get_tri_grams_weighted_with_filter(can_resolve_char, ja_weight, en_weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::azik_extension::AzikExtensionToken;
    use crate::keyboard_layout::{Finger, LogicalLayout, PhysicalLayout, NUM_COLS, NUM_ROWS};
    use std::fs;

    fn test_source<'a>(name: &'a str, kind: SourceKind, path: &'a str) -> NGramSource<&'a str> {
        NGramSource::new(name, kind, path)
    }

    fn count_ngrams_by_source(n_gram_db: &NGramDB, source_name: &str, n: u8, n_gram: &str) -> i64 {
        n_gram_db
            .conn
            .query_row(
                "SELECT count FROM n_grams_by_source
                 WHERE source_id = (
                    SELECT source_id FROM source_stats WHERE source_name = ?1
                 )
                 AND n = ?2 AND n_gram = ?3",
                params![source_name, n, n_gram],
                |row| row.get(0),
            )
            .expect("Failed to get n-gram count by source name")
    }

    fn count_matching_rows_by_source(n_gram_db: &NGramDB, source_name: &str, n_gram: &str) -> i64 {
        n_gram_db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM n_grams_by_source
                 WHERE source_id = (
                    SELECT source_id FROM source_stats WHERE source_name = ?1
                 )
                 AND n_gram = ?2",
                params![source_name, n_gram],
                |row| row.get(0),
            )
            .expect("Failed to count n-gram rows by source name")
    }

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
        let n_gram_db = NGramDB::new(
            &[test_source("test_text", SourceKind::Other, file_path)],
            db_path,
        )
        .expect("Failed to create NGramDB");

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

        let n_gram_db = NGramDB::new(
            &[
                test_source("ja", SourceKind::Japanese, ja_path),
                test_source("en", SourceKind::English, en_path),
            ],
            db_path,
        )
        .expect("Failed to create NGramDB");
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

        let n_gram_db = NGramDB::new(
            &[
                test_source("ja", SourceKind::Japanese, ja_path),
                test_source("en", SourceKind::English, en_path),
            ],
            db_path,
        )
        .expect("Failed to create NGramDB");
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

    #[test]
    fn test_source_specific_preprocessing_is_applied_only_to_japanese() {
        let ja_path = "test_preprocess_ja.txt";
        let en_path = "test_preprocess_en.txt";
        let db_path = "test_preprocess_ja_en.db";

        fs::write(ja_path, "kannzi").expect("Failed to write JA test file");
        fs::write(en_path, "kannzi").expect("Failed to write EN test file");

        let n_gram_db = NGramDB::new(
            &[
                test_source("japanese_corpus", SourceKind::Japanese, ja_path),
                test_source("english_corpus", SourceKind::English, en_path),
            ],
            db_path,
        )
        .expect("Failed to create NGramDB");

        let ja_mono_count = count_ngrams_by_source(
            &n_gram_db,
            "japanese_corpus",
            1,
            &AzikExtensionToken::Ann.as_char().to_string(),
        );
        assert_eq!(ja_mono_count, 1);

        let ja_annzi_count = count_ngrams_by_source(
            &n_gram_db,
            "japanese_corpus",
            3,
            &format!("{}zi", AzikExtensionToken::Ann.as_char()),
        );
        assert_eq!(ja_annzi_count, 1);

        let en_extension_rows = count_matching_rows_by_source(
            &n_gram_db,
            "english_corpus",
            &AzikExtensionToken::Ann.as_char().to_string(),
        );
        assert_eq!(en_extension_rows, 0);

        let en_annzi_rows = count_matching_rows_by_source(
            &n_gram_db,
            "english_corpus",
            &format!("{}zi", AzikExtensionToken::Ann.as_char()),
        );
        assert_eq!(en_annzi_rows, 0);

        let en_kan_count = count_ngrams_by_source(&n_gram_db, "english_corpus", 3, "kan");
        assert_eq!(en_kan_count, 1);

        fs::remove_file(ja_path).expect("Failed to remove JA test file");
        fs::remove_file(en_path).expect("Failed to remove EN test file");
        fs::remove_file(db_path).expect("Failed to remove test database");
    }

    #[test]
    fn test_source_specific_preprocessing_uses_source_identity_not_input_order() {
        let en_path = "test_preprocess_order_en.txt";
        let ja_path = "test_preprocess_order_ja.txt";
        let db_path = "test_preprocess_order_ja_en.db";

        fs::write(ja_path, "kannzi").expect("Failed to write JA test file");
        fs::write(en_path, "kannzi").expect("Failed to write EN test file");

        let n_gram_db = NGramDB::new(
            &[
                test_source("english_corpus", SourceKind::English, en_path),
                test_source("japanese_corpus", SourceKind::Japanese, ja_path),
            ],
            db_path,
        )
        .expect("Failed to create NGramDB");

        let ja_mono_count = count_ngrams_by_source(
            &n_gram_db,
            "japanese_corpus",
            1,
            &AzikExtensionToken::Ann.as_char().to_string(),
        );
        assert_eq!(ja_mono_count, 1);

        let ja_annzi_count = count_ngrams_by_source(
            &n_gram_db,
            "japanese_corpus",
            3,
            &format!("{}zi", AzikExtensionToken::Ann.as_char()),
        );
        assert_eq!(ja_annzi_count, 1);

        let en_extension_rows = count_matching_rows_by_source(
            &n_gram_db,
            "english_corpus",
            &AzikExtensionToken::Ann.as_char().to_string(),
        );
        assert_eq!(en_extension_rows, 0);

        let en_kan_count = count_ngrams_by_source(&n_gram_db, "english_corpus", 3, "kan");
        assert_eq!(en_kan_count, 1);

        fs::remove_file(ja_path).expect("Failed to remove JA test file");
        fs::remove_file(en_path).expect("Failed to remove EN test file");
        fs::remove_file(db_path).expect("Failed to remove test database");
    }

    #[test]
    fn test_multiple_sources_can_share_same_kind() {
        let ja_news_path = "test_multi_ja_news.txt";
        let ja_books_path = "test_multi_ja_books.txt";
        let en_path = "test_multi_en.txt";
        let db_path = "test_multi_kind.db";

        fs::write(ja_news_path, "kannzi").expect("Failed to write JA news file");
        fs::write(ja_books_path, "kannzi").expect("Failed to write JA books file");
        fs::write(en_path, "abc").expect("Failed to write EN test file");

        let n_gram_db = NGramDB::new(
            &[
                test_source("ja_news", SourceKind::Japanese, ja_news_path),
                test_source("ja_books", SourceKind::Japanese, ja_books_path),
                test_source("en_reference", SourceKind::English, en_path),
            ],
            db_path,
        )
        .expect("Failed to create NGramDB");

        let ann = AzikExtensionToken::Ann.as_char().to_string();
        assert_eq!(count_ngrams_by_source(&n_gram_db, "ja_news", 1, &ann), 1);
        assert_eq!(count_ngrams_by_source(&n_gram_db, "ja_books", 1, &ann), 1);

        let ratios = n_gram_db
            .get_source_size_ratios()
            .expect("Failed to get source ratios")
            .expect("Expected JA/EN ratios to exist");
        assert!(ratios.0 > ratios.1);

        fs::remove_file(ja_news_path).expect("Failed to remove JA news file");
        fs::remove_file(ja_books_path).expect("Failed to remove JA books file");
        fs::remove_file(en_path).expect("Failed to remove EN test file");
        fs::remove_file(db_path).expect("Failed to remove test database");
    }

    #[test]
    fn test_requires_rebuild_detects_legacy_source_stats_schema() {
        let db_path = "test_legacy_schema.db";
        let conn = Connection::open(db_path).expect("Failed to open legacy test database");

        conn.execute(
            "CREATE TABLE source_stats (
                source_id INTEGER PRIMARY KEY,
                source_name TEXT NOT NULL UNIQUE,
                char_count INTEGER NOT NULL,
                tri_gram_count INTEGER NOT NULL,
                char_ratio REAL NOT NULL
            )",
            [],
        )
        .expect("Failed to create legacy source_stats table");
        conn.execute(
            "CREATE TABLE n_grams (
                id INTEGER PRIMARY KEY,
                n INTEGER NOT NULL,
                n_gram TEXT NOT NULL,
                count INTEGER NOT NULL
            )",
            [],
        )
        .expect("Failed to create n_grams table");
        conn.execute(
            "CREATE TABLE n_grams_by_source (
                id INTEGER PRIMARY KEY,
                n INTEGER NOT NULL,
                n_gram TEXT NOT NULL,
                count INTEGER NOT NULL
            )",
            [],
        )
        .expect("Failed to create legacy n_grams_by_source table");
        drop(conn);

        assert!(
            NGramDB::requires_rebuild(db_path).expect("Failed to inspect DB schema"),
            "legacy schema should trigger rebuild"
        );

        fs::remove_file(db_path).expect("Failed to remove legacy test database");
    }

    #[test]
    fn test_layout_filter_keeps_extension_trigrams_when_parent_key_exists() {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let ja_path = "test_layout_filter_ja.txt";
                let db_path = "test_layout_filter.db";

                fs::write(ja_path, "kannzi").expect("Failed to write JA test file");

                let n_gram_db = NGramDB::new(
                    &[test_source(
                        "japanese_corpus",
                        SourceKind::Japanese,
                        ja_path,
                    )],
                    db_path,
                )
                .expect("Failed to create NGramDB");

                let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 'z', 'i']);
                layout
                    .assign_extension(0, AzikExtensionToken::Ann)
                    .expect("consonant key should accept extension");

                let ann = AzikExtensionToken::Ann.as_char();
                let tri_grams_without_extension = n_gram_db
                    .get_tri_grams_for_layout(|c| matches!(c, 'k' | 'z' | 'i'))
                    .expect("Failed to get tri-grams without extension");
                assert!(
                    !tri_grams_without_extension.contains_key(&LogicalNGram::new(['k', ann, 'z']))
                );

                let tri_grams_with_extension = n_gram_db
                    .get_tri_grams_for_layout(|c| layout.resolve_char_index(c).is_some())
                    .expect("Failed to get tri-grams with extension");
                assert!(tri_grams_with_extension.contains_key(&LogicalNGram::new(['k', ann, 'z'])));
                assert!(tri_grams_with_extension.contains_key(&LogicalNGram::new([ann, 'z', 'i'])));

                let cost_matrix = [1.0; NUM_COLS * NUM_ROWS];
                let finger_matrix = std::array::from_fn(|_| Finger::I);
                let mut physical_layout = PhysicalLayout::new(cost_matrix, finger_matrix)
                    .expect("layout should be valid");
                physical_layout.calculate_tri_gram_cost();

                let expected_cost = physical_layout.get_tri_gram_cost(
                    layout.get_char_index('k'),
                    layout.get_char_index('k'),
                    layout.get_char_index('z'),
                );
                let actual_cost = layout.evaluate(
                    &physical_layout,
                    &HashMap::from([(LogicalNGram::new(['k', ann, 'z']), 1.0)]),
                );
                assert_eq!(actual_cost, expected_cost);

                fs::remove_file(ja_path).expect("Failed to remove JA test file");
                fs::remove_file(db_path).expect("Failed to remove test database");
            })
            .expect("thread should spawn")
            .join()
            .expect("thread should finish");
    }
}
