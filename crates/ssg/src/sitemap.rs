use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::slug::encode_for_url;
use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct SitemapGenerator;

impl SitemapGenerator {
    pub fn generate(
        config: &SsgConfig,
        metadata: &MetadataCache,
        output_dir: &Path,
    ) -> Result<()> {
        let mut urls = Vec::new();

        Self::add_homepage(&mut urls, config);
        Self::add_posts(&mut urls, config, metadata);
        Self::add_categories(&mut urls, config, metadata);
        Self::add_tags(&mut urls, config, metadata);
        Self::add_pages(&mut urls, config);

        let sitemap_xml = Self::build_sitemap_xml(&urls);

        fs::create_dir_all(output_dir)?;
        let output_path = output_dir.join("sitemap.xml");
        fs::write(&output_path, sitemap_xml)?;

        Ok(())
    }

    fn add_homepage(urls: &mut Vec<SitemapUrl>, config: &SsgConfig) {
        urls.push(SitemapUrl {
            loc: config.site.url.clone(),
            lastmod: None,
            changefreq: Some("daily".to_string()),
            priority: Some(1.0),
        });
    }

    fn add_posts(urls: &mut Vec<SitemapUrl>, config: &SsgConfig, metadata: &MetadataCache) {
        for post in &metadata.posts {
            if post.frontmatter.hidden {
                continue;
            }

            let encoded_category = encode_for_url(&post.category);
            let encoded_slug = encode_for_url(&post.slug);
            let url = format!("{}/{}/{}/", config.site.url, encoded_category, encoded_slug);

            let lastmod = post
                .frontmatter
                .date
                .modified
                .as_ref()
                .unwrap_or(&post.frontmatter.date.posted)
                .to_rfc3339();

            urls.push(SitemapUrl {
                loc: url,
                lastmod: Some(lastmod),
                changefreq: Some("monthly".to_string()),
                priority: Some(0.8),
            });
        }
    }

    fn add_categories(urls: &mut Vec<SitemapUrl>, config: &SsgConfig, metadata: &MetadataCache) {
        let posts_per_page = config.build.posts_per_page;

        for category in metadata.get_category_info() {
            if category.hidden {
                continue;
            }

            let encoded_slug = encode_for_url(&category.slug);
            let category_url = format!("{}/{}/", config.site.url, encoded_slug);
            urls.push(SitemapUrl {
                loc: category_url,
                lastmod: None,
                changefreq: Some("weekly".to_string()),
                priority: Some(0.7),
            });

            let post_count = metadata
                .get_posts_by_category_tree(&category.slug)
                .into_iter()
                .filter(|p| !p.frontmatter.hidden)
                .count();
            let total_pages = (post_count + posts_per_page - 1) / posts_per_page;

            for page in 2..=total_pages {
                let page_url = format!("{}/{}/page/{}/", config.site.url, encoded_slug, page);
                urls.push(SitemapUrl {
                    loc: page_url,
                    lastmod: None,
                    changefreq: Some("weekly".to_string()),
                    priority: Some(0.5),
                });
            }
        }
    }

    fn add_tags(urls: &mut Vec<SitemapUrl>, config: &SsgConfig, metadata: &MetadataCache) {
        let posts_per_page = config.build.posts_per_page;

        urls.push(SitemapUrl {
            loc: format!("{}/tags/", config.site.url),
            lastmod: None,
            changefreq: Some("weekly".to_string()),
            priority: Some(0.6),
        });

        for tag in metadata.get_tags() {
            let encoded_tag = encode_for_url(&tag);
            let tag_url = format!("{}/tags/{}/", config.site.url, encoded_tag);
            urls.push(SitemapUrl {
                loc: tag_url,
                lastmod: None,
                changefreq: Some("weekly".to_string()),
                priority: Some(0.5),
            });

            let post_count = metadata
                .get_posts_by_tag(&tag)
                .into_iter()
                .filter(|p| !p.frontmatter.hidden)
                .count();
            let total_pages = (post_count + posts_per_page - 1) / posts_per_page;

            for page in 2..=total_pages {
                let page_url = format!("{}/tags/{}/page/{}/", config.site.url, encoded_tag, page);
                urls.push(SitemapUrl {
                    loc: page_url,
                    lastmod: None,
                    changefreq: Some("weekly".to_string()),
                    priority: Some(0.4),
                });
            }
        }
    }

