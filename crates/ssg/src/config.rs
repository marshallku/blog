use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Enable search index generation (default: true)
    #[serde(default = "default_search_enabled")]
    pub enabled: bool,
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
}

impl SsgConfig {
    pub fn to_template_config(&self) -> TemplateConfig<'_> {
        TemplateConfig {
            site_title: &self.site.title,
            site_url: &self.site.url,
            author: &self.site.author,
            description: &self.site.description,
            assets: &self.assets,
            api_url: self.site.api_url.as_deref(),
            google_analytics_id: self.site.google_analytics_id.as_deref(),
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
    }
}
