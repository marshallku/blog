use crate::config::SsgConfig;
use crate::image::{ImageProcessor, ThumbnailMetadata};
use crate::metadata::{compare_posts_desc, MetadataCache, PostMetadata};
use crate::slug;
use crate::types::Category;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera, Value};

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

#[derive(Debug, Clone, Serialize)]
struct PageLink {
    number: usize,
    url: String,
    is_current: bool,
}

/// Category with its recent posts for homepage tabs
#[derive(Debug, Clone, Serialize)]
struct CategoryPosts<'a> {
    category: &'a Category,
    posts: Vec<PostCardData<'a>>,
}

/// Post data with CDN thumbnail metadata for post cards
#[derive(Debug, Clone, Serialize)]
struct PostCardData<'a> {
    #[serde(flatten)]
    post: &'a PostMetadata,
    thumbnail_metadata: Option<ThumbnailMetadata>,
}

pub struct IndexGenerator {
    tera: Tera,
    config: SsgConfig,
    image_processor: Option<ImageProcessor>,
    content_dir: PathBuf,
}

impl IndexGenerator {
    pub fn new(config: SsgConfig) -> Result<Self> {
        let tera = create_tera_engine()?;

        let image_processor = config
            .site
            .cdn_url
            .as_ref()
            .map(|url| ImageProcessor::new(Some(url.clone())));
        let content_dir = PathBuf::from(&config.build.content_dir);

        Ok(Self {
            tera,
            config,
            image_processor,
            content_dir,
        })
    }

