use crate::types::{Frontmatter, Page, PageFrontmatter, Post};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct Parser;

impl Parser {
    pub fn parse_file(path: &Path) -> Result<Post> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let (frontmatter_str, markdown) = Self::split_frontmatter(&content)?;
        let frontmatter = Self::parse_frontmatter(frontmatter_str)?;
        let slug = Self::path_to_slug(path)?;
        let category = Self::extract_category(path)?;

        Ok(Post {
            slug,
            category,
            frontmatter,
            content: markdown.to_string(),
            rendered_html: None,
        })
    }

    fn extract_category(path: &Path) -> Result<String> {
        let components: Vec<_> = path.components().collect();

        let mut posts_index = None;
        for (i, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(comp) = component {
                if *comp == "posts" {
                    posts_index = Some(i);
                    break;
                }
            }
        }

        let posts_idx = posts_index.ok_or_else(|| {
            anyhow::anyhow!(
                "Could not find 'posts' in path: {}. Expected format: content/posts/<category>/...",
                path.display()
            )
        })?;

        let category_parts: Vec<&str> = components[posts_idx + 1..components.len() - 1]
            .iter()
            .filter_map(|c| {
                if let std::path::Component::Normal(s) = c {
                    s.to_str()
                } else {
                    None
                }
            })
            .collect();

        if category_parts.is_empty() {
            anyhow::bail!(
                "Could not extract category from path: {}. Expected format: content/posts/<category>/...",
                path.display()
            )
        }

        Ok(category_parts.join("/"))
    }

    pub fn parse_page_file(path: &Path) -> Result<Page> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let slug = Self::path_to_slug(path)?;

        if content.trim_start().starts_with("---") {
            let (frontmatter_str, markdown) = Self::split_frontmatter(&content)?;
            let frontmatter = Self::parse_page_frontmatter(frontmatter_str)?;

            Ok(Page {
                slug,
                frontmatter,
                content: markdown.to_string(),
                rendered_html: None,
            })
        } else {
            Ok(Page {
                slug: slug.clone(),
                frontmatter: PageFrontmatter {
                    title: slug.replace('-', " "),
                    description: None,
                    hidden: false,
                    comments: true,
                    template: None,
                },
                content: content.to_string(),
                rendered_html: None,
            })
        }
    }

    fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();

        if parts.len() < 3 {
            anyhow::bail!("Invalid frontmatter format. Expected:\n---\nfrontmatter\n---\ncontent");
        }

        Ok((parts[1].trim(), parts[2].trim()))
    }

    fn parse_frontmatter(yaml: &str) -> Result<Frontmatter> {
        serde_yaml::from_str(yaml).context("Failed to parse frontmatter YAML")
    }

    fn parse_page_frontmatter(yaml: &str) -> Result<PageFrontmatter> {
        serde_yaml::from_str(yaml).context("Failed to parse page frontmatter YAML")
    }

    fn path_to_slug(path: &Path) -> Result<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Invalid file path: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_split_frontmatter() {
        let content = r#"---
title: Test Post
---
Content here"#;

        let (fm, content) = Parser::split_frontmatter(content).unwrap();
        assert!(fm.contains("title: Test Post"));
        assert_eq!(content, "Content here");
    }

    #[test]
    fn test_split_frontmatter_multiline() {
        let content = r#"---
title: Test
date: 2025-11-11T10:00:00Z
---
# Heading

Content with multiple lines"#;

        let (fm, content) = Parser::split_frontmatter(content).unwrap();
        assert!(fm.contains("title: Test"));
        assert!(content.starts_with("# Heading"));
    }

    #[test]
    fn test_path_to_slug() {
        let path = Path::new("content/posts/dev/hello-world.md");
        let slug = Parser::path_to_slug(path).unwrap();
        assert_eq!(slug, "hello-world");
    }

    #[test]
    fn test_parse_frontmatter_simple_date() {
        let yaml = r#"
title: Test Post
date: 2025-01-01T12:00:00Z
tags: []
"#;
        let fm = Parser::parse_frontmatter(yaml).unwrap();
        assert_eq!(fm.title, "Test Post");
        assert_eq!(fm.date.posted.year(), 2025);
        assert!(fm.date.modified.is_none());
    }

    #[test]
    fn test_parse_frontmatter_nested_date() {
        let yaml = r#"
title: Test Post
date:
  posted: 2025-01-01T12:00:00Z
  modified: 2025-01-15T14:30:00Z
tags: []
"#;
        let fm = Parser::parse_frontmatter(yaml).unwrap();
        assert_eq!(fm.title, "Test Post");
        assert_eq!(fm.date.posted.year(), 2025);
        assert!(fm.date.modified.is_some());
        assert_eq!(fm.date.modified.unwrap().day(), 15);
    }

    #[test]
    fn test_parse_frontmatter_missing_title_fails() {
        let yaml = r#"
date: 2025-01-01T12:00:00Z
tags: []
"#;
        let result = Parser::parse_frontmatter(yaml);
        assert!(result.is_err(), "Should fail when title is missing");
    }

    #[test]
    fn test_parse_frontmatter_missing_date_fails() {
        let yaml = r#"
title: Test Post
tags: []
"#;
        let result = Parser::parse_frontmatter(yaml);
        assert!(result.is_err(), "Should fail when date is missing");
    }

    #[test]
    fn test_parse_frontmatter_invalid_date_fails() {
        let yaml = r#"
title: Test Post
date: not-a-date
tags: []
"#;
        let result = Parser::parse_frontmatter(yaml);
        assert!(result.is_err(), "Should fail with invalid date format");
    }

    #[test]
    fn test_split_frontmatter_missing_closing_delimiter_fails() {
        let content = r#"---
title: Test Post
Content without closing delimiter"#;

        let result = Parser::split_frontmatter(content);
        assert!(result.is_err(), "Should fail without closing ---");
    }

    #[test]
    fn test_extract_category_nested() {
        let path = Path::new("content/posts/dev/rust/hello-world.md");
        let category = Parser::extract_category(path).unwrap();
        assert_eq!(category, "dev/rust");
    }

    #[test]
    fn test_extract_category_missing_posts_fails() {
        let path = Path::new("content/articles/dev/hello-world.md");
        let result = Parser::extract_category(path);
        assert!(result.is_err(), "Should fail when 'posts' not in path");
    }
}
