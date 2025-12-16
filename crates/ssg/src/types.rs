use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PostDate {
    pub posted: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
}

impl PostDate {
    pub fn new(posted: DateTime<Utc>) -> Self {
        Self {
            posted,
            modified: None,
        }
    }

    pub fn to_rfc2822(&self) -> String {
        self.posted.to_rfc2822()
    }
}

fn deserialize_post_date<'de, D>(deserializer: D) -> Result<PostDate, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateFormat {
        Simple(DateTime<Utc>),
        Nested {
            posted: DateTime<Utc>,
            modified: Option<DateTime<Utc>>,
        },
    }

    let date_format = DateFormat::deserialize(deserializer)?;
    Ok(match date_format {
        DateFormat::Simple(dt) => PostDate::new(dt),
        DateFormat::Nested { posted, modified } => PostDate { posted, modified },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    #[serde(deserialize_with = "deserialize_post_date")]
    pub date: PostDate,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(alias = "coverImage", skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    #[serde(alias = "ogImage", skip_serializing_if = "Option::is_none")]
    pub og_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(alias = "displayAd", default)]
    pub display_ad: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default = "default_true")]
    pub comments: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub slug: String,
    pub category: String,
    pub frontmatter: Frontmatter,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    /// URL slug (same as directory name)
    #[serde(default)]
    pub slug: String,

    /// Display name (from .category.yaml or capitalized slug)
    #[serde(default)]
    pub name: String,

    /// Optional description
    #[serde(default)]
    pub description: String,

    /// Sort order (lower = first)
    #[serde(default = "default_category_index")]
    pub index: i32,

    /// Hide from navigation
    #[serde(default)]
    pub hidden: bool,

    /// Optional icon identifier
    #[serde(default)]
    pub icon: Option<String>,

    /// Optional color hex code
    #[serde(default)]
    pub color: Option<String>,

    /// Optional cover image path
    #[serde(default)]
    pub cover_image: Option<String>,
}

fn default_category_index() -> i32 {
    999
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFrontmatter {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default = "default_true")]
    pub comments: bool,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub slug: String,
    pub frontmatter: PageFrontmatter,
    pub content: String,
    pub rendered_html: Option<String>,
}
