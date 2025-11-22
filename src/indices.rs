use crate::config::SsgConfig;
use crate::metadata::{MetadataCache, PostMetadata};
use crate::plugin::{PluginContext, PluginManager};
use crate::slug;
use crate::types::Category;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera, Value};

/// Pagination context for templates
#[derive(Debug, Clone, Serialize)]
struct PaginationContext {
    current_page: usize,
    total_pages: usize,
    total_posts: usize,
    posts_per_page: usize,
    has_prev: bool,
    has_next: bool,
    prev_url: Option<String>,
    next_url: Option<String>,
    first_url: String,
    last_url: String,
    jump_prev_url: Option<String>,
    jump_next_url: Option<String>,
    pages: Vec<PageLink>,
}

/// Individual page link for navigation
#[derive(Debug, Clone, Serialize)]
struct PageLink {
    number: usize,
    url: String,
    is_current: bool,
}

/// Flattened config for template context (backward compatibility)
#[derive(Debug, Clone, Serialize)]
struct TemplateConfig<'a> {
    site_title: &'a str,
    site_url: &'a str,
    author: &'a str,
    description: &'a str,
}

/// Category with its recent posts for homepage tabs
#[derive(Debug, Clone, Serialize)]
struct CategoryPosts<'a> {
    category: &'a Category,
    posts: Vec<&'a PostMetadata>,
}

pub struct IndexGenerator {
    tera: Tera,
    config: SsgConfig,
}

impl IndexGenerator {
    pub fn new(config: SsgConfig) -> Result<Self> {
        let tera = create_tera_engine()?;

        Ok(Self { tera, config })
    }

    pub fn generate_all(
        &self,
        metadata: &MetadataCache,
        plugin_manager: &PluginManager,
    ) -> Result<()> {
        println!("\n📑 Generating indices...");

        let plugin_ctx = PluginContext {
            config: &self.config,
            metadata,
        };
        let plugin_data = plugin_manager.template_context_index(&plugin_ctx)?;

        self.generate_homepage(metadata, &plugin_data)?;

        let category_count = metadata.get_category_info().len();
        for category in metadata.get_category_info() {
            self.generate_category_page(category, metadata, &plugin_data)?;
        }

        for tag in metadata.get_tags() {
            self.generate_tag_page(&tag, metadata, &plugin_data)?;
        }

        self.generate_tags_overview(metadata, &plugin_data)?;

        println!("   ✓ Homepage");
        println!("   ✓ {} category pages", category_count);
        println!("   ✓ {} tag pages", metadata.get_tags().len());

        Ok(())
    }

