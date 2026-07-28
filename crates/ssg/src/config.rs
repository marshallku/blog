use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Contact information for the site
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contacts {
    #[serde(default)]
    pub linkedin: Option<String>,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

/// Site configuration from config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    #[serde(default = "default_site_title")]
    pub title: String,
    #[serde(default = "default_site_url")]
    pub url: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default = "default_description")]
    pub description: String,
    /// CDN URL for image optimization (optional)
    #[serde(default)]
    pub cdn_url: Option<String>,
    /// API URL for backend services (optional)
    #[serde(default)]
    pub api_url: Option<String>,
    /// Google Analytics ID (optional)
    #[serde(default)]
    pub google_analytics_id: Option<String>,
    /// Contact information (optional)
    #[serde(default)]
    pub contacts: Contacts,
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Enable search index generation (default: true)
    #[serde(default = "default_search_enabled")]
    pub enabled: bool,
}

/// Multilingual configuration. The `default` language is served at the site
/// root (`/dev/foo/`); every other supported language is served under a
/// same-named path prefix (`/en/dev/foo/`) and authored as a co-located
/// `foo.<lang>.md` file next to the default `foo.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagesConfig {
    /// Language served at the site root (no URL prefix). Default: "ko".
    #[serde(default = "default_language")]
    pub default: String,
    /// All languages the site publishes, including the default. Default: ["ko"].
    #[serde(default = "default_supported_languages")]
    pub supported: Vec<String>,
}

impl Default for LanguagesConfig {
    fn default() -> Self {
        Self {
            default: default_language(),
            supported: default_supported_languages(),
        }
    }
}

impl LanguagesConfig {
    /// Validate that language codes are safe URL/path segments and internally
    /// consistent. Language codes become filesystem path components and URL
    /// prefixes, so an unchecked value like `..` could escape `dist`.
    pub fn validate(&self) -> Result<()> {
        let is_valid_code = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        };

        for lang in &self.supported {
            if !is_valid_code(lang) {
                anyhow::bail!(
                    "Invalid language code '{}': must be non-empty lowercase ascii, digits, or '-'",
                    lang
                );
            }
        }

        if !is_valid_code(&self.default) {
            anyhow::bail!("Invalid default language '{}'", self.default);
        }

        if !self.supported.iter().any(|l| l == &self.default) {
            anyhow::bail!(
                "Default language '{}' is not in the supported list {:?}",
                self.default,
                self.supported
            );
        }

        let mut seen = std::collections::HashSet::new();
        for lang in &self.supported {
            if !seen.insert(lang) {
                anyhow::bail!("Duplicate language '{}' in supported list", lang);
            }
        }

        Ok(())
    }

    pub fn is_default(&self, lang: &str) -> bool {
        lang == self.default
    }

    /// Supported languages other than the default, in config order.
    pub fn non_default(&self) -> impl Iterator<Item = &String> {
        self.supported.iter().filter(move |l| *l != &self.default)
    }
}

/// Map a language code to its Open Graph `og:locale` form. Known languages get
/// a full region tag; anything else falls back to the bare code.
pub fn lang_to_og_locale(lang: &str) -> String {
    match lang {
        "ko" => "ko_KR".to_string(),
        "en" => "en_US".to_string(),
        "ja" => "ja_JP".to_string(),
        other => other.to_string(),
    }
}

/// Human-readable, in-language name for a language code, used by the language
/// switcher. Falls back to the uppercased code for unknown languages.
pub fn lang_display_name(lang: &str) -> String {
    match lang {
        "ko" => "한국어".to_string(),
        "en" => "English".to_string(),
        "ja" => "日本語".to_string(),
        other => other.to_uppercase(),
    }
}

/// Assets configuration from manifest.json
/// Dynamic structure: { "package_name": { "asset_key": "path", ... }, ... }
/// Example: { "styles": { "version": "0.1.0", "theme": "/styles/0.1.0/theme.css" } }
/// Templates access via: {{ config.assets.styles.theme }}
pub type AssetsConfig = HashMap<String, HashMap<String, String>>;

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enabled: default_search_enabled(),
        }
    }
}

fn default_search_enabled() -> bool {
    true
}