    fn create_post_card_data<'a>(&self, post: &'a PostMetadata) -> PostCardData<'a> {
        let thumbnail_metadata = self.image_processor.as_ref().and_then(|processor| {
            let cover_src = post
                .frontmatter
                .cover_image
                .as_ref()
                .or(post.frontmatter.og_image.as_ref())?;

            let relative_src = if cover_src.starts_with('/') {
                let without_leading_slash = cover_src.trim_start_matches('/');
                if let Some(rest) = without_leading_slash.strip_prefix(&post.category) {
                    format!(".{}", rest)
                } else {
                    format!("./{}", without_leading_slash)
                }
            } else {
                cover_src.clone()
            };

            let post_content_dir = self.content_dir.join(&post.category);
            let base_path = post.category.clone();

            processor
                .process_thumbnail(&relative_src, &post_content_dir, &base_path)
                .ok()
                .flatten()
        });

        PostCardData {
            post,
            thumbnail_metadata,
        }
    }

    pub fn generate_all(&self, metadata: &MetadataCache) -> Result<()> {
        println!("\n📑 Generating indices...");

        self.generate_homepage(metadata)?;

        let category_count = metadata.get_category_info().len();
        for category in metadata.get_category_info() {
            self.generate_category_page(category, metadata)?;
        }

        for tag in metadata.get_tags() {
            self.generate_tag_page(&tag, metadata)?;
        }

        self.remove_stale_tag_dirs(
            &PathBuf::from(&self.config.build.output_dir).join("tag"),
            metadata,
        );

        self.generate_tags_overview(metadata)?;

        println!("   ✓ Homepage");
        println!("   ✓ {} category pages", category_count);
        println!("   ✓ {} tag pages", metadata.get_tags().len());

        Ok(())
    }

    pub fn generate_all_partials(&self, metadata: &MetadataCache) -> Result<()> {
        if !self.config.build.generate_partials {
            return Ok(());
        }

        println!("\n📄 Generating index partials...");

        self.generate_homepage_partial(metadata)?;

        let category_count = metadata.get_category_info().len();
        for category in metadata.get_category_info() {
            self.generate_category_partial(category, metadata)?;
        }

        for tag in metadata.get_tags() {
            self.generate_tag_partial(&tag, metadata)?;
        }

        self.remove_stale_tag_dirs(&self.get_partial_path("tag"), metadata);

        self.generate_tags_overview_partial(metadata)?;

        println!("   ✓ Homepage partial");
        println!("   ✓ {} category partials", category_count);
        println!("   ✓ {} tag partials", metadata.get_tags().len());

        Ok(())
    }

    fn generate_homepage(&self, metadata: &MetadataCache) -> Result<()> {
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
            .map(|p| self.create_post_card_data(p))
            .collect();

        let category_posts: Vec<CategoryPosts> = visible_categories
            .iter()
            .map(|cat| {
                let mut posts = metadata.get_posts_by_category(&cat.slug);
                posts.sort_by(|a, b| compare_posts_desc(a, b));
                CategoryPosts {
                    category: cat,
                    posts: posts
                        .into_iter()
                        .take(posts_limit)
                        .map(|p| self.create_post_card_data(p))
                        .collect(),
                }
            })
            .collect();

        let mut context = TeraContext::new();
        context.insert("posts", &all_recent_posts);
        context.insert("category_posts", &category_posts);
        context.insert("categories", &visible_categories);
        context.insert("config", &self.config.to_template_config());

        let output = self.tera.render("index.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir).join("index.html");

        fs::write(&output_path, output)?;

        Ok(())
    }

    fn generate_category_page(
        &self,
        category_info: &crate::types::Category,
        metadata: &MetadataCache,
    ) -> Result<()> {
        let mut posts = metadata.get_posts_by_category_tree(&category_info.slug);

        posts.sort_by(|a, b| compare_posts_desc(a, b));

        let posts_with_thumbnails: Vec<_> = posts
            .iter()
            .map(|p| self.create_post_card_data(p))
            .collect();

        let total_posts = posts_with_thumbnails.len();
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            total_posts.div_ceil(posts_per_page)
        };

        let base_url = format!("/{}/", category_info.slug);

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = self.config.to_template_config();

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts_with_thumbnails[start_idx..end_idx];

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

        let section_dir = PathBuf::from(&self.config.build.output_dir)
            .join(self.maybe_encode(&category_info.slug));
        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tag_page(&self, tag: &str, metadata: &MetadataCache) -> Result<()> {
        let mut posts = metadata.get_posts_by_tag(tag);

        posts.sort_by(|a, b| compare_posts_desc(a, b));

        let posts_with_thumbnails: Vec<_> = posts
            .iter()
            .map(|p| self.create_post_card_data(p))
            .collect();

        let total_posts = posts_with_thumbnails.len();
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            total_posts.div_ceil(posts_per_page)
        };

        let base_url = format!("/tag/{}/", tag);

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = self.config.to_template_config();

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts_with_thumbnails[start_idx..end_idx];

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

        let section_dir = PathBuf::from(&self.config.build.output_dir)
            .join("tag")
            .join(self.maybe_encode(tag));
        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tags_overview(&self, metadata: &MetadataCache) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("categories", &visible_categories);
        context.insert("config", &self.config.to_template_config());

        let output = self.tera.render("tags.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir)
            .join("tags")
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    fn generate_homepage_partial(&self, metadata: &MetadataCache) -> Result<()> {
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
            .map(|p| self.create_post_card_data(p))
            .collect();

        let category_posts: Vec<CategoryPosts> = visible_categories
            .iter()
            .map(|cat| {
                let mut posts = metadata.get_posts_by_category(&cat.slug);
                posts.sort_by(|a, b| compare_posts_desc(a, b));
                CategoryPosts {
                    category: cat,
                    posts: posts
                        .into_iter()
                        .take(posts_limit)
                        .map(|p| self.create_post_card_data(p))
                        .collect(),
                }
            })
            .collect();

        let mut context = TeraContext::new();
        context.insert("posts", &all_recent_posts);
        context.insert("category_posts", &category_posts);
        context.insert("categories", &visible_categories);
        context.insert("config", &self.config.to_template_config());

        let output = self.tera.render("partials/index.html", &context)?;
        let output_path = self.get_partial_path("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    fn generate_category_partial(
        &self,
        category_info: &crate::types::Category,
        metadata: &MetadataCache,
    ) -> Result<()> {
        let mut posts = metadata.get_posts_by_category_tree(&category_info.slug);
        posts.sort_by(|a, b| compare_posts_desc(a, b));

        let posts_with_thumbnails: Vec<_> = posts
            .iter()
            .map(|p| self.create_post_card_data(p))
            .collect();

        let total_posts = posts_with_thumbnails.len();
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            total_posts.div_ceil(posts_per_page)
        };

        let base_url = format!("/{}/", category_info.slug);

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = self.config.to_template_config();

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts_with_thumbnails[start_idx..end_idx];

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

            let output = self.tera.render("partials/category.html", &context)?;

            let category_slug = self.maybe_encode(&category_info.slug);

            let output_path = if page_num == 1 {
                self.get_partial_path(&format!("{}/index.html", category_slug))
            } else {
                self.get_partial_path(&format!("{}/page/{}/index.html", category_slug, page_num))
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        let section_dir = self.get_partial_path(&self.maybe_encode(&category_info.slug));
        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tag_partial(&self, tag: &str, metadata: &MetadataCache) -> Result<()> {
        let mut posts = metadata.get_posts_by_tag(tag);
        posts.sort_by(|a, b| compare_posts_desc(a, b));

        let posts_with_thumbnails: Vec<_> = posts
            .iter()
            .map(|p| self.create_post_card_data(p))
            .collect();

        let total_posts = posts_with_thumbnails.len();
        let posts_per_page = self.config.build.posts_per_page;
        let total_pages = if total_posts == 0 {
            1
        } else {
            total_posts.div_ceil(posts_per_page)
        };

        let base_url = format!("/tag/{}/", tag);

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = self.config.to_template_config();

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts_with_thumbnails[start_idx..end_idx];

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

            let output = self.tera.render("partials/tag.html", &context)?;

            let encoded_tag = self.maybe_encode(tag);

            let output_path = if page_num == 1 {
                self.get_partial_path(&format!("tag/{}/index.html", encoded_tag))
            } else {
                self.get_partial_path(&format!("tag/{}/page/{}/index.html", encoded_tag, page_num))
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        let section_dir = self.get_partial_path(&format!("tag/{}", self.maybe_encode(tag)));
        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tags_overview_partial(&self, metadata: &MetadataCache) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("categories", &visible_categories);
        context.insert("config", &self.config.to_template_config());

        let output = self.tera.render("partials/tags.html", &context)?;
        let output_path = self.get_partial_path("tags/index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    fn get_partial_path(&self, relative: &str) -> PathBuf {
        PathBuf::from(&self.config.build.output_dir)
            .join(&self.config.build.partial_dir)
            .join(relative)
    }

    /// Removes `page/N` directories beyond the current page count: posts
    /// removed since the last build would otherwise leave orphaned
    /// pagination pages serving stale content.
    fn remove_stale_pagination(section_dir: &Path, total_pages: usize) {
        let page_dir = section_dir.join("page");
        let Ok(entries) = fs::read_dir(&page_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let is_stale = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<usize>().ok())
                .is_some_and(|num| num > total_pages);

            if is_stale {
                if let Err(e) = fs::remove_dir_all(entry.path()) {
                    eprintln!(
                        "⚠️  Failed to remove stale pagination {}: {}",
                        entry.path().display(),
                        e
                    );
                } else {
                    println!("🧹 Removed stale pagination: {}", entry.path().display());
                }
            }
        }

        // Drops the page/ directory itself once no numbered pages remain
        let _ = fs::remove_dir(&page_dir);
    }

    /// Removes listing pages of tags no longer used by any post. Tags may
    /// contain `/` and nest directories, so every generated `index.html` is
    /// mapped back to its tag (normalizing a `page/N` suffix) and removed
    /// when that tag is gone; emptied directories are pruned afterwards.
    fn remove_stale_tag_dirs(&self, tag_base_dir: &Path, metadata: &MetadataCache) {
        use walkdir::WalkDir;

        if !tag_base_dir.exists() {
            return;
        }

        let current_tags: HashSet<PathBuf> = metadata
            .get_tags()
            .iter()
            .map(|tag| PathBuf::from(self.maybe_encode(tag)))
            .collect();

        let stale_files: Vec<PathBuf> = WalkDir::new(tag_base_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "index.html")
            .filter_map(|e| {
                let dir = e.path().parent()?;
                let relative = dir.strip_prefix(tag_base_dir).ok()?;
                // Live when the dir is a tag path itself (covers tags whose
                // name happens to end in `page/N`) or a pagination dir of one
                let live = current_tags.contains(relative)
                    || current_tags.contains(Self::strip_page_suffix(relative));
                (!live).then(|| e.path().to_path_buf())
            })
            .collect();

        for file in stale_files {
            if let Err(e) = fs::remove_file(&file) {
                eprintln!(
                    "⚠️  Failed to remove stale tag page {}: {}",
                    file.display(),
                    e
                );
                continue;
            }
            println!("🧹 Removed stale tag page: {}", file.display());

            let mut dir = file.parent();
            while let Some(d) = dir {
                if d == tag_base_dir || fs::remove_dir(d).is_err() {
                    break;
                }
                dir = d.parent();
            }
        }
    }

    /// Maps a pagination directory (`<tag>/page/N`) back to its tag path.
    fn strip_page_suffix(relative: &Path) -> &Path {
        let is_page_number = relative
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let parent_is_page = relative
            .parent()
            .and_then(|p| p.file_name())
            .is_some_and(|n| n == "page");

        if is_page_number && parent_is_page {
            relative
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(relative)
        } else {
            relative
        }
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
            total_posts.div_ceil(posts_per_page)
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
