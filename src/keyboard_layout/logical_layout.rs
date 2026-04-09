use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use super::physical_layout::{PhysicalLayout, NUM_COLS, NUM_LAYERS, NUM_ROWS};
use crate::azik_extension::{
    can_host_azik_extension, AzikExtensionToken, AZIK_EXTENSION_TOKENS, AZIK_EXTENSION_TOKEN_COUNT,
};
use crate::n_gram::LogicalNGram;

const TOTAL_LOGICAL_KEYS: usize = NUM_COLS * NUM_ROWS * NUM_LAYERS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalLayoutError {
    IndexOutOfRange {
        index: usize,
    },
    KeyCannotHostExtension {
        index: usize,
        base_char: char,
    },
    KeyAlreadyHasExtension {
        index: usize,
        token: AzikExtensionToken,
    },
    ExtensionAlreadyAssigned {
        token: AzikExtensionToken,
        index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutLookup {
    Char(char),
    CharId(usize),
    AzikExtension(AzikExtensionToken),
}

#[derive(Debug, Clone)]
pub struct LogicalLayout {
    layout: [char; TOTAL_LOGICAL_KEYS],
    char_map: HashMap<char, usize>,
    dummy_chars: HashSet<char>,
    extension_by_index: [Option<AzikExtensionToken>; TOTAL_LOGICAL_KEYS],
    extension_parent_indices: [Option<usize>; AZIK_EXTENSION_TOKEN_COUNT],
    char_to_id: HashMap<char, usize>,
    id_to_index: Vec<usize>,
}

impl LogicalLayout {
    pub fn from_usable_chars(usable_chars: &[char]) -> Self {
        if usable_chars.len() > TOTAL_LOGICAL_KEYS {
            panic!("Too many usable characters: {}", usable_chars.len());
        }

        let mut layout: [char; TOTAL_LOGICAL_KEYS] = [' '; TOTAL_LOGICAL_KEYS];
        let mut char_map = HashMap::new();
        let mut char_to_id = HashMap::new();
        let mut id_to_index: Vec<usize> = Vec::with_capacity(usable_chars.len());
        let mut dummy_chars = HashSet::new();
        // Fill layout with provided usable chars; if not enough, use unique dummy chars
        // Use Unicode Private Use Area starting at U+E000 to avoid collisions
        let mut dummy_counter: u32 = 0;
        for i in 0..TOTAL_LOGICAL_KEYS {
            if i < usable_chars.len() {
                let ch = usable_chars[i];
                layout[i] = ch;
                char_map.insert(ch, i);
                // 使用文字に連番IDを付与
                char_to_id.insert(ch, i);
                id_to_index.push(i);
            } else {
                // Generate a unique dummy character that doesn't collide with usable_chars
                let dummy_base: u32 = 0xE000; // Private Use Area start
                let mut dummy_ch = std::char::from_u32(dummy_base + dummy_counter)
                    .expect("Failed to create dummy character");
                // Ensure uniqueness and avoid accidental collision with usable chars
                while usable_chars.contains(&dummy_ch) || char_map.contains_key(&dummy_ch) {
                    dummy_counter += 1;
                    dummy_ch = std::char::from_u32(dummy_base + dummy_counter)
                        .expect("Failed to create dummy character");
                }
                layout[i] = dummy_ch;
                char_map.insert(dummy_ch, i);
                dummy_chars.insert(dummy_ch);
                dummy_counter += 1;
            }
        }
        LogicalLayout {
            layout,
            char_map,
            dummy_chars,
            extension_by_index: [None; TOTAL_LOGICAL_KEYS],
            extension_parent_indices: [None; AZIK_EXTENSION_TOKEN_COUNT],
            char_to_id,
            id_to_index,
        }
    }

    pub fn evaluate(
        &self,
        physical_layout: &PhysicalLayout,
        tri_grams: &HashMap<LogicalNGram<3>, f32>,
    ) -> f32 {
        tri_grams
            .iter()
            .map(|(n_gram, score)| {
                let k1 = self.resolve_lookup_or_panic(
                    LayoutLookup::Char(n_gram.get(0)),
                    "evaluate",
                    Some(n_gram),
                );
                let k2 = self.resolve_lookup_or_panic(
                    LayoutLookup::Char(n_gram.get(1)),
                    "evaluate",
                    Some(n_gram),
                );
                let k3 = self.resolve_lookup_or_panic(
                    LayoutLookup::Char(n_gram.get(2)),
                    "evaluate",
                    Some(n_gram),
                );
                *score * physical_layout.get_tri_gram_cost(k1, k2, k3)
            })
            .sum()
    }

    pub fn evaluate_par(
        &self,
        physical_layout: &PhysicalLayout,
        tri_grams_vec: &[(&LogicalNGram<3>, f32)],
    ) -> f32 {
        tri_grams_vec
            .par_iter()
            .map(|(n_gram, score)| {
                let k1 = self.resolve_lookup_or_panic(
                    LayoutLookup::Char(n_gram.get(0)),
                    "evaluate_par",
                    Some(n_gram),
                );
                let k2 = self.resolve_lookup_or_panic(
                    LayoutLookup::Char(n_gram.get(1)),
                    "evaluate_par",
                    Some(n_gram),
                );
                let k3 = self.resolve_lookup_or_panic(
                    LayoutLookup::Char(n_gram.get(2)),
                    "evaluate_par",
                    Some(n_gram),
                );
                *score * physical_layout.get_tri_gram_cost(k1, k2, k3)
            })
            .sum()
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        // 位置スワップ
        let ca = self.layout[a];
        let cb = self.layout[b];
        self.char_map.insert(ca, b);
        self.char_map.insert(cb, a);
        self.layout.swap(a, b);
        // 使用文字ならID→インデックス配列も更新
        if let Some(&ida) = self.char_to_id.get(&ca) {
            self.id_to_index[ida] = b;
        }
        if let Some(&idb) = self.char_to_id.get(&cb) {
            self.id_to_index[idb] = a;
        }
        self.drop_extension_if_invalid(a);
        self.drop_extension_if_invalid(b);
    }

    pub fn get_char_index(&self, c: char) -> usize {
        self.resolve_lookup(LayoutLookup::Char(c))
    }

    pub fn resolve_char_index(&self, c: char) -> Option<usize> {
        if let Some(token) = AzikExtensionToken::from_char(c) {
            return self.extension_parent_indices[token.as_usize()];
        }

        self.char_map.get(&c).copied()
    }

    pub fn resolve_lookup(&self, lookup: LayoutLookup) -> usize {
        self.resolve_lookup_opt(lookup)
            .unwrap_or_else(|| match lookup {
                LayoutLookup::Char(c) => panic!("Character {} not found", c),
                LayoutLookup::CharId(id) => panic!("Character ID {} not found", id),
                LayoutLookup::AzikExtension(token) => {
                    panic!("AZIK extension token {:?} not found", token)
                }
            })
    }

    pub fn resolve_lookup_opt(&self, lookup: LayoutLookup) -> Option<usize> {
        match lookup {
            LayoutLookup::Char(c) => self.resolve_char_index(c),
            LayoutLookup::CharId(id) => self.id_to_index.get(id).copied(),
            LayoutLookup::AzikExtension(token) => self.extension_parent_indices[token.as_usize()],
        }
    }

    pub fn get(&self, index: usize) -> char {
        self.layout[index]
    }

    pub fn set(&mut self, index: usize, c: char) {
        self.layout[index] = c;
        self.char_map.insert(c, index);
        if let Some(&id) = self.char_to_id.get(&c) {
            self.id_to_index[id] = index;
        }
        self.drop_extension_if_invalid(index);
    }

    pub fn len(&self) -> usize {
        self.layout.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
    }

    pub fn char_nums(&self) -> usize {
        self.char_map.len()
    }

    pub fn output(&self) -> [char; TOTAL_LOGICAL_KEYS] {
        self.layout
    }

    pub fn assign_extension(
        &mut self,
        index: usize,
        token: AzikExtensionToken,
    ) -> Result<(), LogicalLayoutError> {
        if index >= self.layout.len() {
            return Err(LogicalLayoutError::IndexOutOfRange { index });
        }

        let base_char = self.layout[index];
        if !self.can_host_extension(index) {
            return Err(LogicalLayoutError::KeyCannotHostExtension { index, base_char });
        }

        if let Some(existing) = self.extension_by_index[index] {
            return Err(LogicalLayoutError::KeyAlreadyHasExtension {
                index,
                token: existing,
            });
        }

        if let Some(existing_index) = self.extension_parent_indices[token.as_usize()] {
            return Err(LogicalLayoutError::ExtensionAlreadyAssigned {
                token,
                index: existing_index,
            });
        }

        self.extension_by_index[index] = Some(token);
        self.extension_parent_indices[token.as_usize()] = Some(index);
        Ok(())
    }

    pub fn remove_extension(&mut self, index: usize) -> Option<AzikExtensionToken> {
        let token = self.extension_by_index[index].take()?;
        self.extension_parent_indices[token.as_usize()] = None;
        Some(token)
    }

    pub fn get_extension(&self, index: usize) -> Option<AzikExtensionToken> {
        self.extension_by_index.get(index).copied().flatten()
    }

    pub fn get_extension_parent_index(&self, token: AzikExtensionToken) -> Option<usize> {
        self.extension_parent_indices[token.as_usize()]
    }

    pub fn clear_extensions(&mut self) {
        self.extension_by_index.fill(None);
        self.extension_parent_indices.fill(None);
    }

    pub fn extension_assignments(&self) -> Vec<(usize, AzikExtensionToken)> {
        let mut assignments = self
            .extension_parent_indices
            .iter()
            .enumerate()
            .filter_map(|(token_index, index)| {
                index.map(|index| (index, AZIK_EXTENSION_TOKENS[token_index]))
            })
            .collect::<Vec<_>>();
        assignments.sort_by_key(|(index, _)| *index);
        assignments
    }

    pub fn hostable_extension_indices(&self) -> Vec<usize> {
        (0..self.layout.len())
            .filter(|&index| self.can_host_extension(index))
            .collect()
    }

    pub fn extension_parent_indices(&self) -> &[Option<usize>; AZIK_EXTENSION_TOKEN_COUNT] {
        &self.extension_parent_indices
    }

    pub fn assign_default_azik_extensions(&mut self) {
        let mut assigned_count = 0usize;
        for index in 0..self.layout.len() {
            if assigned_count == AZIK_EXTENSION_TOKENS.len() {
                break;
            }
            if !self.can_host_extension(index) || self.extension_by_index[index].is_some() {
                continue;
            }

            let Some(token) = AZIK_EXTENSION_TOKENS
                .iter()
                .copied()
                .find(|token| self.extension_parent_indices[token.as_usize()].is_none())
            else {
                break;
            };

            self.assign_extension(index, token)
                .expect("default AZIK extension assignment should stay valid");
            assigned_count += 1;
        }
    }

    pub fn can_host_extension(&self, index: usize) -> bool {
        self.layout
            .get(index)
            .copied()
            .is_some_and(|c| !self.dummy_chars.contains(&c) && can_host_azik_extension(c))
    }

    pub fn key_label(&self, index: usize) -> Option<String> {
        let ch = *self.layout.get(index)?;
        if self.dummy_chars.contains(&ch) {
            return None;
        }

        let mut label = ch.to_string();
        if let Some(token) = self.extension_by_index[index] {
            label.push('(');
            label.push_str(token.label());
            label.push(')');
        }
        Some(label)
    }

    pub fn display_lines(&self) -> Vec<String> {
        let separator = std::iter::repeat_n("--", NUM_COLS + 1).collect::<String>();
        let mut lines = Vec::with_capacity(NUM_LAYERS * (NUM_ROWS + 2));

        for layer in 0..NUM_LAYERS {
            let mut row_labels = Vec::with_capacity(NUM_ROWS);
            let mut width = 1usize;
            for row in 0..NUM_ROWS {
                let labels = (0..NUM_COLS)
                    .map(|col| {
                        let idx = layer * (NUM_COLS * NUM_ROWS) + row * NUM_COLS + col;
                        self.key_label(idx).unwrap_or_default()
                    })
                    .collect::<Vec<_>>();
                width = width.max(labels.iter().map(String::len).max().unwrap_or(0));
                row_labels.push(labels);
            }

            lines.push(format!("Layer {}:", layer));
            for labels in row_labels {
                let mut row_line = String::new();
                for (col, label) in labels.into_iter().enumerate() {
                    if col == NUM_COLS / 2 {
                        row_line.push_str("| ");
                    }
                    row_line.push_str(&format!("{label:<width$} ", width = width));
                }
                lines.push(row_line.trim_end().to_string());
            }
            lines.push(separator.clone());
        }

        lines
    }

    pub fn print(&self) {
        println!();
        for line in self.display_lines() {
            println!("{line}");
        }
    }

    // IDから現在インデックスをO(1)で取得
    pub fn get_index_by_id(&self, id: usize) -> usize {
        self.id_to_index[id]
    }

    // 使用文字→IDの参照を外部で使えるようにする（必要なら）
    pub fn get_char_to_id(&self) -> &HashMap<char, usize> {
        &self.char_to_id
    }

    // IDベース評価（トライグラムは[usize;3]のID列）
    pub fn evaluate_ids(
        &self,
        physical_layout: &PhysicalLayout,
        tri_grams_ids: &[([LayoutLookup; 3], f32)],
    ) -> f32 {
        tri_grams_ids
            .iter()
            .map(|(ids, score)| {
                let k1 = self.resolve_lookup_or_panic(ids[0], "evaluate_ids", None);
                let k2 = self.resolve_lookup_or_panic(ids[1], "evaluate_ids", None);
                let k3 = self.resolve_lookup_or_panic(ids[2], "evaluate_ids", None);
                *score * physical_layout.get_tri_gram_cost(k1, k2, k3)
            })
            .sum()
    }

    fn resolve_lookup_or_panic(
        &self,
        lookup: LayoutLookup,
        context: &'static str,
        n_gram: Option<&LogicalNGram<3>>,
    ) -> usize {
        self.resolve_lookup_opt(lookup)
            .unwrap_or_else(|| match (lookup, n_gram) {
                (LayoutLookup::Char(c), Some(n_gram)) => {
                    panic!("{context}: unresolved character {c:?} in tri-gram {n_gram:?}")
                }
                (LayoutLookup::Char(c), None) => panic!("{context}: unresolved character {c:?}"),
                (LayoutLookup::CharId(id), Some(n_gram)) => {
                    panic!("{context}: unresolved character id {id} in tri-gram {n_gram:?}")
                }
                (LayoutLookup::CharId(id), None) => {
                    panic!("{context}: unresolved character id {id}")
                }
                (LayoutLookup::AzikExtension(token), Some(n_gram)) => {
                    panic!("{context}: unresolved AZIK extension {token:?} in tri-gram {n_gram:?}")
                }
                (LayoutLookup::AzikExtension(token), None) => {
                    panic!("{context}: unresolved AZIK extension {token:?}")
                }
            })
    }

    fn drop_extension_if_invalid(&mut self, index: usize) {
        if self.can_host_extension(index) {
            return;
        }

        if let Some(token) = self.extension_by_index[index].take() {
            self.extension_parent_indices[token.as_usize()] = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::azik_extension::is_azik_extension_token;
    use std::collections::HashSet;

    fn test_physical_layout() -> PhysicalLayout {
        let cost_matrix = [1.0; NUM_COLS * NUM_ROWS];
        let finger_matrix = std::array::from_fn(|index| match index % NUM_COLS {
            0..=4 => super::super::Finger::I,
            _ => super::super::Finger::M,
        });
        let mut physical_layout =
            PhysicalLayout::new(cost_matrix, finger_matrix).expect("layout should be valid");
        physical_layout.calculate_tri_gram_cost();
        physical_layout
    }

    #[test]
    fn consonant_keys_can_hold_extensions_and_resolve_to_parent_index() {
        let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("consonant key should accept extension");

        assert_eq!(layout.get_extension(0), Some(AzikExtensionToken::Ann));
        assert_eq!(
            layout.get_extension_parent_index(AzikExtensionToken::Ann),
            Some(0)
        );
        assert_eq!(layout.get_char_index(AzikExtensionToken::Ann.as_char()), 0);
    }

    #[test]
    fn vowel_keys_cannot_hold_extensions() {
        let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        let err = layout
            .assign_extension(1, AzikExtensionToken::Ann)
            .expect_err("vowel key should reject extension");

        assert!(matches!(
            err,
            LogicalLayoutError::KeyCannotHostExtension {
                index: 1,
                base_char: 'a'
            }
        ));
    }

    #[test]
    fn h_and_y_keys_cannot_hold_extensions() {
        let mut layout = LogicalLayout::from_usable_chars(&['h', 'y', 'k']);

        for index in [0, 1] {
            let err = layout
                .assign_extension(index, AzikExtensionToken::Ann)
                .expect_err("h and y should reject AZIK extensions");

            let base_char = layout.get(index);
            assert!(matches!(
                err,
                LogicalLayoutError::KeyCannotHostExtension {
                    index: _,
                    base_char: _
                }
            ));
            assert_eq!(
                err,
                LogicalLayoutError::KeyCannotHostExtension { index, base_char }
            );
        }
    }

    #[test]
    fn a_key_cannot_hold_more_than_one_extension() {
        let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("first assignment should succeed");

        let err = layout
            .assign_extension(0, AzikExtensionToken::Ou)
            .expect_err("same key should reject second extension");

        assert!(matches!(
            err,
            LogicalLayoutError::KeyAlreadyHasExtension {
                index: 0,
                token: AzikExtensionToken::Ann
            }
        ));
    }

    #[test]
    fn an_extension_token_cannot_be_assigned_to_multiple_keys() {
        let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("first assignment should succeed");

        let err = layout
            .assign_extension(2, AzikExtensionToken::Ann)
            .expect_err("token should remain unique");

        assert!(matches!(
            err,
            LogicalLayoutError::ExtensionAlreadyAssigned {
                token: AzikExtensionToken::Ann,
                index: 0
            }
        ));
    }

    #[test]
    fn updating_a_key_to_non_consonant_drops_its_extension() {
        let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("first assignment should succeed");
        layout.set(0, 'a');

        assert_eq!(layout.get_extension(0), None);
        assert_eq!(
            layout.get_extension_parent_index(AzikExtensionToken::Ann),
            None
        );
        assert!(!is_azik_extension_token(layout.get(0)));
    }

    #[test]
    fn resolve_char_index_returns_none_for_unassigned_extension_tokens() {
        let layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        assert_eq!(
            layout.resolve_char_index(AzikExtensionToken::Ann.as_char()),
            None
        );
    }

    #[test]
    fn resolve_lookup_supports_both_char_ids_and_extension_tokens() {
        let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);

        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("consonant key should accept extension");

        assert_eq!(
            layout.resolve_lookup(LayoutLookup::Char(AzikExtensionToken::Ann.as_char())),
            layout.get_char_index('k')
        );
        assert_eq!(
            layout.resolve_lookup(LayoutLookup::AzikExtension(AzikExtensionToken::Ann)),
            layout.get_char_index('k')
        );
        assert_eq!(
            layout.resolve_lookup(LayoutLookup::CharId(
                *layout.get_char_to_id().get(&'s').expect("id should exist")
            )),
            layout.get_char_index('s')
        );
    }

    #[test]
    fn evaluate_paths_score_assigned_extension_tokens_with_parent_key_cost() {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let mut layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);
                layout
                    .assign_extension(0, AzikExtensionToken::Ann)
                    .expect("consonant key should accept extension");

                let physical_layout = test_physical_layout();
                let ann = AzikExtensionToken::Ann.as_char();
                let tri_gram = LogicalNGram::new(['k', ann, 's']);
                let tri_grams = HashMap::from([(tri_gram, 1.0)]);
                let tri_gram_ids = [(
                    [
                        LayoutLookup::CharId(
                            *layout.get_char_to_id().get(&'k').expect("id should exist"),
                        ),
                        LayoutLookup::AzikExtension(AzikExtensionToken::Ann),
                        LayoutLookup::CharId(
                            *layout.get_char_to_id().get(&'s').expect("id should exist"),
                        ),
                    ],
                    1.0,
                )];

                let expected_cost = physical_layout.get_tri_gram_cost(
                    layout.get_char_index('k'),
                    layout.get_char_index('k'),
                    layout.get_char_index('s'),
                );

                assert_eq!(layout.evaluate(&physical_layout, &tri_grams), expected_cost);
                assert_eq!(
                    layout.evaluate_ids(&physical_layout, &tri_gram_ids),
                    expected_cost
                );
            })
            .expect("thread should spawn")
            .join()
            .expect("thread should finish");
    }

    #[test]
    fn evaluate_paths_panic_on_unassigned_extension_tokens() {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let layout = LogicalLayout::from_usable_chars(&['k', 'a', 's']);
                let physical_layout = test_physical_layout();
                let ann = AzikExtensionToken::Ann.as_char();
                let tri_gram = LogicalNGram::new(['k', ann, 's']);
                let tri_grams = HashMap::from([(tri_gram, 1.0)]);
                let tri_gram_ids = [(
                    [
                        LayoutLookup::CharId(
                            *layout.get_char_to_id().get(&'k').expect("id should exist"),
                        ),
                        LayoutLookup::AzikExtension(AzikExtensionToken::Ann),
                        LayoutLookup::CharId(
                            *layout.get_char_to_id().get(&'s').expect("id should exist"),
                        ),
                    ],
                    1.0,
                )];

                let evaluate_err =
                    std::panic::catch_unwind(|| layout.evaluate(&physical_layout, &tri_grams))
                        .expect_err("evaluate should fail fast on unresolved extension");
                let evaluate_message = panic_message(&evaluate_err);
                assert!(evaluate_message.contains("evaluate: unresolved character"));
                assert!(evaluate_message.contains(&format!("{ann:?}")));

                let evaluate_ids_err = std::panic::catch_unwind(|| {
                    layout.evaluate_ids(&physical_layout, &tri_gram_ids)
                })
                .expect_err("evaluate_ids should fail fast on unresolved extension");
                let evaluate_ids_message = panic_message(&evaluate_ids_err);
                assert!(evaluate_ids_message.contains("evaluate_ids: unresolved AZIK extension"));
                assert!(evaluate_ids_message.contains("Ann"));
            })
            .expect("thread should spawn")
            .join()
            .expect("thread should finish");
    }

    fn panic_message(err: &Box<dyn std::any::Any + Send>) -> String {
        if let Some(message) = err.downcast_ref::<String>() {
            return message.clone();
        }
        if let Some(message) = err.downcast_ref::<&str>() {
            return (*message).to_string();
        }
        "non-string panic payload".to_string()
    }

    #[test]
    fn default_azik_extensions_are_assigned_to_consonant_keys_only() {
        let mut layout = LogicalLayout::from_usable_chars(&[
            'k', 'a', 's', 'i', 't', 'u', 'n', 'e', 'h', 'o', 'm', 'y', 'r', 'w', 'z', 'd',
        ]);

        layout.assign_default_azik_extensions();

        for token in AZIK_EXTENSION_TOKENS {
            let index = layout
                .get_extension_parent_index(token)
                .expect("every token should get a default parent");
            assert!(layout.can_host_extension(index));
            assert_eq!(layout.resolve_char_index(token.as_char()), Some(index));
        }
    }

    #[test]
    fn default_azik_extensions_assign_each_token_and_parent_once() {
        let mut layout = LogicalLayout::from_usable_chars(&[
            'k', 'a', 's', 'i', 't', 'u', 'n', 'e', 'h', 'o', 'm', 'y', 'r', 'w', 'z', 'd',
        ]);

        layout.assign_default_azik_extensions();

        let assignments = layout.extension_assignments();
        let assigned_indices: HashSet<usize> =
            assignments.iter().map(|(index, _)| *index).collect();
        let assigned_tokens: HashSet<AzikExtensionToken> =
            assignments.iter().map(|(_, token)| *token).collect();

        assert_eq!(assignments.len(), AZIK_EXTENSION_TOKENS.len());
        assert_eq!(assigned_indices.len(), AZIK_EXTENSION_TOKENS.len());
        assert_eq!(
            assigned_tokens,
            AZIK_EXTENSION_TOKENS.iter().copied().collect()
        );
    }

    #[test]
    fn key_label_formats_extension_as_parenthesized_suffix() {
        let mut layout = LogicalLayout::from_usable_chars(&['s', 'a', 'k']);
        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("consonant key should accept extension");

        assert_eq!(layout.key_label(0).as_deref(), Some("s(ann)"));
        assert_eq!(layout.key_label(1).as_deref(), Some("a"));
    }

    #[test]
    fn display_lines_keep_dummy_keys_blank_and_show_extension_labels() {
        let mut layout = LogicalLayout::from_usable_chars(&['s', 'a', 'k']);
        layout
            .assign_extension(0, AzikExtensionToken::Ann)
            .expect("consonant key should accept extension");

        let lines = layout.display_lines();

        assert_eq!(lines[0], "Layer 0:");
        assert!(lines[1].contains("s(ann)"));
        assert!(lines[1].contains("a"));
        assert!(lines[1].contains('|'));
        assert!(!lines[1].contains('\u{e000}'));
    }
}
