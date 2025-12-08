use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::slug;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const RECENT_POSTS_COUNT: usize = 6;

#[derive(Debug, Serialize)]
pub struct RecentPost {
    pub title: String,
    pub uri: String,
    pub date: String,
    pub desc: String,
}

pub struct RecentGenerator {
    config: SsgConfig,
}

impl RecentGenerator {
    pub fn new(config: SsgConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self, metadata: &MetadataCache) -> Result<()> {
        println!("\n📋 Generating recent posts...");

        let mut filtered: Vec<_> = metadata
            .posts
            .iter()
            .filter(|p| !p.frontmatter.hidden)
            .collect();

        filtered.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let posts: Vec<RecentPost> = filtered
            .into_iter()
            .take(RECENT_POSTS_COUNT)
            .map(|post| {
                let uri = if self.config.build.encode_filenames {
                    format!(
                        "/{}/{}/",
                        slug::encode_for_url(&post.category),
                        slug::encode_for_url(&post.slug)
                    )
                } else {
                    format!("/{}/{}/", post.category, post.slug)
                };

                RecentPost {
                    title: post.frontmatter.title.clone(),
                    uri,
                    date: post.frontmatter.date.posted.to_rfc3339(),
                    desc: post.frontmatter.description.clone().unwrap_or_default(),
                }
            })
            .collect();

        let json = serde_json::to_string(&posts)?;
        let output_path = PathBuf::from(&self.config.build.output_dir).join("recent.json");
        fs::write(&output_path, json)?;

        println!("   ✓ {} recent posts generated", posts.len());

        Ok(())
    }
}
