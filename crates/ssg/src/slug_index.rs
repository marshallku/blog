use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::slug;
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// A lookup entry for the client-side popular-posts widget: the view API only
/// knows slugs + counts, so the widget joins those counts to this index to get
/// titles/links/thumbnails without the API ever touching post content.
#[derive(Debug, Serialize)]
pub struct SlugIndexEntry {
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

pub struct SlugIndexGenerator {
    config: SsgConfig,
}

impl SlugIndexGenerator {
    pub fn new(config: SsgConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self, metadata: &MetadataCache) -> Result<()> {
        println!("\n🔑 Generating slug index...");

        // BTreeMap keeps the JSON key order stable across builds (determinism).
        let mut index: BTreeMap<String, SlugIndexEntry> = BTreeMap::new();

        for post in metadata.posts.iter().filter(|p| !p.frontmatter.hidden) {
            // Key matches the `postSlug` the browser sends to the view API:
            // `/{category}/{slug}` with raw (decoded) segments.
            let key = format!("/{}/{}", post.category, post.slug);

            let url = if self.config.build.encode_filenames {
                format!(
                    "/{}/{}/",
                    slug::encode_for_url(&post.category),
                    slug::encode_for_url(&post.slug)
                )
            } else {
                format!("/{}/{}/", post.category, post.slug)
            };

            let thumbnail = post
                .frontmatter
                .og_image
                .clone()
                .or_else(|| post.frontmatter.cover_image.clone());

            index.insert(
                key,
                SlugIndexEntry {
                    title: post.frontmatter.title.clone(),
                    url,
                    thumbnail,
                },
            );
        }

        let json = serde_json::to_string(&index)?;
        let output_path = PathBuf::from(&self.config.build.output_dir).join("slug-index.json");
        fs::write(&output_path, json)?;

        println!("   ✓ {} entries indexed", index.len());

        Ok(())
    }
}
