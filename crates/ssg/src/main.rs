mod cache;
mod category;
mod config;
mod feeds;
mod generator;
mod image;
mod indices;
mod metadata;
mod navigation;
mod parallel;
mod parser;
mod recent;
mod renderer;
mod robots;
mod search;
mod shortcodes;
mod sitemap;
mod slug;
mod syntax_highlighter;
mod types;

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use walkdir::WalkDir;

use crate::cache::{hash_directory, hash_file, BuildCache};
use crate::category::{discover_categories, validate_category};
use crate::config::{load_config, SsgConfig};
use crate::feeds::FeedGenerator;
use crate::generator::Generator;
use crate::image::{ImageProcessor, ThumbnailMetadata};
use crate::indices::IndexGenerator;
use crate::metadata::MetadataCache;
use crate::navigation::{build_post_navigation, build_post_navigation_with_cdn};
use crate::parallel::{
    get_thread_count, BuildProgress, BuildResult, SkipReason, WorkQueue, WorkerPool,
};
use crate::parser::Parser;
use crate::recent::RecentGenerator;
use crate::renderer::Renderer;
use crate::search::SearchIndexGenerator;
use crate::robots::RobotsGenerator;
use crate::shortcodes::ShortcodeRegistry;
use crate::sitemap::SitemapGenerator;
use crate::types::Post;

const RELATED_POSTS_COUNT: usize = 4;
const DEV_SERVER_BUFFER_SIZE: usize = 1024;

fn build_pages(
    shortcode_registry: &ShortcodeRegistry,
    renderer: &Renderer,
    generator: &Generator,
) -> Result<usize> {
    let pages_dir = Path::new("content/pages");
    if !pages_dir.exists() {
        return Ok(0);
    }

    println!("\n📄 Building pages...");
    let mut pages_built = 0;

    for entry in WalkDir::new(pages_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        let path = entry.path();
        println!("🔨 Building page: {}", path.display());

        let mut page = Parser::parse_page_file(path)?;

        if page.frontmatter.hidden {
            println!("   ⚠  Hidden - skipping output");
            continue;
        }

        let processed_content = shortcode_registry.process(&page.content)?;
        let html = renderer.render_markdown_with_components(
            &processed_content,
            generator.get_tera(),
            &page.slug,
        )?;
        page.rendered_html = Some(html);

        let output_path = generator.generate_page(&page, &HashMap::new())?;
        println!("   ✓ {}", output_path.display());

        if generator.should_generate_partials() {
            generator.generate_page_partial(&page, &HashMap::new())?;
        }

        pages_built += 1;
    }

    if pages_built > 0 {
        println!("✅ Built {} page(s)", pages_built);
    }

    Ok(pages_built)
}

fn resolve_post_images(post: &mut Post) {
    post.frontmatter.cover_image = post
        .frontmatter
        .cover_image
        .take()
        .map(|cover| Renderer::resolve_path(&cover, &post.category));
    post.frontmatter.og_image = post
        .frontmatter
        .og_image
        .take()
        .map(|og| Renderer::resolve_path(&og, &post.category));
}

struct PostProcessingContext<'a> {
    renderer: &'a Renderer,
    generator: &'a Generator,
    shortcode_registry: &'a ShortcodeRegistry,
    config: &'a SsgConfig,
    cache: &'a Arc<Mutex<BuildCache>>,
    template_hash: &'a str,
    metadata: &'a MetadataCache,
    use_cache: bool,
}

#[derive(ClapParser)]
#[command(name = "blog")]
#[command(about = "Static site generator for marshallku blog")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the site
    Build {
        /// Build only changed files (incremental build)
        #[arg(short, long)]
        incremental: bool,

        /// Build a specific post
        #[arg(short, long)]
        post: Option<String>,

        /// Use parallel processing for faster builds
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        parallel: bool,
    },

    /// Watch for changes and rebuild
    Watch {
        /// Port for dev server
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Create a new post
    New {
        /// Category (dev, chat, gallery, notice)
        category: String,

        /// Post title
        title: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            incremental,
            post,
            parallel,
        } => {
            if let Some(post_path) = post {
                return build_single_post(&post_path);
            }

            if incremental {
                println!("Note: Incremental build uses cache to skip unchanged files");
            }

            if parallel {
                build_all_parallel(incremental)?;
            } else {
                build_all(incremental)?;
            }
        }
        Commands::Watch { port } => watch_mode(port)?,
        Commands::New { category, title } => create_new_post(&category, &title)?,
    }

    Ok(())
}

