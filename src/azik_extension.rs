#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AzikExtensionToken {
    Ann,
    Inn,
    Unn,
    Enn,
    Onn,
    Ou,
    Aa,
    Ai,
    Ei,
    Oi,
    Uu,
    Oo,
}

pub const AZIK_EXTENSION_TOKENS: [AzikExtensionToken; 12] = [
    AzikExtensionToken::Ann,
    AzikExtensionToken::Inn,
    AzikExtensionToken::Unn,
    AzikExtensionToken::Enn,
    AzikExtensionToken::Onn,
    AzikExtensionToken::Ou,
    AzikExtensionToken::Aa,
    AzikExtensionToken::Ai,
    AzikExtensionToken::Ei,
    AzikExtensionToken::Oi,
    AzikExtensionToken::Uu,
    AzikExtensionToken::Oo,
];

pub const AZIK_EXTENSION_TOKEN_COUNT: usize = AZIK_EXTENSION_TOKENS.len();

const AZIK_TOKEN_CHAR_BASE: u32 = 0xF0000;

impl AzikExtensionToken {
    pub const fn pattern(self) -> &'static str {
        match self {
            Self::Ann => "ann",
            Self::Inn => "inn",
            Self::Unn => "unn",
            Self::Enn => "enn",
            Self::Onn => "onn",
            Self::Ou => "ou",
            Self::Aa => "ui",
            Self::Ai => "ai",
            Self::Ei => "ei",
            Self::Oi => "oi",
            Self::Uu => "uu",
            Self::Oo => "ao",
        }
    }

    pub const fn label(self) -> &'static str {
        self.pattern()
    }

    pub const fn as_index(self) -> u32 {
        match self {
            Self::Ann => 0,
            Self::Inn => 1,
            Self::Unn => 2,
            Self::Enn => 3,
            Self::Onn => 4,
            Self::Ou => 5,
            Self::Aa => 6,
            Self::Ai => 7,
            Self::Ei => 8,
            Self::Oi => 9,
            Self::Uu => 10,
            Self::Oo => 11,
        }
    }

    pub const fn as_usize(self) -> usize {
        self.as_index() as usize
    }

    pub fn as_char(self) -> char {
        char::from_u32(AZIK_TOKEN_CHAR_BASE + self.as_index())
            .expect("AZIK extension token char must be valid")
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c as u32 {
            AZIK_TOKEN_CHAR_BASE => Some(Self::Ann),
            x if x == AZIK_TOKEN_CHAR_BASE + 1 => Some(Self::Inn),
            x if x == AZIK_TOKEN_CHAR_BASE + 2 => Some(Self::Unn),
            x if x == AZIK_TOKEN_CHAR_BASE + 3 => Some(Self::Enn),
            x if x == AZIK_TOKEN_CHAR_BASE + 4 => Some(Self::Onn),
            x if x == AZIK_TOKEN_CHAR_BASE + 5 => Some(Self::Ou),
            x if x == AZIK_TOKEN_CHAR_BASE + 6 => Some(Self::Aa),
            x if x == AZIK_TOKEN_CHAR_BASE + 7 => Some(Self::Ai),
            x if x == AZIK_TOKEN_CHAR_BASE + 8 => Some(Self::Ei),
            x if x == AZIK_TOKEN_CHAR_BASE + 9 => Some(Self::Oi),
            x if x == AZIK_TOKEN_CHAR_BASE + 10 => Some(Self::Uu),
            x if x == AZIK_TOKEN_CHAR_BASE + 11 => Some(Self::Oo),
            _ => None,
        }
    }

    pub fn from_pattern(pattern: &str) -> Option<Self> {
        match pattern {
            "ann" => Some(Self::Ann),
            "inn" => Some(Self::Inn),
            "unn" => Some(Self::Unn),
            "enn" => Some(Self::Enn),
            "onn" => Some(Self::Onn),
            "ou" => Some(Self::Ou),
            "ui" => Some(Self::Aa),
            "ai" => Some(Self::Ai),
            "ei" => Some(Self::Ei),
            "oi" => Some(Self::Oi),
            "uu" => Some(Self::Uu),
            "ao" => Some(Self::Oo),
            _ => None,
        }
    }
}

pub fn is_azik_extension_token(c: char) -> bool {
    AzikExtensionToken::from_char(c).is_some()
}

pub fn azik_extension_label(c: char) -> Option<&'static str> {
    AzikExtensionToken::from_char(c).map(AzikExtensionToken::label)
}

pub fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'i' | 'u' | 'e' | 'o')
}

pub fn is_consonant(c: char) -> bool {
    c.is_ascii_lowercase() && !is_vowel(c)
}

pub fn can_host_azik_extension(c: char) -> bool {
    is_consonant(c) && !matches!(c, 'h' | 'y')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn azik_extension_tokens_are_defined_once() {
        assert_eq!(AZIK_EXTENSION_TOKENS.len(), 12);

        let unique_chars: HashSet<char> = AZIK_EXTENSION_TOKENS
            .iter()
            .map(|token| token.as_char())
            .collect();
        assert_eq!(unique_chars.len(), AZIK_EXTENSION_TOKENS.len());
    }

    #[test]
    fn azik_extension_char_roundtrip_and_labels_work() {
        for token in AZIK_EXTENSION_TOKENS {
            let token_char = token.as_char();
            assert_eq!(AzikExtensionToken::from_char(token_char), Some(token));
            assert_eq!(
                AzikExtensionToken::from_pattern(token.pattern()),
                Some(token)
            );
            assert_eq!(azik_extension_label(token_char), Some(token.label()));
            assert!(is_azik_extension_token(token_char));
        }

        assert!(!is_azik_extension_token('a'));
        assert_eq!(azik_extension_label('a'), None);
        assert_eq!(AzikExtensionToken::from_pattern("xyz"), None);
    }

    #[test]
    fn vowel_and_consonant_detection_is_ascii_lowercase_only() {
        for vowel in ['a', 'i', 'u', 'e', 'o'] {
            assert!(is_vowel(vowel));
            assert!(!is_consonant(vowel));
        }

        for consonant in ['k', 's', 't', 'n', 'h', 'm', 'y', 'r', 'w', 'z'] {
            assert!(is_consonant(consonant));
            assert!(!is_vowel(consonant));
        }

        for non_consonant in ['A', '1', '-', AzikExtensionToken::Ann.as_char()] {
            assert!(!is_consonant(non_consonant));
        }
    }

    #[test]
    fn azik_extension_hosts_exclude_h_and_y() {
        for host in ['k', 's', 't', 'n', 'm', 'r', 'w', 'z'] {
            assert!(can_host_azik_extension(host));
        }

        for forbidden in ['h', 'y', 'a', '1'] {
            assert!(!can_host_azik_extension(forbidden));
        }
    }
}
