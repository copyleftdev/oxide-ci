//! Cache key generation utilities.

use sha2::{Digest, Sha256};
use std::path::Path;

/// Generate a cache key from a template and file contents.
pub fn generate_key(template: &str, file_paths: &[&Path]) -> String {
    let mut hasher = Sha256::new();

    // Hash the template
    hasher.update(template.as_bytes());

    // Hash file contents
    for path in file_paths {
        if let Ok(contents) = std::fs::read(path) {
            hasher.update(&contents);
        }
    }

    let hash = hasher.finalize();
    let hash_str = hex::encode(&hash[..8]); // Use first 8 bytes

    // Replace {{ hashFiles(...) }} pattern with actual hash
    if template.contains("{{ hashFiles") {
        template
            .split("{{ hashFiles")
            .next()
            .unwrap_or(template)
            .to_string()
            + &hash_str
    } else {
        format!("{}-{}", template, hash_str)
    }
}

/// Check if a key matches a prefix pattern.
pub fn matches_prefix(key: &str, prefix: &str) -> bool {
    key.starts_with(prefix)
}

/// Sanitize a key for use in filenames.
pub fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key = generate_key("cargo", &[]);
        assert!(key.starts_with("cargo-"));
    }

    #[test]
    fn test_matches_prefix() {
        assert!(matches_prefix("cargo-abc123", "cargo-"));
        assert!(matches_prefix("cargo-abc123", "cargo"));
        assert!(!matches_prefix("npm-abc123", "cargo-"));
    }

    #[test]
    fn test_sanitize_key() {
        assert_eq!(sanitize_key("my/cache/key"), "my_cache_key");
        assert_eq!(sanitize_key("cache:key"), "cache_key");
    }
}