fn build_all(use_cache: bool) -> Result<()> {
    println!("Building site...\n");

    let config = load_config()?;
    let renderer = Renderer::new();
    let shortcode_registry = ShortcodeRegistry::new();
    let generator = Generator::new(config.clone())?;
    let mut cache = if use_cache {
        BuildCache::load()?
    } else {
        BuildCache::new()
    };
    let mut metadata = if use_cache {
        MetadataCache::load().unwrap_or_else(|_| MetadataCache::new())
    } else {
        MetadataCache::new()
    };

    let posts_dir = Path::new(&config.build.content_dir);

    if !posts_dir.exists() {
        anyhow::bail!(
            "Content directory '{}' does not exist. Create it first with: mkdir -p {}",
            config.build.content_dir,
            config.build.content_dir
        );
    }

    let template_hash = hash_directory(Path::new("templates"))?;

    let categories = discover_categories(posts_dir)?;
    if categories.is_empty() {
        eprintln!("⚠️  Warning: No categories found in content directory");
        eprintln!("   Create a category by adding a subdirectory with markdown files:");
        eprintln!("   mkdir -p {}/dev", config.build.content_dir);
    }
    metadata.set_category_info(categories);

    for entry in WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        if let Ok(mut post) = Parser::parse_file(entry.path()) {
            if !post.frontmatter.hidden {
                resolve_post_images(&mut post);
                metadata.upsert_post(post.slug, post.category, post.frontmatter);
            }
        }
    }

    let mut built_count = 0;
    let mut skipped_count = 0;

    for entry in WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        let path = entry.path();
        let file_hash = hash_file(path)?;

        if use_cache && !cache.needs_rebuild(path, &file_hash, &template_hash) {
            println!("⏭  Skipping (unchanged): {}", path.display());
            skipped_count += 1;
            continue;
        }

        println!("🔨 Building: {}", path.display());

        let mut post = Parser::parse_file(path)?;

        if post.frontmatter.hidden {
            println!("   ⚠  Hidden - skipping output");
            skipped_count += 1;
            continue;
        }

        let processed_content = shortcode_registry.process(&post.content)?;

        let base_path = post.category.clone();
        let content_dir = Path::new(&config.build.content_dir);
        let html = renderer.render_markdown_with_components_and_images(
            &processed_content,
            generator.get_tera(),
            &base_path,
            config.site.cdn_url.as_deref(),
            Some(content_dir),
        )?;

        post.rendered_html = Some(html);

        // Capture original paths before resolution for CDN processing
        let original_paths = OriginalImagePaths {
            cover_image: post.frontmatter.cover_image.clone(),
            og_image: post.frontmatter.og_image.clone(),
        };
        resolve_post_images(&mut post);

        let extra_data = build_post_extra_data(
            &post,
            &metadata,
            config.site.cdn_url.as_deref(),
            content_dir,
            Some(&original_paths),
        );
        let output_path = generator.generate_post(&post, &extra_data)?;

        if generator.should_generate_partials() {
            generator.generate_post_partial(&post, &extra_data)?;
        }

        cache.update_entry(
            path,
            file_hash,
            template_hash.clone(),
            output_path.to_string_lossy().to_string(),
        );

        metadata.upsert_post(
            post.slug.clone(),
            post.category.clone(),
            post.frontmatter.clone(),
        );

        built_count += 1;
    }

    if use_cache {
        cache.save()?;
    }
    metadata.save()?;

    build_pages(&shortcode_registry, &renderer, &generator)?;

    let index_generator = IndexGenerator::new(config.clone())?;
    index_generator.generate_all(&metadata)?;
    index_generator.generate_all_partials(&metadata)?;

    println!("📄 Generating RSS feeds...");
    FeedGenerator::generate_all_feeds(
        &config,
        &metadata,
        posts_dir,
        Path::new(&config.build.output_dir),
    )?;

    println!("🗺  Generating sitemap...");
    SitemapGenerator::generate(&config, &metadata, Path::new(&config.build.output_dir))?;

    println!("🤖 Generating robots.txt...");
    RobotsGenerator::generate(&config, Path::new(&config.build.output_dir))?;

    if config.build.search.enabled {
        let search_generator = SearchIndexGenerator::new(config.clone());
        search_generator.generate(&metadata)?;
    }

    let recent_generator = RecentGenerator::new(config.clone());
    recent_generator.generate(&metadata)?;

    generator.copy_content_assets()?;
    generator.copy_static_assets()?;

    println!("\n✅ Build complete!");
    println!("   Built: {}", built_count);
    if use_cache {
        println!("   Skipped: {}", skipped_count);
    }
    println!("   Categories: {}", metadata.get_categories().len());
    println!("   Tags: {}", metadata.get_tags().len());

    Ok(())
}