/// Build configuration from config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_content_dir")]
    pub content_dir: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    /// Maximum number of page links to show in pagination (default: 5)
    /// Shows prev N/2, current, next N/2 pages
    #[serde(default = "default_pagination_window")]
    pub pagination_window: usize,
    /// Number of posts to show on the homepage (default: posts_per_page)
    #[serde(default)]
    pub homepage_posts_limit: Option<usize>,
    /// Percent-encode filenames for URL safety (default: false)
    /// Set to true for compatibility with older web servers
    #[serde(default)]
    pub encode_filenames: bool,
    /// Search index configuration
    #[serde(default)]
    pub search: SearchConfig,
    /// Generate partial HTML files for SPA navigation (default: false)
    #[serde(default)]
    pub generate_partials: bool,
    /// Directory name for partial files (default: "html")
    #[serde(default = "default_partial_dir")]
    pub partial_dir: String,
}

/// Complete config.yaml structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SsgConfig {
    #[serde(default)]
    pub site: SiteConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub assets: AssetsConfig,
    #[serde(default)]
    pub languages: LanguagesConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateConfig<'a> {
    pub site_title: &'a str,
    pub site_url: &'a str,
    pub author: &'a str,
    pub description: &'a str,
    pub assets: &'a AssetsConfig,
    pub api_url: Option<&'a str>,
    pub google_analytics_id: Option<&'a str>,
    pub contacts: &'a Contacts,
    /// Language served at the site root; the `lang`/`og_locale` fallback for
    /// templates (index/category/tag/pages) that don't inject a per-post value.
    pub default_language: &'a str,
    pub default_og_locale: String,
}

/// Top-level output path segments the generator reserves for structural pages,
/// independent of `partial_dir`. A non-default language code becomes a
/// top-level segment (`/en/...`) and must not collide with these.
const RESERVED_PATH_SEGMENTS: &[&str] = &["tag", "tags", "page"];

impl SsgConfig {
    /// Cross-section validation that `LanguagesConfig::validate` cannot do alone:
    /// a non-default language code becomes a top-level path segment, so it must
    /// not collide with the SPA partial directory or reserved listing segments,
    /// or one output would silently overwrite the other.
    pub fn validate(&self) -> Result<()> {
        self.languages.validate()?;

        for lang in self.languages.non_default() {
            if lang == &self.build.partial_dir {
                anyhow::bail!(
                    "Language code '{}' collides with build.partial_dir; translations would \
                     overwrite SPA partials",
                    lang
                );
            }
            if RESERVED_PATH_SEGMENTS.contains(&lang.as_str()) {
                anyhow::bail!(
                    "Language code '{}' collides with a reserved path segment {:?}",
                    lang,
                    RESERVED_PATH_SEGMENTS
                );
            }
        }

        Ok(())
    }

    /// Rejects a non-default language code that collides with a content category
    /// (top-level or the first segment of a nested one). Without this, a Korean
    /// post in an `en/...` category and an `<slug>.en.md` translation would both
    /// map to `/en/...` and silently overwrite each other. Categories are
    /// discovered at build time, so this runs separately from `validate`.
    pub fn validate_against_categories(&self, category_slugs: &[String]) -> Result<()> {
        for lang in self.languages.non_default() {
            let nested_prefix = format!("{}/", lang);
            for slug in category_slugs {
                if slug == lang || slug.starts_with(&nested_prefix) {
                    anyhow::bail!(
                        "Language code '{}' collides with content category '{}'; a translation and \
                         an '{}' category post would map to the same /{}/ output path",
                        lang,
                        slug,
                        lang,
                        lang
                    );
                }
            }
        }
        Ok(())
    }

    pub fn to_template_config(&self) -> TemplateConfig<'_> {
        TemplateConfig {
            site_title: &self.site.title,
            site_url: &self.site.url,
            author: &self.site.author,
            description: &self.site.description,
            assets: &self.assets,
            api_url: self.site.api_url.as_deref(),
            google_analytics_id: self.site.google_analytics_id.as_deref(),
            contacts: &self.site.contacts,
            default_language: &self.languages.default,
            default_og_locale: lang_to_og_locale(&self.languages.default),
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: default_site_title(),
            url: default_site_url(),
            author: default_author(),
            description: default_description(),
            cdn_url: None,
            api_url: None,
            google_analytics_id: None,
            contacts: Contacts::default(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            content_dir: default_content_dir(),
            output_dir: default_output_dir(),
            posts_per_page: default_posts_per_page(),
            pagination_window: default_pagination_window(),
            homepage_posts_limit: None,
            encode_filenames: false,
            search: SearchConfig::default(),
            generate_partials: false,
            partial_dir: default_partial_dir(),
        }
    }
}

