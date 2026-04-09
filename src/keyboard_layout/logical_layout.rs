use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use super::physical_layout::{PhysicalLayout, NUM_COLS, NUM_LAYERS, NUM_ROWS};
use crate::azik_extension::{is_consonant, AzikExtensionToken};
use crate::n_gram::LogicalNGram;

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

#[derive(Debug, Clone)]
pub struct LogicalLayout {
    layout: [char; NUM_COLS * NUM_ROWS * NUM_LAYERS],
    char_map: HashMap<char, usize>,
    dummy_chars: HashSet<char>,
    extension_map: HashMap<usize, AzikExtensionToken>,
    extension_parent_map: HashMap<char, usize>,
    // 追加: 使用文字のID割り当て（固定）、ID→現在のインデックスのO(1)配列
    char_to_id: HashMap<char, usize>,
    id_to_index: Vec<usize>,
}

impl LogicalLayout {
    pub fn from_usable_chars(usable_chars: &[char]) -> Self {
        if usable_chars.len() > NUM_COLS * NUM_ROWS * NUM_LAYERS {
            panic!("Too many usable characters: {}", usable_chars.len());
        }

        let mut layout: [char; NUM_COLS * NUM_ROWS * NUM_LAYERS] =
            [' '; NUM_COLS * NUM_ROWS * NUM_LAYERS];
        let mut char_map = HashMap::new();
        let mut char_to_id = HashMap::new();
        let mut id_to_index: Vec<usize> = Vec::with_capacity(usable_chars.len());
        let mut dummy_chars = HashSet::new();
        // Fill layout with provided usable chars; if not enough, use unique dummy chars
        // Use Unicode Private Use Area starting at U+E000 to avoid collisions
        let mut dummy_counter: u32 = 0;
        for i in 0..NUM_COLS * NUM_ROWS * NUM_LAYERS {
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
            extension_map: HashMap::new(),
            extension_parent_map: HashMap::new(),
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
            .map(|(n_gram, score)| -> f32 {
                let k1 = self.get_char_index(n_gram.get(0));
                let k2 = self.get_char_index(n_gram.get(1));
                let k3 = self.get_char_index(n_gram.get(2));
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
            .map(|(n_gram, score)| -> f32 {
                let k1 = self.get_char_index(n_gram.get(0));
                let k2 = self.get_char_index(n_gram.get(1));
                let k3 = self.get_char_index(n_gram.get(2));
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
        if let Some(&index) = self.extension_parent_map.get(&c) {
            return index;
        }

        *self
            .char_map
            .get(&c)
            .unwrap_or_else(|| panic!("Character {} not found", c))
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

    pub fn output(&self) -> [char; NUM_COLS * NUM_ROWS * NUM_LAYERS] {
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

        if let Some(existing) = self.extension_map.get(&index) {
            return Err(LogicalLayoutError::KeyAlreadyHasExtension {
                index,
                token: *existing,
            });
        }

        let token_char = token.as_char();
        if let Some(&existing_index) = self.extension_parent_map.get(&token_char) {
            return Err(LogicalLayoutError::ExtensionAlreadyAssigned {
                token,
                index: existing_index,
            });
        }

        self.extension_map.insert(index, token);
        self.extension_parent_map.insert(token_char, index);
        Ok(())
    }

    pub fn remove_extension(&mut self, index: usize) -> Option<AzikExtensionToken> {
        let token = self.extension_map.remove(&index)?;
        self.extension_parent_map.remove(&token.as_char());
        Some(token)
    }

    pub fn get_extension(&self, index: usize) -> Option<AzikExtensionToken> {
        self.extension_map.get(&index).copied()
    }

    pub fn get_extension_parent_index(&self, token: AzikExtensionToken) -> Option<usize> {
        self.extension_parent_map.get(&token.as_char()).copied()
    }

    pub fn can_host_extension(&self, index: usize) -> bool {
        self.layout
            .get(index)
            .copied()
            .is_some_and(|c| !self.dummy_chars.contains(&c) && is_consonant(c))
    }

    pub fn print(&self) {
        println!();
        for layer in 0..NUM_LAYERS {
            println!("Layer {}:", layer);
            for row in 0..NUM_ROWS {
                for col in 0..NUM_COLS {
                    let idx = layer * (NUM_COLS * NUM_ROWS) + row * NUM_COLS + col;
                    let ch = self.layout[idx];
                    if col == NUM_COLS / 2 {
                        print!("| ");
                    }
                    if self.dummy_chars.contains(&ch) {
                        // Do not display dummy characters
                        print!("  ");
                    } else {
                        print!("{} ", ch);
                    }
                }
                println!();
            }
            std::iter::repeat_n("--", NUM_COLS + 1).for_each(|c| {
                print!("{}", c);
            });
            println!();
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
        tri_grams_ids: &[([usize; 3], f32)],
    ) -> f32 {
        tri_grams_ids
            .par_iter()
            .map(|(ids, score)| -> f32 {
                let k1 = self.get_index_by_id(ids[0]);
                let k2 = self.get_index_by_id(ids[1]);
                let k3 = self.get_index_by_id(ids[2]);
                *score * physical_layout.get_tri_gram_cost(k1, k2, k3)
            })
            .sum()
    }

    fn drop_extension_if_invalid(&mut self, index: usize) {
        if self.can_host_extension(index) {
            return;
        }

        if let Some(token) = self.extension_map.remove(&index) {
            self.extension_parent_map.remove(&token.as_char());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::azik_extension::is_azik_extension_token;

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
}