    fn generate_homepage(
        &self,
        metadata: &MetadataCache,
        plugin_data: &HashMap<String, JsonValue>,
    ) -> Result<()> {
        let posts_limit = self
            .config
            .build
            .homepage_posts_limit
            .unwrap_or(self.config.build.posts_per_page);

        let mut visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();
        visible_categories.sort_by_key(|c| c.index);

        let visible_category_slugs: HashSet<_> =
            visible_categories.iter().map(|c| c.slug.as_str()).collect();

        let all_recent_posts: Vec<_> = metadata
            .get_recent_posts(posts_limit)
            .into_iter()
            .filter(|p| visible_category_slugs.contains(p.category.as_str()))
            .collect();

        let category_posts: Vec<CategoryPosts> = visible_categories
            .iter()
            .map(|cat| {
                let mut posts = metadata.get_posts_by_category(&cat.slug);
                posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));
                CategoryPosts {
                    category: cat,
                    posts: posts.into_iter().take(posts_limit).collect(),
                }
            })
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
            description: &self.config.site.description,
        };

        let mut context = TeraContext::new();
        context.insert("posts", &all_recent_posts);
        context.insert("category_posts", &category_posts);
        context.insert("categories", &visible_categories);
        context.insert("config", &template_config);

        for (key, value) in plugin_data {
            context.insert(key, value);
        }

        let output = self.tera.render("index.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir).join("index.html");

        fs::write(&output_path, output)?;

        Ok(())
    }

    fn generate_category_page(
        &self,
        category_info: &crate::types::Category,
        metadata: &MetadataCache,
        plugin_data: &HashMap<String, JsonValue>,
    ) -> Result<()> {
        let mut posts = metadata.get_posts_by_category_tree(&category_info.slug);

        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let total_posts = posts.len();
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            (total_posts + posts_per_page - 1) / posts_per_page
        };

        let base_url = format!("/{}/", category_info.slug);

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
            description: &self.config.site.description,
        };

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts[start_idx..end_idx];

            let mut context = TeraContext::new();
            context.insert("category", category_info);
            context.insert("posts", &page_posts);
            context.insert("post_count", &total_posts);
            context.insert("categories", &visible_categories);
            context.insert("config", &template_config);

            if total_pages > 1 {
                let pagination = self.build_pagination_context(page_num, total_posts, &base_url);
                context.insert("pagination", &pagination);
            }

            for (key, value) in plugin_data {
                context.insert(key, value);
            }

            let output = self.tera.render("category.html", &context)?;

            let category_slug = self.maybe_encode(&category_info.slug);

            let output_path = if page_num == 1 {
                PathBuf::from(&self.config.build.output_dir)
                    .join(&category_slug)
                    .join("index.html")
            } else {
                PathBuf::from(&self.config.build.output_dir)
                    .join(&category_slug)
                    .join("page")
                    .join(page_num.to_string())
                    .join("index.html")
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        Ok(())
    }

    fn generate_tag_page(
        &self,
        tag: &str,
        metadata: &MetadataCache,
        plugin_data: &HashMap<String, JsonValue>,
    ) -> Result<()> {
        let mut posts = metadata.get_posts_by_tag(tag);

        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let total_posts = posts.len();
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            (total_posts + posts_per_page - 1) / posts_per_page
        };

        let base_url = format!("/tag/{}/", tag);

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
            description: &self.config.site.description,
        };

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts[start_idx..end_idx];

            let mut context = TeraContext::new();
            context.insert("tag", tag);
            context.insert("posts", &page_posts);
            context.insert("post_count", &total_posts);
            context.insert("categories", &visible_categories);
            context.insert("config", &template_config);

            if total_pages > 1 {
                let pagination = self.build_pagination_context(page_num, total_posts, &base_url);
                context.insert("pagination", &pagination);
            }

            for (key, value) in plugin_data {
                context.insert(key, value);
            }

            let output = self.tera.render("tag.html", &context)?;

            let encoded_tag = self.maybe_encode(tag);

            let output_path = if page_num == 1 {
                PathBuf::from(&self.config.build.output_dir)
                    .join("tag")
                    .join(&encoded_tag)
                    .join("index.html")
            } else {
                PathBuf::from(&self.config.build.output_dir)
                    .join("tag")
                    .join(&encoded_tag)
                    .join("page")
                    .join(page_num.to_string())
                    .join("index.html")
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        Ok(())
    }

    fn generate_tags_overview(
        &self,
        metadata: &MetadataCache,
        plugin_data: &HashMap<String, JsonValue>,
    ) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1));

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
            description: &self.config.site.description,
        };

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("categories", &visible_categories);
        context.insert("config", &template_config);

        for (key, value) in plugin_data {
            context.insert(key, value);
        }

        let output = self.tera.render("tags.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir)
            .join("tags")
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    fn build_pagination_context(
        &self,
        current_page: usize,
        total_posts: usize,
        base_url: &str,
    ) -> PaginationContext {
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            (total_posts + posts_per_page - 1) / posts_per_page
        };

        let first_url = base_url.to_string();
        let last_url = if total_pages == 1 {
            base_url.to_string()
        } else {
            format!("{}page/{}", base_url, total_pages)
        };

        let window = self.config.build.pagination_window;
        let half_window = window / 2;

        let (start_page, end_page) = if total_pages <= window {
            (1, total_pages)
        } else if current_page <= half_window + 1 {
            (1, window)
        } else if current_page >= total_pages - half_window {
            (total_pages - window + 1, total_pages)
        } else {
            (current_page - half_window, current_page + half_window)
        };

        let pages = (start_page..=end_page)
            .map(|num| PageLink {
                number: num,
                url: if num == 1 {
                    base_url.to_string()
                } else {
                    format!("{}page/{}", base_url, num)
                },
                is_current: num == current_page,
            })
            .collect();

        let jump_prev_url = if start_page > 1 {
            let jump_page = start_page - 1;
            Some(if jump_page == 1 {
                base_url.to_string()
            } else {
                format!("{}page/{}", base_url, jump_page)
            })
        } else {
            None
        };

        let jump_next_url = if end_page < total_pages {
            Some(format!("{}page/{}", base_url, end_page + 1))
        } else {
            None
        };

        let prev_url = jump_prev_url.clone().or_else(|| {
            if current_page > 1 {
                Some(if current_page == 2 {
                    base_url.to_string()
                } else {
                    format!("{}page/{}", base_url, current_page - 1)
                })
            } else {
                None
            }
        });
        let next_url = jump_next_url.clone().or_else(|| {
            if current_page < total_pages {
                Some(format!("{}page/{}", base_url, current_page + 1))
            } else {
                None
            }
        });
        let has_prev = prev_url.is_some();
        let has_next = next_url.is_some();

        PaginationContext {
            current_page,
            total_pages,
            total_posts,
            posts_per_page,
            has_prev,
            has_next,
            prev_url,
            next_url,
            first_url,
            last_url,
            jump_prev_url,
            jump_next_url,
            pages,
        }
    }

    fn maybe_encode(&self, s: &str) -> String {
        if self.config.build.encode_filenames {
            slug::encode_for_url(s)
        } else {
            s.to_string()
        }
    }
}

fn create_tera_engine() -> Result<Tera> {
    let template_dir = Path::new("templates");

    if !template_dir.exists() {
        anyhow::bail!(
            "Templates directory not found. Expected templates at {:?}",
            template_dir
        );
    }

    let glob_pattern = format!("{}/**/*.html", template_dir.display());
    let mut tera = Tera::new(&glob_pattern)
        .context(format!("Failed to load templates from {:?}", template_dir))?;

    tera.register_filter("urldecode", urldecode_filter);

    Ok(tera)
}

fn urldecode_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = tera::try_get_value!("urldecode", "value", String, value);
    let decoded = slug::decode_from_url(&s);
    Ok(Value::String(decoded))
}
