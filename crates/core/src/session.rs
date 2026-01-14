//! Cryptographically secure session token generation using ChaCha20 RNG.
//!
//! Session tokens use ChaCha20 via `rand_chacha` for cryptographic security,
//! providing 256 bits of entropy for session identifiers.

use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rand_core::RngCore;
use std::sync::Mutex;

/// Thread-safe ChaCha20 RNG seeded from system entropy.
static CHACHA_RNG: Lazy<Mutex<ChaCha20Rng>> = Lazy::new(|| Mutex::new(ChaCha20Rng::from_entropy()));

/// URL-safe base64 alphabet for token encoding.
const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Session token length in bytes (256 bits = 32 bytes).
const SESSION_TOKEN_BYTES: usize = 32;

/// Generate a cryptographically secure session token.
///
/// Returns a URL-safe base64-encoded token with 256 bits of entropy.
/// The token is suitable for use as a session identifier in cookies.
///
/// # Example
///
/// ```
/// use dguesser_core::session::generate_session_token;
///
/// let token = generate_session_token();
/// assert_eq!(token.len(), 43); // 32 bytes -> 43 base64 chars (no padding)
/// ```
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; SESSION_TOKEN_BYTES];
    {
        let mut rng = CHACHA_RNG.lock().expect("ChaCha RNG lock poisoned");
        rng.fill_bytes(&mut bytes);
    }
    base64_url_encode(&bytes)
}

/// Generate a prefixed session token with the `ses_` prefix.
///
/// This combines the session prefix with a ChaCha20-generated token
/// for a complete session identifier.
///
/// # Example
///
/// ```
/// use dguesser_core::session::generate_prefixed_session_token;
///
/// let token = generate_prefixed_session_token();
/// assert!(token.starts_with("ses_"));
/// assert_eq!(token.len(), 47); // "ses_" (4) + 43 base64 chars
/// ```
pub fn generate_prefixed_session_token() -> String {
    format!("ses_{}", generate_session_token())
}

/// URL-safe base64 encode bytes without padding.
fn base64_url_encode(bytes: &[u8]) -> String {
    let mut result = String::with_capacity((bytes.len() * 4).div_ceil(3));

    let mut i = 0;
    while i + 2 < bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        result.push(BASE64_ALPHABET[(n >> 18) as usize & 0x3F] as char);
        result.push(BASE64_ALPHABET[(n >> 12) as usize & 0x3F] as char);
        result.push(BASE64_ALPHABET[(n >> 6) as usize & 0x3F] as char);
        result.push(BASE64_ALPHABET[n as usize & 0x3F] as char);
        i += 3;
    }

    // Handle remaining bytes
    match bytes.len() - i {
        2 => {
            let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
            result.push(BASE64_ALPHABET[(n >> 18) as usize & 0x3F] as char);
            result.push(BASE64_ALPHABET[(n >> 12) as usize & 0x3F] as char);
            result.push(BASE64_ALPHABET[(n >> 6) as usize & 0x3F] as char);
        }
        1 => {
            let n = (bytes[i] as u32) << 16;
            result.push(BASE64_ALPHABET[(n >> 18) as usize & 0x3F] as char);
            result.push(BASE64_ALPHABET[(n >> 12) as usize & 0x3F] as char);
        }
        _ => {}
    }

    result
}

/// Validate that a string looks like a valid session token.
///
/// This performs a basic format check, not cryptographic validation.
pub fn is_valid_token_format(token: &str) -> bool {
    // Check length: 43 chars for raw token, 47 for prefixed
    let valid_length = token.len() == 43 || token.len() == 47;
    if !valid_length {
        return false;
    }

    // If prefixed, check prefix
    if token.len() == 47 && !token.starts_with("ses_") {
        return false;
    }

    // Check that all characters are valid base64url
    let token_part = token.strip_prefix("ses_").unwrap_or(token);

    token_part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_session_token_length() {
        let token = generate_session_token();
        assert_eq!(token.len(), 43);
    }

    #[test]
    fn test_prefixed_session_token() {
        let token = generate_prefixed_session_token();
        assert!(token.starts_with("ses_"));
        assert_eq!(token.len(), 47);
    }

    #[test]
    fn test_tokens_are_unique() {
        let tokens: HashSet<String> = (0..100).map(|_| generate_session_token()).collect();
        assert_eq!(tokens.len(), 100);
    }

    #[test]
    fn test_token_is_url_safe() {
        let token = generate_session_token();
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_valid_token_format() {
        let token = generate_session_token();
        assert!(is_valid_token_format(&token));

        let prefixed = generate_prefixed_session_token();
        assert!(is_valid_token_format(&prefixed));
    }

    #[test]
    fn test_invalid_token_format() {
        assert!(!is_valid_token_format("too_short"));
        assert!(!is_valid_token_format("xxx_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk"));
        assert!(!is_valid_token_format(""));
    }
}