    fn add_pages(urls: &mut Vec<SitemapUrl>, config: &SsgConfig) {
        let pages_dir = Path::new("content/pages");
        if !pages_dir.exists() {
            return;
        }

        for entry in WalkDir::new(pages_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        {
            let path = entry.path();

            if let Some(stem) = path.file_stem() {
                let slug = stem.to_string_lossy();
                let encoded_slug = encode_for_url(&slug);
                let page_url = format!("{}/{}/", config.site.url, encoded_slug);

                urls.push(SitemapUrl {
                    loc: page_url,
                    lastmod: None,
                    changefreq: Some("monthly".to_string()),
                    priority: Some(0.6),
                });
            }
        }
    }

    fn build_sitemap_xml(urls: &[SitemapUrl]) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
        );

        for url in urls {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", Self::escape_xml(&url.loc)));

            if let Some(ref lastmod) = url.lastmod {
                xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
            }

            if let Some(ref changefreq) = url.changefreq {
                xml.push_str(&format!("    <changefreq>{}</changefreq>\n", changefreq));
            }

            if let Some(priority) = url.priority {
                xml.push_str(&format!("    <priority>{:.1}</priority>\n", priority));
            }

            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>\n");
        xml
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

struct SitemapUrl {
    loc: String,
    lastmod: Option<String>,
    changefreq: Option<String>,
    priority: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BuildConfig, SiteConfig};
    use crate::types::{Category, Frontmatter, PostDate};
    use crate::metadata::PostMetadata;
    use chrono::Utc;

    fn create_test_config() -> SsgConfig {
        SsgConfig {
            site: SiteConfig {
                url: "https://example.com".to_string(),
                title: "Test Blog".to_string(),
                author: "Test Author".to_string(),
                description: "Test Description".to_string(),
                ..Default::default()
            },
            build: BuildConfig {
                posts_per_page: 10,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn create_test_post(slug: &str, category: &str, hidden: bool) -> PostMetadata {
        PostMetadata {
            slug: slug.to_string(),
            category: category.to_string(),
            frontmatter: Frontmatter {
                title: format!("Test Post {}", slug),
                date: PostDate::new(Utc::now()),
                tags: vec!["test".to_string()],
                cover_image: None,
                og_image: None,
                description: None,
                display_ad: false,
                hidden,
                comments: true,
            },
        }
    }

    fn create_test_category(slug: &str, hidden: bool) -> Category {
        Category {
            slug: slug.to_string(),
            name: slug.to_string(),
            description: String::new(),
            index: 0,
            hidden,
            icon: None,
            color: None,
            cover_image: None,
        }
    }

    #[test]
    fn test_escape_xml() {
        let input = "https://example.com/path?a=1&b=2";
        let expected = "https://example.com/path?a=1&amp;b=2";
        assert_eq!(SitemapGenerator::escape_xml(input), expected);
    }

    #[test]
    fn test_hidden_posts_excluded_from_sitemap() {
        let config = create_test_config();
        let mut metadata = MetadataCache::new();

        metadata.posts.push(create_test_post("visible-post", "dev", false));
        metadata.posts.push(create_test_post("hidden-post", "dev", true));
        metadata.posts.push(create_test_post("another-visible", "dev", false));

        let mut urls = Vec::new();
        SitemapGenerator::add_posts(&mut urls, &config, &metadata);

        assert_eq!(urls.len(), 2, "Should only include 2 visible posts");
        assert!(urls.iter().any(|u| u.loc.contains("visible-post")));
        assert!(urls.iter().any(|u| u.loc.contains("another-visible")));
        assert!(!urls.iter().any(|u| u.loc.contains("hidden-post")), "Hidden post should not be in sitemap");
    }

    #[test]
    fn test_hidden_categories_excluded_from_sitemap() {
        let config = create_test_config();
        let mut metadata = MetadataCache::new();

        metadata.category_info.push(create_test_category("visible-cat", false));
        metadata.category_info.push(create_test_category("hidden-cat", true));

        let mut urls = Vec::new();
        SitemapGenerator::add_categories(&mut urls, &config, &metadata);

        assert_eq!(urls.len(), 1, "Should only include 1 visible category");
        assert!(urls.iter().any(|u| u.loc.contains("visible-cat")));
        assert!(!urls.iter().any(|u| u.loc.contains("hidden-cat")), "Hidden category should not be in sitemap");
    }

    #[test]
    fn test_pagination_urls_calculated_correctly() {
        let mut config = create_test_config();
        config.build.posts_per_page = 5;

        let mut metadata = MetadataCache::new();
        metadata.category_info.push(create_test_category("dev", false));

        // Add 12 posts - should result in 3 pages (5 + 5 + 2)
        for i in 0..12 {
            let mut post = create_test_post(&format!("post-{}", i), "dev", false);
            post.frontmatter.tags = vec![];
            metadata.posts.push(post);
        }
        metadata.recalculate_stats();

        let mut urls = Vec::new();
        SitemapGenerator::add_categories(&mut urls, &config, &metadata);

        // Should have: 1 category index + 2 pagination pages (page/2, page/3)
        assert_eq!(urls.len(), 3, "Should have category + 2 pagination pages");
        assert!(urls.iter().any(|u| u.loc == "https://example.com/dev/"));
        assert!(urls.iter().any(|u| u.loc == "https://example.com/dev/page/2/"));
        assert!(urls.iter().any(|u| u.loc == "https://example.com/dev/page/3/"));
        assert!(!urls.iter().any(|u| u.loc.contains("page/1")), "Page 1 should not exist (it's the index)");
    }

    #[test]
    fn test_korean_urls_properly_encoded() {
        let config = create_test_config();
        let mut metadata = MetadataCache::new();

        metadata.posts.push(create_test_post("한글-포스트", "개발", false));

        let mut urls = Vec::new();
        SitemapGenerator::add_posts(&mut urls, &config, &metadata);

        assert_eq!(urls.len(), 1);
        let url = &urls[0].loc;
        assert!(url.contains("%"), "Korean characters should be percent-encoded");
        assert!(!url.contains("한"), "Korean characters should not appear literally");
    }

    #[test]
    fn test_empty_metadata_produces_valid_sitemap() {
        let config = create_test_config();
        let metadata = MetadataCache::new();

        let mut urls = Vec::new();
        SitemapGenerator::add_homepage(&mut urls, &config);
        SitemapGenerator::add_posts(&mut urls, &config, &metadata);
        SitemapGenerator::add_categories(&mut urls, &config, &metadata);
        SitemapGenerator::add_tags(&mut urls, &config, &metadata);

        // Should still have homepage + tags overview
        assert!(urls.len() >= 1, "Should at least have homepage");
        assert!(urls.iter().any(|u| u.loc == "https://example.com"));

        let xml = SitemapGenerator::build_sitemap_xml(&urls);
        assert!(xml.starts_with("<?xml version"));
        assert!(xml.contains("<urlset"));
        assert!(xml.contains("</urlset>"));
    }

    #[test]
    fn test_sitemap_xml_structure_valid() {
        let urls = vec![
            SitemapUrl {
                loc: "https://example.com/test/".to_string(),
                lastmod: Some("2025-01-01T00:00:00+00:00".to_string()),
                changefreq: Some("weekly".to_string()),
                priority: Some(0.8),
            },
        ];

        let xml = SitemapGenerator::build_sitemap_xml(&urls);

        assert!(xml.contains("<loc>https://example.com/test/</loc>"));
        assert!(xml.contains("<lastmod>2025-01-01T00:00:00+00:00</lastmod>"));
        assert!(xml.contains("<changefreq>weekly</changefreq>"));
        assert!(xml.contains("<priority>0.8</priority>"));
    }
}