fn build_all_parallel(use_cache: bool) -> Result<()> {
    let start_time = std::time::Instant::now();
    let num_threads = get_thread_count();
    println!("Building site with {} threads...\n", num_threads);

    let config = Arc::new(load_config()?);
    let posts_dir = Path::new(&config.build.content_dir);

    if !posts_dir.exists() {
        anyhow::bail!(
            "Content directory '{}' does not exist",
            config.build.content_dir
        );
    }

    let template_hash = Arc::new(hash_directory(Path::new("templates"))?);

    let categories = discover_categories(posts_dir)?;
    let mut metadata = if use_cache {
        MetadataCache::load().unwrap_or_else(|_| MetadataCache::new())
    } else {
        MetadataCache::new()
    };
    metadata.set_category_info(categories);

    let cache = Arc::new(Mutex::new(if use_cache {
        BuildCache::load()?
    } else {
        BuildCache::new()
    }));

    let shortcode_registry = Arc::new(ShortcodeRegistry::new());

    let file_paths: Vec<PathBuf> = WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| e.path().to_path_buf())
        .collect();

    for path in &file_paths {
        if let Ok(mut post) = Parser::parse_file(path) {
            if !post.frontmatter.hidden {
                resolve_post_images(&mut post);
                metadata.upsert_post(post.slug, post.category, post.frontmatter);
            }
        }
    }

    let metadata_for_nav = Arc::new(metadata.clone());

    let progress = Arc::new(BuildProgress::new());

    let work_queue = WorkQueue::new();
    let work_rx = work_queue.get_receiver();
    let (result_tx, result_rx) = mpsc::channel();

    for path in file_paths {
        work_queue.send(path)?;
    }
    work_queue.close();

    let mut pool = WorkerPool::new();

    for _ in 0..num_threads {
        let work_rx = Arc::clone(&work_rx);
        let result_tx = result_tx.clone();
        let config = Arc::clone(&config);
        let cache = Arc::clone(&cache);
        let template_hash = Arc::clone(&template_hash);
        let shortcode_registry = Arc::clone(&shortcode_registry);
        let progress = Arc::clone(&progress);
        let metadata_for_nav = Arc::clone(&metadata_for_nav);

        pool.spawn(move || {
            let renderer = Renderer::new();
            let generator = match Generator::new((*config).clone()) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Failed to create generator: {}", e);
                    return;
                }
            };

            loop {
                let path = {
                    let rx = work_rx.lock().unwrap();
                    rx.recv().ok()
                };

                let path = match path {
                    Some(p) => p,
                    None => break,
                };

                let ctx = PostProcessingContext {
                    renderer: &renderer,
                    generator: &generator,
                    shortcode_registry: &shortcode_registry,
                    config: &config,
                    cache: &cache,
                    template_hash: &template_hash,
                    metadata: &metadata_for_nav,
                    use_cache,
                };
                let result = process_post_parallel(&path, &ctx);

                match &result {
                    BuildResult::Success { .. } => progress.increment_built(),
                    BuildResult::Skipped { .. } => progress.increment_skipped(),
                    BuildResult::Error { .. } => {}
                }

                let _ = result_tx.send(result);
            }
        });
    }

    drop(result_tx);

    let mut results = Vec::new();
    for result in result_rx {
        results.push(result);
    }

    pool.join().map_err(|e| anyhow::anyhow!(e))?;

    let mut errors = Vec::new();
    for result in results {
        match result {
            BuildResult::Success {
                path,
                slug,
                category,
                frontmatter,
                file_hash,
                template_hash,
                output_path,
            } => {
                println!("🔨 Built: {}", path.display());
                metadata.upsert_post(slug, category, *frontmatter);
                cache
                    .lock()
                    .unwrap()
                    .update_entry(&path, file_hash, template_hash, output_path);
            }
            BuildResult::Skipped { path, reason } => match reason {
                SkipReason::Cached => println!("⏭  Skipped (unchanged): {}", path.display()),
                SkipReason::Draft => println!("   ⚠  Draft - skipping: {}", path.display()),
            },
            BuildResult::Error { path, error } => {
                eprintln!("❌ Error building {}: {}", path.display(), error);
                errors.push((path, error));
            }
        }
    }

    if !errors.is_empty() {
        anyhow::bail!("{} posts failed to build", errors.len());
    }

    if use_cache {
        cache.lock().unwrap().save()?;
    }
    metadata.save()?;

    let renderer = Renderer::new();
    let generator = Generator::new((*config).clone())?;
    build_pages(&shortcode_registry, &renderer, &generator)?;

    let index_generator = IndexGenerator::new((*config).clone())?;
    index_generator.generate_all(&metadata)?;
    index_generator.generate_all_partials(&metadata)?;

    println!("📄 Generating RSS feeds...");
    FeedGenerator::generate_all_feeds(
        &config,
        &metadata,
        posts_dir,
        Path::new(&config.build.output_dir),
    )?;

    println!("🗺  Generating sitemap...");
    SitemapGenerator::generate(&config, &metadata, Path::new(&config.build.output_dir))?;

    println!("🤖 Generating robots.txt...");
    RobotsGenerator::generate(&config, Path::new(&config.build.output_dir))?;

    if config.build.search.enabled {
        let search_generator = SearchIndexGenerator::new((*config).clone());
        search_generator.generate(&metadata)?;
    }

    let recent_generator = RecentGenerator::new((*config).clone());
    recent_generator.generate(&metadata)?;

    generator.copy_content_assets()?;
    generator.copy_static_assets()?;

    let elapsed = start_time.elapsed();
    println!("\n✅ Build complete in {:.2}s!", elapsed.as_secs_f64());
    println!("   Built: {}", progress.get_built());
    if use_cache {
        println!("   Skipped: {}", progress.get_skipped());
    }
    println!("   Categories: {}", metadata.get_categories().len());
    println!("   Tags: {}", metadata.get_tags().len());

    Ok(())
}

