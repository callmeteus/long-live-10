//! Shared validation primitives.
//!
//! These functions answer a yes/no question only. Each crate maps a `false`
//! into its own error type and message, so the on-disk and CLI error wording
//! stays untouched while the rule itself lives in one place.

/// True when a required text field has content after trimming.
pub fn is_nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

/// True for a 64-character hexadecimal digest.
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// True when a URL uses HTTPS, the only accepted source scheme.
pub fn is_https_url(value: &str) -> bool {
    value.starts_with("https://")
}
