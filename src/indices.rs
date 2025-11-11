use crate::metadata::MetadataCache;
use crate::types::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tera::{Context as TeraContext, Tera};

pub struct IndexGenerator {
    tera: Tera,
    config: Config,
}

impl IndexGenerator {
    pub fn new(config: Config) -> Result<Self> {
        let template_glob = format!("{}/**/*.html", config.template_dir);
        let tera = Tera::new(&template_glob)
            .context("Failed to load templates")?;

        Ok(Self { tera, config })
    }

    /// Generate all indices (homepage, categories, tags)
    pub fn generate_all(&self, metadata: &MetadataCache) -> Result<()> {
        println!("\n📑 Generating indices...");

        // Generate homepage
        self.generate_homepage(metadata)?;

        // Generate category pages
        for category in metadata.get_categories() {
            self.generate_category_page(&category, metadata)?;
        }

        // Generate tag pages
        for tag in metadata.get_tags() {
            self.generate_tag_page(&tag, metadata)?;
        }

        // Generate tags overview page
        self.generate_tags_overview(metadata)?;

        println!("   ✓ Homepage");
        println!("   ✓ {} category pages", metadata.get_categories().len());
        println!("   ✓ {} tag pages", metadata.get_tags().len());

        Ok(())
    }

    /// Generate homepage with recent posts
    fn generate_homepage(&self, metadata: &MetadataCache) -> Result<()> {
        let recent_posts = metadata.get_recent_posts(10);

        let mut context = TeraContext::new();
        context.insert("posts", &recent_posts);
        context.insert("config", &self.config);

        let output = self.tera.render("index.html", &context)?;
        let output_path = PathBuf::from(&self.config.output_dir).join("index.html");

        fs::write(&output_path, output)?;

        Ok(())
    }

    /// Generate category page
    fn generate_category_page(&self, category: &str, metadata: &MetadataCache) -> Result<()> {
        let mut posts = metadata.get_posts_by_category(category);

        // Sort by date, descending
        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let post_count = metadata.categories.get(category).unwrap_or(&0);

        let mut context = TeraContext::new();
        context.insert("category", category);
        context.insert("posts", &posts);
        context.insert("post_count", post_count);
        context.insert("config", &self.config);

        let output = self.tera.render("category.html", &context)?;
        let output_path = PathBuf::from(&self.config.output_dir)
            .join(category)
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    /// Generate tag page
    fn generate_tag_page(&self, tag: &str, metadata: &MetadataCache) -> Result<()> {
        let mut posts = metadata.get_posts_by_tag(tag);

        // Sort by date, descending
        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let post_count = metadata.tags.get(tag).unwrap_or(&0);

        let mut context = TeraContext::new();
        context.insert("tag", tag);
        context.insert("posts", &posts);
        context.insert("post_count", post_count);
        context.insert("config", &self.config);

        let output = self.tera.render("tag.html", &context)?;
        let output_path = PathBuf::from(&self.config.output_dir)
            .join("tag")
            .join(tag)
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    /// Generate tags overview page (list of all tags)
    fn generate_tags_overview(&self, metadata: &MetadataCache) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count, descending

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("config", &self.config);

        let output = self.tera.render("tags.html", &context)?;
        let output_path = PathBuf::from(&self.config.output_dir)
            .join("tags")
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }
}