macro_rules! try_or_error {
    ($path:expr, $result:expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                return BuildResult::Error {
                    path: $path.to_path_buf(),
                    error: e.to_string(),
                }
            }
        }
    };
}

fn process_post_parallel(path: &Path, ctx: &PostProcessingContext) -> BuildResult {
    let file_hash = try_or_error!(path, hash_file(path));

    if ctx.use_cache {
        let cache = ctx.cache.lock().unwrap();
        if !cache.needs_rebuild(path, &file_hash, ctx.template_hash) {
            return BuildResult::Skipped {
                path: path.to_path_buf(),
                reason: SkipReason::Cached,
            };
        }
    }

    let mut post = try_or_error!(path, Parser::parse_file(path));

    if post.frontmatter.hidden {
        return BuildResult::Skipped {
            path: path.to_path_buf(),
            reason: SkipReason::Draft,
        };
    }

    let processed_content = try_or_error!(path, ctx.shortcode_registry.process(&post.content));

    let base_path = post.category.clone();
    let content_dir = Path::new(&ctx.config.build.content_dir);
    let html = try_or_error!(
        path,
        ctx.renderer.render_markdown_with_components_and_images(
            &processed_content,
            ctx.generator.get_tera(),
            &base_path,
            ctx.config.site.cdn_url.as_deref(),
            Some(content_dir),
        )
    );

    post.rendered_html = Some(html);

    // Capture original paths before resolution for CDN processing
    let original_paths = OriginalImagePaths {
        cover_image: post.frontmatter.cover_image.clone(),
        og_image: post.frontmatter.og_image.clone(),
    };
    resolve_post_images(&mut post);

    let extra_data = build_post_extra_data(
        &post,
        ctx.metadata,
        ctx.config.site.cdn_url.as_deref(),
        content_dir,
        Some(&original_paths),
    );
    let output_path = try_or_error!(path, ctx.generator.generate_post(&post, &extra_data));

    if ctx.generator.should_generate_partials() {
        try_or_error!(
            path,
            ctx.generator
                .generate_post_partial(&post, &extra_data)
                .map_err(|e| anyhow::anyhow!("Failed to generate partial: {}", e))
        );
    }

    BuildResult::Success {
        path: path.to_path_buf(),
        slug: post.slug,
        category: post.category,
        frontmatter: Box::new(post.frontmatter),
        file_hash,
        template_hash: ctx.template_hash.to_string(),
        output_path: output_path.to_string_lossy().to_string(),
    }
}

