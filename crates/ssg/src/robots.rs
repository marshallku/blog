use crate::config::SsgConfig;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub struct RobotsGenerator;

impl RobotsGenerator {
    pub fn generate(config: &SsgConfig, output_dir: &Path) -> Result<()> {
        let sitemap_url = format!("{}/sitemap.xml", config.site.url);

        let robots_txt = format!(
            "User-agent: *\n\
             Allow: /\n\
             \n\
             Sitemap: {}\n",
            sitemap_url
        );

        fs::create_dir_all(output_dir)?;
        let output_path = output_dir.join("robots.txt");
        fs::write(&output_path, robots_txt)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BuildConfig, SiteConfig};

    #[test]
    fn test_robots_txt_contains_sitemap() {
        let config = SsgConfig {
            site: SiteConfig {
                url: "https://example.com".to_string(),
                ..Default::default()
            },
            build: BuildConfig::default(),
            ..Default::default()
        };

        let temp_dir = std::env::temp_dir().join("robots_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        RobotsGenerator::generate(&config, &temp_dir).unwrap();

        let content = fs::read_to_string(temp_dir.join("robots.txt")).unwrap();
        assert!(content.contains("User-agent: *"));
        assert!(content.contains("Allow: /"));
        assert!(content.contains("Sitemap: https://example.com/sitemap.xml"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
