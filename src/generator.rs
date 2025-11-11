use crate::types::{Config, Post};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

pub struct Generator {
    tera: Tera,
    config: Config,
}

impl Generator {
    pub fn new(config: Config) -> Result<Self> {
        let template_glob = format!("{}/**/*.html", config.template_dir);
        let tera = Tera::new(&template_glob)
            .context("Failed to load templates")?;

        Ok(Self { tera, config })
    }

    /// Generate a single post page
    pub fn generate_post(&self, post: &Post) -> Result<PathBuf> {
        let html = post.rendered_html.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Post not rendered: {}", post.slug))?;

        let mut context = TeraContext::new();
        context.insert("post", &post.frontmatter);
        context.insert("slug", &post.slug);
        context.insert("content", html);
        context.insert("config", &self.config);

        let output = self.tera.render("post.html", &context)?;

        // Write to dist/{category}/{slug}/index.html
        let output_path = self.get_post_path(post);
        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(output_path)
    }

    /// Get the output path for a post
    fn get_post_path(&self, post: &Post) -> PathBuf {
        PathBuf::from(&self.config.output_dir)
            .join(&post.frontmatter.category)
            .join(&post.slug)
            .join("index.html")
    }

    /// Copy static assets from static/ to dist/
    pub fn copy_static_assets(&self) -> Result<()> {
        let src = Path::new("static");
        let dst = Path::new(&self.config.output_dir);

        if !src.exists() {
            println!("No static/ directory found, skipping asset copy");
            return Ok(());
        }

        Self::copy_dir_all(src, dst)?;
        println!("Copied static assets");

        Ok(())
    }

    /// Recursively copy directory
    fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if ty.is_dir() {
                Self::copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }
}
