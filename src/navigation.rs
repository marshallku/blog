use crate::metadata::{MetadataCache, PostMetadata};
use crate::slug;
use serde::Serialize;

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
    }
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
