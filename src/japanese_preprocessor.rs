use crate::azik_extension::{is_consonant, AzikExtensionToken, AZIK_EXTENSION_TOKENS};

pub fn preprocess_japanese_romanization(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];
        result.push(current);
        index += 1;

        if !is_consonant(current) {
            continue;
        }

        if let Some((token, consumed)) = longest_matching_extension(&chars[index..]) {
            result.push(token.as_char());
            index += consumed;
        }
    }

    result
}

fn longest_matching_extension(chars: &[char]) -> Option<(AzikExtensionToken, usize)> {
    AZIK_EXTENSION_TOKENS
        .iter()
        .copied()
        .filter_map(|token| {
            let pattern_chars: Vec<char> = token.pattern().chars().collect();
            chars
                .starts_with(&pattern_chars)
                .then_some((token, pattern_chars.len()))
        })
        .max_by_key(|(_, len)| *len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::azik_extension::AzikExtensionToken;

    #[test]
    fn preprocesses_spec_examples_with_longest_match() {
        assert_eq!(
            preprocess_japanese_romanization("kannzi"),
            format!("k{}zi", AzikExtensionToken::Ann.as_char())
        );
        assert_eq!(
            preprocess_japanese_romanization("tou"),
            format!("t{}", AzikExtensionToken::Ou.as_char())
        );
        assert_eq!(
            preprocess_japanese_romanization("kannou"),
            format!("k{}ou", AzikExtensionToken::Ann.as_char())
        );
    }

    #[test]
    fn does_not_replace_without_preceding_plain_consonant() {
        assert_eq!(preprocess_japanese_romanization("ann"), "ann");
        assert_eq!(preprocess_japanese_romanization("aou"), "aou");
        assert_eq!(
            preprocess_japanese_romanization("touu"),
            format!("t{}u", AzikExtensionToken::Ou.as_char())
        );
        assert_eq!(
            preprocess_japanese_romanization("kouei"),
            format!("k{}ei", AzikExtensionToken::Ou.as_char())
        );
    }

    #[test]
    fn can_chain_multiple_extensions_after_separate_consonants() {
        assert_eq!(
            preprocess_japanese_romanization("kanntei"),
            format!(
                "k{}t{}",
                AzikExtensionToken::Ann.as_char(),
                AzikExtensionToken::Ei.as_char()
            )
        );
    }
}
