use crate::image::{ImageProcessor, ThumbnailMetadata};
use crate::metadata::{MetadataCache, PostMetadata};
use crate::slug;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct PostNavigation {
    pub prev: Option<PostLink>,
    pub next: Option<PostLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostLink {
    pub slug: String,
    pub title: String,
    pub url: String,
    pub category: String,
    pub cover_image: Option<String>,
    pub thumbnail_metadata: Option<ThumbnailMetadata>,
}

fn create_post_link(post: &PostMetadata) -> PostLink {
    PostLink {
        slug: post.slug.clone(),
        title: post.frontmatter.title.clone(),
        url: format!(
            "/{}/{}/",
            slug::encode_for_url(&post.category),
            slug::encode_for_url(&post.slug)
        ),
        category: post.category.clone(),
        cover_image: post
            .frontmatter
            .cover_image
            .clone()
            .or_else(|| post.frontmatter.og_image.clone()),
        thumbnail_metadata: None,
    }
}

fn create_post_link_with_cdn(
    post: &PostMetadata,
    image_processor: &ImageProcessor,
    content_dir: &Path,
) -> PostLink {
    let mut link = create_post_link(post);

    // Try to generate thumbnail metadata for cover image
    if link.cover_image.is_some() {
        // Get cover image path - it's already resolved to absolute path like /chat/slug/image.png
        let resolved_src = post
            .frontmatter
            .cover_image
            .as_ref()
            .or(post.frontmatter.og_image.as_ref());

        if let Some(src) = resolved_src {
            // Convert resolved path back to relative path for CDN processing
            // Resolved paths look like: /chat/post-slug/image.png
            // We need: ./post-slug/image.png relative to content_dir/category
            let relative_src = if src.starts_with('/') {
                // Strip leading slash and category prefix
                let without_leading_slash = src.trim_start_matches('/');
                // The path is: category/rest-of-path, we need ./rest-of-path
                if let Some(rest) = without_leading_slash.strip_prefix(&post.category) {
                    format!(".{}", rest)
                } else {
                    // Fallback: use path as-is with ./ prefix
                    format!("./{}", without_leading_slash)
                }
            } else {
                src.clone()
            };

            let post_content_dir = content_dir.join(&post.category);
            let base_path = post.category.clone();

            if let Ok(Some(metadata)) =
                image_processor.process_thumbnail(&relative_src, &post_content_dir, &base_path)
            {
                link.thumbnail_metadata = Some(metadata);
            }
        }
    }

    link
}

pub fn build_post_navigation(
    current_slug: &str,
    current_category: &str,
    metadata: &MetadataCache,
    same_category: bool,
) -> PostNavigation {
    let mut posts: Vec<_> = metadata
        .posts
        .iter()
        .filter(|p| !same_category || p.category == current_category)
        .collect();

    posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

    let Some(index) = posts.iter().position(|p| p.slug == current_slug) else {
        return PostNavigation {
            prev: None,
            next: None,
        };
    };

    let prev = posts.get(index + 1).map(|p| create_post_link(p));
    let next = if index > 0 {
        posts.get(index - 1).map(|p| create_post_link(p))
    } else {
        None
    };

    PostNavigation { prev, next }
}

pub fn build_post_navigation_with_cdn(
    current_slug: &str,
    current_category: &str,
    metadata: &MetadataCache,
    same_category: bool,
    image_processor: &ImageProcessor,
    content_dir: &Path,
) -> PostNavigation {
    let mut posts: Vec<_> = metadata
        .posts
        .iter()
        .filter(|p| !same_category || p.category == current_category)
        .collect();

    posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

    let Some(index) = posts.iter().position(|p| p.slug == current_slug) else {
        return PostNavigation {
            prev: None,
            next: None,
        };
    };

    let prev = posts
        .get(index + 1)
        .map(|p| create_post_link_with_cdn(p, image_processor, content_dir));
    let next = if index > 0 {
        posts
            .get(index - 1)
            .map(|p| create_post_link_with_cdn(p, image_processor, content_dir))
    } else {
        None
    };

    PostNavigation { prev, next }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Frontmatter, PostDate};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_metadata() -> MetadataCache {
        let base_date = Utc::now();

        let posts = vec![
            PostMetadata {
                slug: "post-1".to_string(),
                category: "dev".to_string(),
                frontmatter: Frontmatter {
                    title: "Post 1".to_string(),
                    date: PostDate {
                        posted: base_date - chrono::Duration::days(2),
                        modified: None,
                    },
                    tags: vec![],
                    description: None,
                    cover_image: None,
                    og_image: None,
                    display_ad: false,
                    hidden: false,
                    comments: true,
                },
            },
            PostMetadata {
                slug: "post-2".to_string(),
                category: "dev".to_string(),
                frontmatter: Frontmatter {
                    title: "Post 2".to_string(),
                    date: PostDate {
                        posted: base_date - chrono::Duration::days(1),
                        modified: None,
                    },
                    tags: vec![],
                    description: None,
                    cover_image: None,
                    og_image: None,
                    display_ad: false,
                    hidden: false,
                    comments: true,
                },
            },
            PostMetadata {
                slug: "post-3".to_string(),
                category: "dev".to_string(),
                frontmatter: Frontmatter {
                    title: "Post 3".to_string(),
                    date: PostDate {
                        posted: base_date,
                        modified: None,
                    },
                    tags: vec![],
                    description: None,
                    cover_image: None,
                    og_image: None,
                    display_ad: false,
                    hidden: false,
                    comments: true,
                },
            },
        ];

        MetadataCache {
            version: "1".to_string(),
            posts,
            categories: HashMap::new(),
            tags: HashMap::new(),
            category_info: vec![],
        }
    }

    #[test]
    fn test_navigation_middle_post() {
        let metadata = create_test_metadata();
        let nav = build_post_navigation("post-2", "dev", &metadata, true);

        assert!(nav.prev.is_some());
        assert_eq!(nav.prev.as_ref().unwrap().slug, "post-1");

        assert!(nav.next.is_some());
        assert_eq!(nav.next.as_ref().unwrap().slug, "post-3");
    }

    #[test]
    fn test_navigation_first_post() {
        let metadata = create_test_metadata();
        let nav = build_post_navigation("post-3", "dev", &metadata, true);

        assert!(nav.prev.is_some());
        assert_eq!(nav.prev.as_ref().unwrap().slug, "post-2");
        assert!(nav.next.is_none());
    }

    #[test]
    fn test_navigation_last_post() {
        let metadata = create_test_metadata();
        let nav = build_post_navigation("post-1", "dev", &metadata, true);

        assert!(nav.prev.is_none());
        assert!(nav.next.is_some());
        assert_eq!(nav.next.as_ref().unwrap().slug, "post-2");
    }

    #[test]
    fn test_navigation_url_encoding() {
        let metadata = create_test_metadata();
        let nav = build_post_navigation("post-2", "dev", &metadata, true);

        assert!(nav.prev.is_some());
        assert_eq!(nav.prev.as_ref().unwrap().url, "/dev/post-1/");
    }
}