fn build_single_post(post_path: &str) -> Result<()> {
    println!("Building single post: {}\n", post_path);

    let config = load_config()?;
    let renderer = Renderer::new();
    let shortcode_registry = ShortcodeRegistry::new();
    let generator = Generator::new(config.clone())?;
    let metadata = MetadataCache::load().unwrap_or_else(|_| MetadataCache::new());

    let path = Path::new(post_path);

    if !path.exists() {
        anyhow::bail!("Post file not found: {}", post_path);
    }

    let mut post = Parser::parse_file(path)?;

    if post.frontmatter.hidden {
        println!("⚠  This is a hidden post");
    }

    let processed_content = shortcode_registry.process(&post.content)?;

    let base_path = post.category.clone();
    let content_dir = Path::new(&config.build.content_dir);
    let html = renderer.render_markdown_with_components_and_images(
        &processed_content,
        generator.get_tera(),
        &base_path,
        config.site.cdn_url.as_deref(),
        Some(content_dir),
    )?;

    post.rendered_html = Some(html);

    // Capture original paths before resolution for CDN processing
    let original_paths = OriginalImagePaths {
        cover_image: post.frontmatter.cover_image.clone(),
        og_image: post.frontmatter.og_image.clone(),
    };
    resolve_post_images(&mut post);

    let extra_data = build_post_extra_data(
        &post,
        &metadata,
        config.site.cdn_url.as_deref(),
        content_dir,
        Some(&original_paths),
    );
    let output_path = generator.generate_post(&post, &extra_data)?;

    println!("\n✅ Built: {}", output_path.display());

    Ok(())
}

struct OriginalImagePaths {
    cover_image: Option<String>,
    og_image: Option<String>,
}

#[derive(Serialize)]
struct RelatedPostData {
    #[serde(flatten)]
    post: crate::metadata::PostMetadata,
    thumbnail_metadata: Option<ThumbnailMetadata>,
}

