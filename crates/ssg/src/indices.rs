use crate::config::SsgConfig;
use crate::i18n::UiCatalog;
use crate::image::{ImageProcessor, ThumbnailMetadata};
use crate::metadata::{compare_posts_desc, MetadataCache, PostMetadata};
use crate::slug;
use crate::types::Category;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    ui: Arc<UiCatalog>,
    image_processor: Option<ImageProcessor>,
    content_dir: PathBuf,
}

impl IndexGenerator {
    pub fn new(config: SsgConfig, ui: Arc<UiCatalog>) -> Result<Self> {
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
            ui,
            image_processor,
            content_dir,
        })
    }

    /// Inserts the localization variables every template needs via base.html:
    /// `lang`, the resolved string map `t`, and `lang_prefix` (the URL prefix
    /// used to build language-aware internal links, e.g. `/en`).
    fn insert_localization(&self, context: &mut TeraContext, lang: &str) {
        context.insert("lang", lang);
        context.insert("t", &self.ui.resolved(lang));
        context.insert("lang_prefix", &self.lang_url_prefix(lang));
    }

    /// URL prefix for a language: empty for the default language (served at the
    /// root), otherwise `/<lang>` (e.g. `/en`). Derived from `lang`, so it works
    /// for any configured non-default language.
    fn lang_url_prefix(&self, lang: &str) -> String {
        if self.config.languages.is_default(lang) {
            String::new()
        } else {
            format!("/{}", lang)
        }
    }

    /// Filesystem root for a language's listing output: `output_dir` for the
    /// default language, `output_dir/<lang>` otherwise.
    fn lang_output_root(&self, lang: &str) -> PathBuf {
        let mut root = PathBuf::from(&self.config.build.output_dir);
        if !self.config.languages.is_default(lang) {
            root.push(lang);
        }
        root
    }

    /// Filesystem root for a language's SPA partials: `output_dir/<partial_dir>`
    /// for the default language, `output_dir/<partial_dir>/<lang>` otherwise. The
    /// language segment goes AFTER `partial_dir` so the SPA's `/html/<path>`
    /// mapping resolves `/en/chat/` to `/html/en/chat/`.
    fn lang_partial_root(&self, lang: &str) -> PathBuf {
        let mut root =
            PathBuf::from(&self.config.build.output_dir).join(&self.config.build.partial_dir);
        if !self.config.languages.is_default(lang) {
            root.push(lang);
        }
        root
    }

    /// A category with its display name resolved to `lang`, so listing templates
    /// that render `category.name` show the right language. For the default
    /// language this is an unchanged clone (byte-identical output).
    fn localize_category(&self, category: &Category, lang: &str) -> Category {
        let mut localized = category.clone();
        localized.name = category.display_name(lang).to_string();
        localized
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

    pub fn generate_all(&self, metadata: &MetadataCache, lang: &str) -> Result<()> {
        println!("\n📑 Generating indices ({})...", lang);

        self.generate_homepage(metadata, lang)?;

        let category_count = metadata.get_category_info().len();
        for category in metadata.get_category_info() {
            self.generate_category_page(category, metadata, lang)?;
        }

        for tag in metadata.get_tags() {
            self.generate_tag_page(&tag, metadata, lang)?;
        }

        self.remove_stale_tag_dirs(&self.lang_output_root(lang).join("tag"), metadata);

        self.generate_tags_overview(metadata, lang)?;

        println!("   ✓ Homepage");
        println!("   ✓ {} category pages", category_count);
        println!("   ✓ {} tag pages", metadata.get_tags().len());

        Ok(())
    }

    pub fn generate_all_partials(&self, metadata: &MetadataCache, lang: &str) -> Result<()> {
        if !self.config.build.generate_partials {
            return Ok(());
        }

        println!("\n📄 Generating index partials ({})...", lang);

        self.generate_homepage_partial(metadata, lang)?;

        let category_count = metadata.get_category_info().len();
        for category in metadata.get_category_info() {
            self.generate_category_partial(category, metadata, lang)?;
        }

        for tag in metadata.get_tags() {
            self.generate_tag_partial(&tag, metadata, lang)?;
        }

        self.remove_stale_tag_dirs(&self.lang_partial_root(lang).join("tag"), metadata);

        self.generate_tags_overview_partial(metadata, lang)?;

        println!("   ✓ Homepage partial");
        println!("   ✓ {} category partials", category_count);
        println!("   ✓ {} tag partials", metadata.get_tags().len());

        Ok(())
    }

    fn generate_homepage(&self, metadata: &MetadataCache, lang: &str) -> Result<()> {
        let output = self.render_homepage(metadata, lang, "index.html")?;
        let output_path = self.lang_output_root(lang).join("index.html");
        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;
        Ok(())
    }

    /// Shared homepage render for both the full page and the SPA partial. The
    /// category tabs are localized to `lang`; for non-default languages, tabs and
    /// the recent list only include categories that have posts in that language.
    fn render_homepage(
        &self,
        metadata: &MetadataCache,
        lang: &str,
        template: &str,
    ) -> Result<String> {
        let is_default = self.config.languages.is_default(lang);
        let posts_limit = self
            .config
            .build
            .homepage_posts_limit
            .unwrap_or(self.config.build.posts_per_page);

        let mut visible_categories: Vec<Category> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .filter(|c| is_default || !metadata.get_posts_by_category_tree(&c.slug).is_empty())
            .map(|c| self.localize_category(c, lang))
            .collect();
        visible_categories.sort_by_key(|c| c.index);

        let visible_category_slugs: HashSet<String> =
            visible_categories.iter().map(|c| c.slug.clone()).collect();

        let all_recent_posts: Vec<_> = metadata
            .get_recent_posts(posts_limit)
            .into_iter()
            .filter(|p| visible_category_slugs.contains(&p.category))
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
        self.insert_localization(&mut context, lang);

        Ok(self.tera.render(template, &context)?)
    }

    fn generate_category_page(
        &self,
        category_info: &Category,
        metadata: &MetadataCache,
        lang: &str,
    ) -> Result<()> {
        let category_slug = self.maybe_encode(&category_info.slug);
        let section_dir = self.lang_output_root(lang).join(&category_slug);

        let mut posts = metadata.get_posts_by_category_tree(&category_info.slug);
        posts.sort_by(|a, b| compare_posts_desc(a, b));

        // For a non-default language, a category with no posts is not published;
        // remove any stale page it may have from an earlier build. The default
        // language keeps generating an (empty) page-1 to preserve existing output.
        if posts.is_empty() && !self.config.languages.is_default(lang) {
            let index = section_dir.join("index.html");
            if index.exists() {
                let _ = fs::remove_file(&index);
            }
            Self::remove_stale_pagination(&section_dir, 0);
            return Ok(());
        }

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

        let base_url = format!("{}/{}/", self.lang_url_prefix(lang), category_info.slug);
        let localized_category = self.localize_category(category_info, lang);
        let visible_categories = self.localized_visible_categories(metadata, lang);
        let template_config = self.config.to_template_config();

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts_with_thumbnails[start_idx..end_idx];

            let mut context = TeraContext::new();
            context.insert("category", &localized_category);
            context.insert("posts", &page_posts);
            context.insert("post_count", &total_posts);
            context.insert("categories", &visible_categories);
            context.insert("config", &template_config);
            self.insert_localization(&mut context, lang);

            if total_pages > 1 {
                let pagination = self.build_pagination_context(page_num, total_posts, &base_url);
                context.insert("pagination", &pagination);
            }

            let output = self.tera.render("category.html", &context)?;

            let output_path = if page_num == 1 {
                section_dir.join("index.html")
            } else {
                section_dir
                    .join("page")
                    .join(page_num.to_string())
                    .join("index.html")
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    /// Visible (non-hidden) categories with names localized to `lang`, for the
    /// nav/category lists passed to listing templates.
    fn localized_visible_categories(&self, metadata: &MetadataCache, lang: &str) -> Vec<Category> {
        metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .map(|c| self.localize_category(c, lang))
            .collect()
    }

    fn generate_tag_page(&self, tag: &str, metadata: &MetadataCache, lang: &str) -> Result<()> {
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

        let base_url = format!("{}/tag/{}/", self.lang_url_prefix(lang), tag);
        let visible_categories = self.localized_visible_categories(metadata, lang);
        let template_config = self.config.to_template_config();

        let encoded_tag = self.maybe_encode(tag);
        let section_dir = self.lang_output_root(lang).join("tag").join(&encoded_tag);

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
            self.insert_localization(&mut context, lang);

            if total_pages > 1 {
                let pagination = self.build_pagination_context(page_num, total_posts, &base_url);
                context.insert("pagination", &pagination);
            }

            let output = self.tera.render("tag.html", &context)?;

            let output_path = if page_num == 1 {
                section_dir.join("index.html")
            } else {
                section_dir
                    .join("page")
                    .join(page_num.to_string())
                    .join("index.html")
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tags_overview(&self, metadata: &MetadataCache, lang: &str) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        let visible_categories = self.localized_visible_categories(metadata, lang);

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("categories", &visible_categories);
        context.insert("config", &self.config.to_template_config());
        self.insert_localization(&mut context, lang);

        let output = self.tera.render("tags.html", &context)?;
        let output_path = self.lang_output_root(lang).join("tags").join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    fn generate_homepage_partial(&self, metadata: &MetadataCache, lang: &str) -> Result<()> {
        let output = self.render_homepage(metadata, lang, "partials/index.html")?;
        let output_path = self.lang_partial_root(lang).join("index.html");
        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;
        Ok(())
    }

    fn generate_category_partial(
        &self,
        category_info: &Category,
        metadata: &MetadataCache,
        lang: &str,
    ) -> Result<()> {
        let category_slug = self.maybe_encode(&category_info.slug);
        let section_dir = self.lang_partial_root(lang).join(&category_slug);

        let mut posts = metadata.get_posts_by_category_tree(&category_info.slug);
        posts.sort_by(|a, b| compare_posts_desc(a, b));

        if posts.is_empty() && !self.config.languages.is_default(lang) {
            let index = section_dir.join("index.html");
            if index.exists() {
                let _ = fs::remove_file(&index);
            }
            Self::remove_stale_pagination(&section_dir, 0);
            return Ok(());
        }

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

        let base_url = format!("{}/{}/", self.lang_url_prefix(lang), category_info.slug);
        let localized_category = self.localize_category(category_info, lang);
        let visible_categories = self.localized_visible_categories(metadata, lang);
        let template_config = self.config.to_template_config();

        for page_num in 1..=total_pages {
            let start_idx = (page_num - 1) * posts_per_page;
            let end_idx = std::cmp::min(start_idx + posts_per_page, total_posts);
            let page_posts = &posts_with_thumbnails[start_idx..end_idx];

            let mut context = TeraContext::new();
            context.insert("category", &localized_category);
            context.insert("posts", &page_posts);
            context.insert("post_count", &total_posts);
            context.insert("categories", &visible_categories);
            context.insert("config", &template_config);
            self.insert_localization(&mut context, lang);

            if total_pages > 1 {
                let pagination = self.build_pagination_context(page_num, total_posts, &base_url);
                context.insert("pagination", &pagination);
            }

            let output = self.tera.render("partials/category.html", &context)?;

            let output_path = if page_num == 1 {
                section_dir.join("index.html")
            } else {
                section_dir
                    .join("page")
                    .join(page_num.to_string())
                    .join("index.html")
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tag_partial(&self, tag: &str, metadata: &MetadataCache, lang: &str) -> Result<()> {
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

        let base_url = format!("{}/tag/{}/", self.lang_url_prefix(lang), tag);
        let visible_categories = self.localized_visible_categories(metadata, lang);
        let template_config = self.config.to_template_config();

        let encoded_tag = self.maybe_encode(tag);
        let section_dir = self.lang_partial_root(lang).join("tag").join(&encoded_tag);

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
            self.insert_localization(&mut context, lang);

            if total_pages > 1 {
                let pagination = self.build_pagination_context(page_num, total_posts, &base_url);
                context.insert("pagination", &pagination);
            }

            let output = self.tera.render("partials/tag.html", &context)?;

            let output_path = if page_num == 1 {
                section_dir.join("index.html")
            } else {
                section_dir
                    .join("page")
                    .join(page_num.to_string())
                    .join("index.html")
            };

            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, output)?;
        }

        Self::remove_stale_pagination(&section_dir, total_pages);

        Ok(())
    }

    fn generate_tags_overview_partial(&self, metadata: &MetadataCache, lang: &str) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        let visible_categories = self.localized_visible_categories(metadata, lang);

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("categories", &visible_categories);
        context.insert("config", &self.config.to_template_config());
        self.insert_localization(&mut context, lang);

        let output = self.tera.render("partials/tags.html", &context)?;
        let output_path = self.lang_partial_root(lang).join("tags").join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
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
    tera.register_filter("str", crate::generator::str_filter);

    Ok(tera)
}

fn urldecode_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = tera::try_get_value!("urldecode", "value", String, value);
    let decoded = slug::decode_from_url(&s);
    Ok(Value::String(decoded))
}
