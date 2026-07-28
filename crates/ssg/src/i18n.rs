use crate::config::{lang_display_name, LanguagesConfig};
use crate::slug::encode_for_url;
use serde::Serialize;
use std::collections::HashMap;

/// Records which languages have a published (non-hidden) version of each post,
/// keyed by the post's `(category, slug)` identity. Built once before rendering
/// so any post can discover its translations for the language switcher and
/// hreflang alternates.
#[derive(Debug, Default, Clone)]
pub struct TranslationIndex {
    map: HashMap<(String, String), Vec<String>>,
}

impl TranslationIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register that `lang` exists for `(category, slug)`. Idempotent.
    pub fn record(&mut self, category: &str, slug: &str, lang: &str) {
        let langs = self
            .map
            .entry((category.to_string(), slug.to_string()))
            .or_default();
        if !langs.iter().any(|l| l == lang) {
            langs.push(lang.to_string());
        }
    }

    /// All languages available for this post (unordered set as recorded).
    fn languages_for(&self, category: &str, slug: &str) -> &[String] {
        self.map
            .get(&(category.to_string(), slug.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// A deterministic fingerprint of the multilingual topology — every post that
    /// has more than one language version, with its sorted language set. A post's
    /// switcher/hreflang markup depends on which languages exist for it, but its
    /// own file hash does not change when a sibling translation is added, removed,
    /// or hidden. Folding this fingerprint into cache validity forces the
    /// counterparts to rebuild on those events, while monolingual posts (excluded
    /// here) keep ordinary incremental behavior.
    pub fn topology_fingerprint(&self) -> String {
        let mut groups: Vec<String> = self
            .map
            .iter()
            .filter(|(_, langs)| langs.len() > 1)
            .map(|((category, slug), langs)| {
                let mut langs = langs.clone();
                langs.sort();
                format!("{}\u{0}{}\u{0}{}", category, slug, langs.join(","))
            })
            .collect();
        groups.sort();
        blake3::hash(groups.join("\u{1}").as_bytes())
            .to_hex()
            .to_string()
    }
}

/// A link to another language version of the current post (for the switcher).
#[derive(Debug, Serialize)]
pub struct LanguageLink {
    pub lang: String,
    pub label: String,
    /// Site-relative path, e.g. `/en/chat/foo/`.
    pub url: String,
    /// True for the language currently being rendered.
    pub current: bool,
}

/// An `<link rel="alternate" hreflang>` entry (absolute URL).
#[derive(Debug, Serialize)]
pub struct HreflangAlternate {
    pub hreflang: String,
    pub href: String,
}

/// Site-relative path for a post in `lang`. The default language is served at
/// the root; others under a `/<lang>/` prefix. Category and slug are
/// URL-encoded to match sitemap/feed URL construction.
fn post_path(languages: &LanguagesConfig, lang: &str, category: &str, slug: &str) -> String {
    let category = encode_for_url(category);
    let slug = encode_for_url(slug);
    if languages.is_default(lang) {
        format!("/{}/{}/", category, slug)
    } else {
        format!("/{}/{}/{}/", lang, category, slug)
    }
}

/// Ordered language links for the switcher, one per available language
/// (including the current one so the UI can highlight it). Ordered by
/// `languages.supported` for a stable, configured sequence. Returns an empty
/// vec when the post has no translations (only one language available).
pub fn language_links(
    index: &TranslationIndex,
    languages: &LanguagesConfig,
    category: &str,
    slug: &str,
    current_lang: &str,
) -> Vec<LanguageLink> {
    let available = index.languages_for(category, slug);
    if available.len() < 2 {
        return Vec::new();
    }

    languages
        .supported
        .iter()
        .filter(|lang| available.iter().any(|a| a == *lang))
        .map(|lang| LanguageLink {
            lang: lang.clone(),
            label: lang_display_name(lang),
            url: post_path(languages, lang, category, slug),
            current: lang == current_lang,
        })
        .collect()
}

/// hreflang alternates for the post's `<head>`: every available language plus an
/// `x-default` pointing at the default language (when present). Absolute URLs.
/// Empty when the post has no translations.
pub fn hreflang_alternates(
    index: &TranslationIndex,
    languages: &LanguagesConfig,
    site_url: &str,
    category: &str,
    slug: &str,
) -> Vec<HreflangAlternate> {
    let available = index.languages_for(category, slug);
    if available.len() < 2 {
        return Vec::new();
    }

    let mut alternates: Vec<HreflangAlternate> = languages
        .supported
        .iter()
        .filter(|lang| available.iter().any(|a| a == *lang))
        .map(|lang| HreflangAlternate {
            hreflang: lang.clone(),
            href: format!("{}{}", site_url, post_path(languages, lang, category, slug)),
        })
        .collect();

    if available.iter().any(|a| a == &languages.default) {
        alternates.push(HreflangAlternate {
            hreflang: "x-default".to_string(),
            href: format!(
                "{}{}",
                site_url,
                post_path(languages, &languages.default, category, slug)
            ),
        });
    }

    alternates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn langs() -> LanguagesConfig {
        LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string()],
        }
    }

    #[test]
    fn test_record_is_idempotent() {
        let mut index = TranslationIndex::new();
        index.record("chat", "foo", "ko");
        index.record("chat", "foo", "ko");
        index.record("chat", "foo", "en");
        assert_eq!(index.languages_for("chat", "foo"), &["ko", "en"]);
    }

    #[test]
    fn test_no_links_without_translation() {
        let mut index = TranslationIndex::new();
        index.record("chat", "foo", "ko");
        assert!(language_links(&index, &langs(), "chat", "foo", "ko").is_empty());
        assert!(hreflang_alternates(&index, &langs(), "https://x.com", "chat", "foo").is_empty());
    }

    #[test]
    fn test_language_links_ordered_and_flagged() {
        let mut index = TranslationIndex::new();
        // Record out of config order to prove the output follows `supported`.
        index.record("chat", "foo", "en");
        index.record("chat", "foo", "ko");

        let links = language_links(&index, &langs(), "chat", "foo", "en");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].lang, "ko");
        assert_eq!(links[0].url, "/chat/foo/");
        assert!(!links[0].current);
        assert_eq!(links[1].lang, "en");
        assert_eq!(links[1].url, "/en/chat/foo/");
        assert!(links[1].current);
    }

    #[test]
    fn test_topology_fingerprint_ignores_monolingual_and_is_order_stable() {
        let mut a = TranslationIndex::new();
        a.record("chat", "foo", "ko");
        a.record("chat", "foo", "en");
        a.record("dev", "solo", "ko"); // monolingual — must not affect fingerprint

        let mut b = TranslationIndex::new();
        // Same multilingual topology, different insertion order, extra monolingual.
        b.record("chat", "foo", "en");
        b.record("chat", "foo", "ko");
        b.record("dev", "other-solo", "ko");

        assert_eq!(a.topology_fingerprint(), b.topology_fingerprint());
    }

    #[test]
    fn test_topology_fingerprint_changes_when_translation_added() {
        let mut before = TranslationIndex::new();
        before.record("chat", "foo", "ko");

        let mut after = before.clone();
        after.record("chat", "foo", "en");

        assert_ne!(before.topology_fingerprint(), after.topology_fingerprint());
    }

    #[test]
    fn test_hreflang_includes_x_default() {
        let mut index = TranslationIndex::new();
        index.record("chat", "foo", "ko");
        index.record("chat", "foo", "en");

        let alts = hreflang_alternates(&index, &langs(), "https://x.com", "chat", "foo");
        // ko, en, x-default
        assert_eq!(alts.len(), 3);
        assert_eq!(alts[0].hreflang, "ko");
        assert_eq!(alts[0].href, "https://x.com/chat/foo/");
        assert_eq!(alts[1].hreflang, "en");
        assert_eq!(alts[1].href, "https://x.com/en/chat/foo/");
        assert_eq!(alts[2].hreflang, "x-default");
        assert_eq!(alts[2].href, "https://x.com/chat/foo/");
    }
}