fn default_site_title() -> String {
    "marshallku blog".to_string()
}

fn default_site_url() -> String {
    "https://marshallku.com".to_string()
}

fn default_author() -> String {
    "Marshall K".to_string()
}

fn default_description() -> String {
    "marshallku blog".to_string()
}

fn default_content_dir() -> String {
    "content/posts".to_string()
}

fn default_output_dir() -> String {
    "dist".to_string()
}

fn default_posts_per_page() -> usize {
    10
}

fn default_pagination_window() -> usize {
    5
}

fn default_partial_dir() -> String {
    "html".to_string()
}

fn default_language() -> String {
    "ko".to_string()
}

fn default_supported_languages() -> Vec<String> {
    vec![default_language()]
}

pub fn load_config() -> Result<SsgConfig> {
    let config_path = Path::new("config.yaml");

    let mut config = if config_path.exists() {
        let content = fs::read_to_string(config_path).context("Failed to read config.yaml")?;
        serde_yaml::from_str(&content).context("Failed to parse config.yaml")?
    } else {
        SsgConfig::default()
    };

    // Load manifest.json if it exists - directly deserialize as HashMap
    let manifest_path = Path::new("manifest.json");
    if manifest_path.exists() {
        let manifest_content =
            fs::read_to_string(manifest_path).context("Failed to read manifest.json")?;
        config.assets =
            serde_json::from_str(&manifest_content).context("Failed to parse manifest.json")?;
    }

    config
        .validate()
        .context("Invalid languages configuration")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SsgConfig::default();
        assert_eq!(config.site.title, "marshallku blog");
        assert_eq!(config.build.posts_per_page, 10);
        assert_eq!(config.languages.default, "ko");
        assert_eq!(config.languages.supported, vec!["ko".to_string()]);
    }

    #[test]
    fn test_languages_validate_ok() {
        let langs = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string()],
        };
        assert!(langs.validate().is_ok());
    }

    #[test]
    fn test_languages_default_must_be_supported() {
        let langs = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["en".to_string()],
        };
        assert!(langs.validate().is_err());
    }

    #[test]
    fn test_languages_reject_path_traversal() {
        let langs = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "..".to_string()],
        };
        assert!(langs.validate().is_err());
    }

    #[test]
    fn test_languages_reject_duplicates() {
        let langs = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string(), "en".to_string()],
        };
        assert!(langs.validate().is_err());
    }

    #[test]
    fn test_non_default_languages() {
        let langs = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string(), "ja".to_string()],
        };
        let non_default: Vec<_> = langs.non_default().cloned().collect();
        assert_eq!(non_default, vec!["en".to_string(), "ja".to_string()]);
    }

    #[test]
    fn test_lang_to_og_locale() {
        assert_eq!(lang_to_og_locale("ko"), "ko_KR");
        assert_eq!(lang_to_og_locale("en"), "en_US");
        assert_eq!(lang_to_og_locale("fr"), "fr");
    }

    #[test]
    fn test_config_rejects_language_colliding_with_partial_dir() {
        let mut config = SsgConfig::default();
        config.build.partial_dir = "html".to_string();
        config.languages = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "html".to_string()],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_rejects_language_colliding_with_reserved_segment() {
        let mut config = SsgConfig::default();
        config.languages = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "tag".to_string()],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_accepts_normal_languages() {
        let mut config = SsgConfig::default();
        config.languages = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string()],
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_against_categories_detects_collision() {
        let mut config = SsgConfig::default();
        config.languages = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string()],
        };
        // A top-level category named "en" collides with the /en/ prefix.
        assert!(config
            .validate_against_categories(&["en".to_string(), "dev".to_string()])
            .is_err());
        // A nested category "en/chat" also collides on its first segment.
        assert!(config
            .validate_against_categories(&["en/chat".to_string()])
            .is_err());
    }

    #[test]
    fn test_validate_against_categories_allows_disjoint() {
        let mut config = SsgConfig::default();
        config.languages = LanguagesConfig {
            default: "ko".to_string(),
            supported: vec!["ko".to_string(), "en".to_string()],
        };
        assert!(config
            .validate_against_categories(&[
                "dev".to_string(),
                "chat".to_string(),
                "gallery".to_string(),
            ])
            .is_ok());
    }
}