fn build_post_extra_data(
    post: &crate::types::Post,
    metadata: &MetadataCache,
    cdn_url: Option<&str>,
    content_dir: &Path,
    original_paths: Option<&OriginalImagePaths>,
) -> HashMap<String, serde_json::Value> {
    let mut data = HashMap::new();

    if let Some(cat_info) = metadata
        .category_info
        .iter()
        .find(|c| c.slug == post.category)
    {
        data.insert("category_info".to_string(), json!(cat_info));
    }

    // Build navigation with or without CDN processing
    let navigation = if let Some(url) = cdn_url {
        let image_processor = ImageProcessor::new(Some(url.to_string()));
        build_post_navigation_with_cdn(
            &post.slug,
            &post.category,
            metadata,
            true,
            &image_processor,
            content_dir,
        )
    } else {
        build_post_navigation(&post.slug, &post.category, metadata, true)
    };
    data.insert("prev_post".to_string(), json!(navigation.prev));
    data.insert("next_post".to_string(), json!(navigation.next));

    // Process cover image for CDN if available
    if let (Some(url), Some(paths)) = (cdn_url, original_paths) {
        let image_processor = ImageProcessor::new(Some(url.to_string()));
        let base_path = post.category.clone();
        let post_content_dir = content_dir.join(&post.category);

        // Process cover image (full responsive)
        if let Some(ref cover_src) = paths.cover_image {
            if let Ok(Some(metadata)) =
                image_processor.process_image(cover_src, &post_content_dir, &base_path)
            {
                data.insert("cover_image_metadata".to_string(), json!(metadata));
            }
        }

        // Process og_image for post card thumbnails
        if let Some(ref og_src) = paths.og_image {
            if let Ok(Some(metadata)) =
                image_processor.process_thumbnail(og_src, &post_content_dir, &base_path)
            {
                data.insert("og_image_metadata".to_string(), json!(metadata));
            }
        }
    }

    let mut related: Vec<_> = metadata
        .posts
        .iter()
        .filter(|p| p.category == post.category && p.slug != post.slug)
        .collect();
    related.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

    let related_posts: Vec<RelatedPostData> = related
        .into_iter()
        .take(RELATED_POSTS_COUNT)
        .map(|p| {
            let thumbnail_metadata = cdn_url.and_then(|url| {
                let image_processor = ImageProcessor::new(Some(url.to_string()));
                let cover_src = p.frontmatter.cover_image.as_ref()
                    .or(p.frontmatter.og_image.as_ref())?;

                let relative_src = if cover_src.starts_with('/') {
                    let without_leading_slash = cover_src.trim_start_matches('/');
                    if let Some(rest) = without_leading_slash.strip_prefix(&p.category) {
                        format!(".{}", rest)
                    } else {
                        format!("./{}", without_leading_slash)
                    }
                } else {
                    cover_src.clone()
                };

                let post_content_dir = content_dir.join(&p.category);
                let base_path = p.category.clone();

                image_processor.process_thumbnail(&relative_src, &post_content_dir, &base_path)
                    .ok()
                    .flatten()
            });

            RelatedPostData {
                post: p.clone(),
                thumbnail_metadata,
            }
        })
        .collect();
    data.insert("related_posts".to_string(), json!(related_posts));

    data
}

fn create_new_post(category: &str, title: &str) -> Result<()> {
    let config = load_config()?;
    let posts_dir = Path::new(&config.build.content_dir);

    let categories = discover_categories(posts_dir)?;

    if !validate_category(category, &categories) {
        println!("⚠️  Category '{}' doesn't exist yet.", category);
        println!();

        if categories.is_empty() {
            println!("No categories found. To create one:");
            println!(
                "  1. Create a directory: mkdir -p {}/{}",
                config.build.content_dir, category
            );
            println!(
                "  2. Optionally add metadata: echo 'name: {}' > {}/{}/.category.yaml",
                category
                    .chars()
                    .next()
                    .unwrap()
                    .to_uppercase()
                    .chain(category.chars().skip(1))
                    .collect::<String>(),
                config.build.content_dir,
                category
            );
            println!("  3. Run this command again");
        } else {
            let category_list: Vec<String> = categories
                .iter()
                .map(|c| format!("  - {} ({})", c.slug, c.name))
                .collect();

            println!("Available categories:");
            for cat in category_list {
                println!("{}", cat);
            }
            println!();
            println!("To create a new category:");
            println!(
                "  1. Create a directory: mkdir -p {}/{}",
                config.build.content_dir, category
            );
            println!(
                "  2. Optionally add metadata: echo 'name: Your Name' > {}/{}/.category.yaml",
                config.build.content_dir, category
            );
            println!("  3. Add at least one post to the category");
            println!("  4. Run this command again");
        }

        std::process::exit(0);
    }

    let slug = title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    let filename = format!("content/posts/{}/{}.md", category, slug);

    if Path::new(&filename).exists() {
        anyhow::bail!("Post already exists: {}", filename);
    }

    let content = format!(
        r#"---
title: "{}"
date: {}
category: {}
tags: []
hidden: false
---

Write your post here...
"#,
        title,
        chrono::Utc::now().to_rfc3339(),
        category
    );

    std::fs::create_dir_all(format!("content/posts/{}", category))?;
    std::fs::write(&filename, content)?;

    println!("✅ Created: {}", filename);
    println!("   Title: {}", title);
    println!("   Category: {}", category);
    println!("   Slug: {}", slug);

    Ok(())
}

