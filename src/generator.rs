use crate::config::{AssetsConfig, SsgConfig};
use crate::slug;
use crate::types::{Page, Post};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera, Value};

/// Flattened config for template context (backward compatibility)
#[derive(Debug, Clone, Serialize)]
struct TemplateConfig<'a> {
    site_title: &'a str,
    site_url: &'a str,
    author: &'a str,
    description: &'a str,
    assets: &'a AssetsConfig,
}

pub struct Generator {
    tera: Tera,
    config: SsgConfig,
}

impl Generator {
    pub fn new(config: SsgConfig) -> Result<Self> {
        let tera = create_tera_engine()?;

        Ok(Self { tera, config })
    }

    pub fn generate_post(
        &self,
        post: &Post,
        plugin_data: &HashMap<String, JsonValue>,
    ) -> Result<PathBuf> {
        let html = post
            .rendered_html
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Post not rendered: {}", post.slug))?;

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
            description: &self.config.site.description,
            assets: &self.config.assets,
        };

        let mut context = TeraContext::new();
        context.insert("post", post);
        context.insert("slug", &post.slug);
        context.insert("category", &post.category);
        context.insert("content", html);
        context.insert("config", &template_config);

        for (key, value) in plugin_data {
            context.insert(key, value);
        }

        let output = self.tera.render("post.html", &context)?;

        let output_path = self.get_post_path(post);
        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(output_path)
    }

    pub fn generate_page(
        &self,
        page: &Page,
        plugin_data: &HashMap<String, JsonValue>,
    ) -> Result<PathBuf> {
        let html = page
            .rendered_html
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Page not rendered: {}", page.slug))?;

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
            description: &self.config.site.description,
            assets: &self.config.assets,
        };

        let mut context = TeraContext::new();
        context.insert("page", &page.frontmatter);
        context.insert("slug", &page.slug);
        context.insert("content", html);
        context.insert("config", &template_config);

        for (key, value) in plugin_data {
            context.insert(key, value);
        }

        let output = self.tera.render("page.html", &context)?;

        let output_path = self.get_page_path(page);
        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(output_path)
    }

    pub fn get_tera(&self) -> &Tera {
        &self.tera
    }

    fn get_post_path(&self, post: &Post) -> PathBuf {
        let category = self.maybe_encode(&post.category);
        let slug = self.maybe_encode(&post.slug);

        PathBuf::from(&self.config.build.output_dir)
            .join(category)
            .join(slug)
            .join("index.html")
    }

    fn get_page_path(&self, page: &Page) -> PathBuf {
        let slug = self.maybe_encode(&page.slug);

        PathBuf::from(&self.config.build.output_dir)
            .join(slug)
            .join("index.html")
    }

    fn maybe_encode(&self, s: &str) -> String {
        if self.config.build.encode_filenames {
            slug::encode_for_url(s)
        } else {
            s.to_string()
        }
    }

    pub fn copy_static_assets(&self) -> Result<()> {
        let dst = Path::new(&self.config.build.output_dir);

        let src = Path::new("static");
        if src.exists() {
            Self::copy_dir_all(src, dst)?;
            println!("📦 Copied static assets");
        }

        Ok(())
    }

    pub fn copy_content_assets(&self) -> Result<()> {
        let content_dir = Path::new(&self.config.build.content_dir);
        let output_dir = Path::new(&self.config.build.output_dir);

        if !content_dir.exists() {
            return Ok(());
        }

        let mut copied_count = 0;

        for entry in walkdir::WalkDir::new(content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "md" {
                    continue;
                }

                let is_image = matches!(
                    ext_str.as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp"
                );

                let is_media = matches!(ext_str.as_str(), "mp4" | "webm" | "mp3" | "wav");

                let is_document = matches!(ext_str.as_str(), "pdf" | "zip" | "tar" | "gz");

                if is_image || is_media || is_document {
                    let relative_path = path.strip_prefix(content_dir)?;
                    let encoded_path = Self::encode_asset_path(relative_path);
                    let full_output_path = output_dir.join(&encoded_path);

                    if let Some(parent) = full_output_path.parent() {
                        fs::create_dir_all(parent).with_context(|| {
                            format!("Failed to create directory for: {}", encoded_path.display())
                        })?;
                    }

                    fs::copy(path, &full_output_path).with_context(|| {
                        format!(
                            "Failed to copy {} to {}",
                            path.display(),
                            full_output_path.display()
                        )
                    })?;
                    copied_count += 1;
                }
            }
        }

        if copied_count > 0 {
            println!("📦 Copied {} asset(s) from content directory", copied_count);
        }

        Ok(())
    }

    fn encode_asset_path(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

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
