use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

// Define characters that should NOT be percent-encoded
// https://url.spec.whatwg.org/#path-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');

/// Encode a slug or tag for use in URLs and filesystem paths
/// - Percent-encodes non-ASCII characters
/// - Keeps ASCII letters, numbers, hyphens, underscores, and dots as-is
/// - Truncates very long strings with a hash for uniqueness
pub fn encode_for_url(input: &str) -> String {
    let encoded = utf8_percent_encode(input, PATH).to_string();

    // Filesystem limit is usually 255 bytes, keep some margin
    const MAX_LEN: usize = 200;
    if encoded.len() > MAX_LEN {
        let hash = blake3::hash(encoded.as_bytes());
        format!("{}-{}", &encoded[..180], &hash.to_hex()[..16])
    } else {
        encoded
    }
}

/// Turn a heading's text into a stable anchor slug.
/// - ASCII letters/digits are lowercased and kept readable
/// - Runs of spaces/separators collapse into a single hyphen
/// - Punctuation is dropped
/// - Non-ASCII letters (e.g. Korean) are preserved, then percent-encoded so the
///   result is safe as both an `id` attribute and a URL fragment
pub fn slugify_heading(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut pending_separator = false;

    for ch in text.trim().chars() {
        if ch.is_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            pending_separator = false;
            slug.extend(ch.to_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' || ch == '\t' {
            pending_separator = true;
        }
    }

    if slug.is_empty() {
        return "section".to_string();
    }

    encode_for_url(&slug)
}

/// Decode a percent-encoded slug or tag back to the original string
pub fn decode_from_url(input: &str) -> String {
    percent_encoding::percent_decode_str(input)
        .decode_utf8()
        .unwrap_or(std::borrow::Cow::Borrowed(input))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ascii() {
        assert_eq!(encode_for_url("hello-world"), "hello-world");
        assert_eq!(encode_for_url("test_file.md"), "test_file.md");
    }

    #[test]
    fn test_encode_korean() {
        let encoded = encode_for_url("한글-테스트");
        assert!(encoded.contains("%ED%95%9C"));
    }

    #[test]
    fn test_decode() {
        let encoded = encode_for_url("한글-테스트");
        let decoded = decode_from_url(&encoded);
        assert_eq!(decoded, "한글-테스트");
    }

    #[test]
    fn test_slugify_heading_ascii() {
        assert_eq!(slugify_heading("Hello World"), "hello-world");
        assert_eq!(slugify_heading("  Trim  Me  "), "trim-me");
        assert_eq!(slugify_heading("Rust & Cargo: Tips!"), "rust-cargo-tips");
        assert_eq!(
            slugify_heading("snake_case-and-dash"),
            "snake-case-and-dash"
        );
    }

    #[test]
    fn test_slugify_heading_korean() {
        let slug = slugify_heading("한글 제목");
        assert!(slug.contains("%ED%95%9C"));
        assert!(slug.contains('-'));
    }

    #[test]
    fn test_slugify_heading_empty_fallback() {
        assert_eq!(slugify_heading("!!!"), "section");
        assert_eq!(slugify_heading(""), "section");
    }

    #[test]
    fn test_encode_long_string() {
        let long_string = "가".repeat(100); // 100 Korean characters
        let encoded = encode_for_url(&long_string);
        assert!(encoded.len() <= 200);
        assert!(encoded.contains('-')); // Should have hash separator
    }
}
