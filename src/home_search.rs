//! Port of `HomeView.vue`'s `FEATURE_KEYWORDS` scoring logic (English keywords only,
//! since i18n is not ported — the it/en toggle is skipped per spec).

use crate::app::Route;

struct FeatureKeywords {
    route: Route,
    exact: &'static [&'static str],
    partial: &'static [&'static str],
}

const STOPWORDS: &[&str] = &[
    "to", "a", "the", "an", "of", "my", "need", "want", "how", "can", "do", "i",
];

const FEATURES: &[FeatureKeywords] = &[
    FeatureKeywords {
        route: Route::Converter,
        exact: &["convert", "conversion", "base64", "binary", "transform", "format", "encode", "decode", "encoding", "decoding"],
        partial: &["text", "string"],
    },
    FeatureKeywords {
        route: Route::KeyGenerator,
        exact: &["key", "keys", "generate", "generation", "password", "random", "secure", "cryptographic", "bit", "128", "256", "512"],
        partial: &["security"],
    },
    FeatureKeywords {
        route: Route::Encrypter,
        exact: &["encrypt", "encryption", "aes", "des", "protect", "hide", "secret", "cbc", "ecb", "crypt"],
        partial: &["secure", "security", "md5"],
    },
    FeatureKeywords {
        route: Route::Decrypter,
        exact: &["decrypt", "decryption", "decode", "aes", "des", "cbc", "ecb", "read", "open", "reveal"],
        partial: &["ciphertext", "payload"],
    },
    FeatureKeywords {
        route: Route::FileHasher,
        exact: &["hash", "hashing", "file", "md5", "checksum", "fingerprint", "integrity", "verify", "document", "calculate", "digest"],
        partial: &["check"],
    },
];

/// Mirrors `searchResult` computed prop: tokenize, score each feature (exact
/// match = 3 points, partial/substring match on tokens >= 4 chars = 1 point),
/// return the best-scoring feature if its score is >= 1.
pub fn search(query: &str) -> Option<Route> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return None;
    }
    let tokens: Vec<&str> = q
        .split_whitespace()
        .filter(|t| t.len() > 1 && !STOPWORDS.contains(t))
        .collect();
    if tokens.is_empty() {
        return None;
    }

    let mut best: Option<Route> = None;
    let mut best_score = 0i32;
    for feature in FEATURES {
        let mut score = 0i32;
        for token in &tokens {
            if feature.exact.contains(token) {
                score += 3;
                continue;
            }
            if token.len() >= 4 {
                if feature.exact.iter().any(|k| k.contains(token) || token.contains(k)) {
                    score += 1;
                    continue;
                }
                if feature.partial.iter().any(|k| k.contains(token) || token.contains(k)) {
                    score += 1;
                }
            }
        }
        if score > best_score {
            best_score = score;
            best = Some(feature.route);
        }
    }
    if best_score < 1 { None } else { best }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_converter() {
        assert_eq!(search("convert this text to base64"), Some(Route::Converter));
    }

    #[test]
    fn matches_key_generator() {
        assert_eq!(search("generate a secure key"), Some(Route::KeyGenerator));
    }

    #[test]
    fn matches_encrypter() {
        assert_eq!(search("encrypt my secret with aes"), Some(Route::Encrypter));
    }

    #[test]
    fn matches_decrypter() {
        assert_eq!(search("decrypt this ciphertext"), Some(Route::Decrypter));
    }

    #[test]
    fn matches_file_hasher() {
        assert_eq!(search("calculate the md5 hash of a file"), Some(Route::FileHasher));
    }

    #[test]
    fn no_match_for_gibberish() {
        assert_eq!(search("qwertyuiop asdfgh"), None);
    }

    #[test]
    fn empty_query_no_match() {
        assert_eq!(search(""), None);
    }
}