fn watch_mode(port: u16) -> Result<()> {
    use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    println!("🔍 Watch mode starting...");
    println!("   Watching for changes in:");
    println!("   - content/");
    println!("   - templates/");
    println!("   - static/");
    println!("\n   Serving on http://localhost:{}", port);
    println!("   Press Ctrl+C to stop\n");

    println!("📦 Initial build...");
    build_all(true)?;
    println!();

    let server_thread = std::thread::spawn(move || {
        if let Err(e) = start_dev_server(port) {
            eprintln!("Dev server error: {}", e);
        }
    });

    let (tx, rx) = channel();

    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| {
        if let Ok(event) = res {
            tx.send(event).unwrap();
        }
    })?;

    watcher.watch(Path::new("content"), RecursiveMode::Recursive)?;
    watcher.watch(Path::new("templates"), RecursiveMode::Recursive)?;

    if Path::new("static").exists() {
        watcher.watch(Path::new("static"), RecursiveMode::Recursive)?;
    }

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                if !should_rebuild(&event) {
                    continue;
                }

                println!("📝 File changed, rebuilding...");
                match build_all(true) {
                    Ok(_) => println!("✅ Rebuild complete!\n"),
                    Err(e) => eprintln!("❌ Build error: {}\n", e),
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if server_thread.is_finished() {
                    anyhow::bail!("Dev server stopped unexpectedly");
                }
                continue;
            }
            Err(e) => {
                anyhow::bail!("Watch error: {}", e);
            }
        }
    }
}

fn should_rebuild(event: &notify::Event) -> bool {
    use notify::EventKind;

    match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
            for path in &event.paths {
                let path_str = path.to_string_lossy();
                if path_str.contains(".build-cache") || path_str.contains("dist/") {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn start_dev_server(port: u16) -> Result<()> {
    use anyhow::Context as _;
    use std::io::Read;
    use std::net::TcpListener;

    let listener =
        TcpListener::bind(format!("127.0.0.1:{}", port)).context("Failed to bind dev server")?;

    println!("🌐 Dev server listening on http://localhost:{}", port);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Connection error: {}", e);
                continue;
            }
        };

        let mut buffer = [0; DEV_SERVER_BUFFER_SIZE];
        if stream.read(&mut buffer).is_err() {
            continue;
        }

        let request = String::from_utf8_lossy(&buffer);
        let request_line = request.lines().next().unwrap_or("");

        let path = if let Some(path_part) = request_line.split_whitespace().nth(1) {
            slug::decode_from_url(path_part)
        } else {
            "/".to_string()
        };

        serve_file(&mut stream, &path);
    }

    Ok(())
}

fn serve_file(stream: &mut std::net::TcpStream, path: &str) {
    use std::io::Write;

    let path = path.split('?').next().unwrap_or(path);
    let file_path = if path == "/" {
        "dist/index.html".to_string()
    } else if path.ends_with('/') {
        format!("dist{}index.html", path)
    } else {
        format!("dist{}", path)
    };

    let (status, content_type, body) = if let Ok(contents) = std::fs::read(&file_path) {
        let content_type = get_content_type(&file_path);
        ("200 OK", content_type, contents)
    } else {
        let index_path = format!("{}/index.html", file_path);
        if let Ok(contents) = std::fs::read(&index_path) {
            ("200 OK", "text/html", contents)
        } else {
            let body = b"404 Not Found".to_vec();
            ("404 NOT FOUND", "text/plain", body)
        }
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        status,
        content_type,
        body.len()
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else {
        "application/octet-stream"
    }
}
