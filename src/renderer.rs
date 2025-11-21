use anyhow::Result;
use pulldown_cmark::{html, Options, Parser as MdParser};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{css_for_theme_with_class_style, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use tera::{Context, Tera};

use crate::image::ImageProcessor;

pub struct Renderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn render_markdown(&self, markdown: &str) -> String {
        let options = Options::all();
        let parser = MdParser::new_ext(markdown, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        self.highlight_code_blocks(&html_output)
    }

    pub fn render_markdown_with_components(
        &self,
        markdown: &str,
        tera: &Tera,
        base_path: &str,
    ) -> Result<String> {
        self.render_markdown_with_components_and_images(markdown, tera, base_path, None, None)
    }

    pub fn render_markdown_with_components_and_images(
        &self,
        markdown: &str,
        tera: &Tera,
        base_path: &str,
        cdn_url: Option<&str>,
        content_dir: Option<&Path>,
    ) -> Result<String> {
        let options = Options::all();
        let parser = MdParser::new_ext(markdown, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // Apply syntax highlighting first
        let highlighted = self.highlight_code_blocks(&html_output);

        // Then apply component templates
        Self::post_process_components(&highlighted, tera, base_path, cdn_url, content_dir)
    }

    fn post_process_components(
        html: &str,
        tera: &Tera,
        base_path: &str,
        cdn_url: Option<&str>,
        content_dir: Option<&Path>,
    ) -> Result<String> {
        let mut result = html.to_string();

        let tag_patterns = vec![
            "img",
            "code",
            "pre",
            "blockquote",
            "table",
            "a",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "p",
            "ul",
            "ol",
            "li",
            "strong",
            "em",
            "del",
        ];

        // Create image processor if CDN URL is configured
        let image_processor = cdn_url.map(|url| ImageProcessor::new(Some(url.to_string())));

        for tag_name in tag_patterns {
            let template_name = format!("components/{}.html", tag_name);

            if tera.get_template(&template_name).is_err() {
                continue;
            }

            result = Self::replace_tag(
                &result,
                tag_name,
                tera,
                &template_name,
                base_path,
                image_processor.as_ref(),
                content_dir,
            )?;
        }

        Ok(result)
    }

    fn replace_tag(
        html: &str,
        tag_name: &str,
        tera: &Tera,
        template_name: &str,
        base_path: &str,
        image_processor: Option<&ImageProcessor>,
        content_dir: Option<&Path>,
    ) -> Result<String> {
        let mut result = String::new();
        let mut chars = html.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '<' {
                let tag_start_pos = result.len();
                result.push(ch);

                let mut tag_content = String::new();
                let mut in_quotes = false;
                let mut quote_char = ' ';

                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    result.push(next_ch);

                    if next_ch == '"' || next_ch == '\'' {
                        if in_quotes && next_ch == quote_char {
                            in_quotes = false;
                        } else if !in_quotes {
                            in_quotes = true;
                            quote_char = next_ch;
                        }
                    }

                    if next_ch == '>' && !in_quotes {
                        tag_content = result[tag_start_pos..].to_string();
                        break;
                    }
                }

                if tag_content.starts_with(&format!("<{} ", tag_name))
                    || tag_content == format!("<{}>", tag_name)
                {
                    let attrs = Self::extract_attributes(&tag_content);
                    let mut inner_content = String::new();

                    if !tag_content.ends_with("/>") {
                        let mut depth = 1;
                        let close_tag = format!("</{}>", tag_name);

                        while depth > 0 && chars.peek().is_some() {
                            let ch = chars.next().unwrap();

                            if ch == '<' {
                                let mut potential_tag = String::from('<');
                                while let Some(&next_ch) = chars.peek() {
                                    chars.next();
                                    potential_tag.push(next_ch);
                                    if next_ch == '>' {
                                        break;
                                    }
                                }

                                if potential_tag == close_tag {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                } else if potential_tag.starts_with(&format!("<{} ", tag_name))
                                    || potential_tag == format!("<{}>", tag_name)
                                {
                                    depth += 1;
                                }

                                if depth > 0 {
                                    inner_content.push_str(&potential_tag);
                                }
                            } else {
                                inner_content.push(ch);
                            }
                        }
                    }

                    let mut context = Context::new();
                    let mut original_src = String::new();

                    for (key, value) in &attrs {
                        if Self::is_url_attribute(key) {
                            let resolved = Self::resolve_path(value, base_path);
                            context.insert(key, &resolved);
                            if key == "src" {
                                original_src = value.clone();
                            }
                        } else {
                            context.insert(key, value);
                        }
                    }

                    // Process image for CDN srcset if this is an img tag
                    if tag_name == "img" {
                        if let (Some(processor), Some(content_path)) =
                            (image_processor, content_dir)
                        {
                            let category = base_path.split('/').next().unwrap_or(base_path);
                            let post_content_dir = content_path.join(category.trim_matches('/'));

                            if let Ok(Some(metadata)) =
                                processor.process_image(&original_src, &post_content_dir)
                            {
                                context.insert("cdn_src", &metadata.src);
                                context.insert("srcset", &metadata.srcset);
                                context.insert("webp_srcset", &metadata.webp_srcset);
                                context.insert("width", &metadata.width);
                                context.insert("height", &metadata.height);
                                context.insert("has_srcset", &true);
                            } else {
                                context.insert("has_srcset", &false);
                            }
                        } else {
                            context.insert("has_srcset", &false);
                        }
                    }

                    if !inner_content.is_empty() {
                        context.insert("content", &inner_content);
                    }

                    if let Ok(rendered) = tera.render(template_name, &context) {
                        result.truncate(tag_start_pos);
                        result.push_str(&rendered);
                        continue;
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    fn extract_attributes(tag: &str) -> HashMap<String, String> {
        let mut attrs = HashMap::new();

        let tag = tag
            .trim_start_matches('<')
            .trim_end_matches('>')
            .trim_end_matches('/');
        let parts: Vec<&str> = tag.splitn(2, ' ').collect();

        if parts.len() < 2 {
            return attrs;
        }

        let attr_string = parts[1];
        let mut chars = attr_string.chars().peekable();

        while chars.peek().is_some() {
            while chars.peek() == Some(&' ') {
                chars.next();
            }

            let mut key = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '=' || ch == ' ' {
                    break;
                }
                key.push(chars.next().unwrap());
            }

            if key.is_empty() {
                break;
            }

            while chars.peek() == Some(&' ') {
                chars.next();
            }

            if chars.peek() != Some(&'=') {
                attrs.insert(key, String::from("true"));
                continue;
            }

            chars.next();

            while chars.peek() == Some(&' ') {
                chars.next();
            }

            let mut value = String::new();
            if let Some(&quote) = chars.peek() {
                if quote == '"' || quote == '\'' {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        if ch == quote {
                            chars.next();
                            break;
                        }
                        value.push(chars.next().unwrap());
                    }
                } else {
                    while let Some(&ch) = chars.peek() {
                        if ch == ' ' {
                            break;
                        }
                        value.push(chars.next().unwrap());
                    }
                }
            }

            attrs.insert(key, value);
        }

        attrs
    }

    fn is_url_attribute(attr: &str) -> bool {
        matches!(attr, "src" | "href" | "data" | "poster" | "srcset")
    }

    fn resolve_path(path: &str, base_path: &str) -> String {
        let trimmed = path.trim();

        if trimmed.is_empty() {
            return "/".to_string();
        }

        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("data:")
            || trimmed.starts_with("mailto:")
        {
            return trimmed.to_string();
        }

        if trimmed.starts_with('/') {
            return trimmed.to_string();
        }

        let category = base_path.split('/').next().unwrap_or("").trim_matches('/');

        if trimmed.starts_with("./") {
            let mut relative_path = &trimmed[2..];

            // Strip any additional ./ patterns (e.g., "././image.png")
            while relative_path.starts_with("./") {
                relative_path = &relative_path[2..];
            }

            let relative_path = relative_path.trim_start_matches('/');

            if relative_path.is_empty() {
                return if category.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", category)
                };
            }

            return if category.is_empty() {
                format!("/{}", relative_path)
            } else {
                format!("/{}/{}", category, relative_path)
            };
        }

        if trimmed.starts_with("../") {
            let base_parts: Vec<&str> = category.split('/').filter(|s| !s.is_empty()).collect();

            let mut remaining_path = trimmed;
            let mut up_count = 0;
            while remaining_path.starts_with("../") {
                up_count += 1;
                remaining_path = &remaining_path[3..];
            }

            let remaining_base = if up_count >= base_parts.len() {
                vec![]
            } else {
                base_parts[..base_parts.len() - up_count].to_vec()
            };

            let remaining_path = remaining_path.trim_start_matches('/');
            if remaining_base.is_empty() && remaining_path.is_empty() {
                return "/".to_string();
            }

            let mut result = String::from("/");
            if !remaining_base.is_empty() {
                result.push_str(&remaining_base.join("/"));
                if !remaining_path.is_empty() {
                    result.push('/');
                }
            }
            if !remaining_path.is_empty() {
                result.push_str(remaining_path);
            }

            return result;
        }

        let trimmed = trimmed.trim_start_matches('/');
        if category.is_empty() {
            format!("/{}", trimmed)
        } else {
            format!("/{}/{}", category, trimmed)
        }
    }

    fn highlight_code_blocks(&self, html: &str) -> String {
        let mut result = String::new();
        let mut chars = html.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '<' {
                let start_pos = result.len();
                result.push(ch);

                // Check if this is the start of a <pre> tag
                let mut tag_buf = String::from("<");
                let mut is_pre_tag = false;

                // Read until we hit '>'
                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    result.push(next_ch);
                    tag_buf.push(next_ch);

                    if next_ch == '>' {
                        if tag_buf.starts_with("<pre>") || tag_buf.starts_with("<pre ") {
                            is_pre_tag = true;
                        }
                        break;
                    }
                }

                // If this is a <pre> tag, look for <code> inside
                if is_pre_tag {
                    // Collect everything until </pre>
                    let mut pre_content = String::new();
                    let mut depth = 1;

                    while depth > 0 && chars.peek().is_some() {
                        let ch = chars.next().unwrap();

                        if ch == '<' {
                            let mut potential_tag = String::from('<');
                            while let Some(&next_ch) = chars.peek() {
                                chars.next();
                                potential_tag.push(next_ch);
                                if next_ch == '>' {
                                    break;
                                }
                            }

                            if potential_tag == "</pre>" {
                                depth -= 1;
                                if depth == 0 {
                                    // Process the pre_content for code highlighting
                                    if let Some(highlighted) =
                                        self.process_pre_content(&pre_content)
                                    {
                                        // Replace the accumulated content with highlighted version
                                        result.truncate(start_pos);
                                        result.push_str(&highlighted);
                                    } else {
                                        // Keep original
                                        result.push_str(&pre_content);
                                        result.push_str("</pre>");
                                    }
                                    break;
                                }
                            }

                            pre_content.push_str(&potential_tag);
                        } else {
                            pre_content.push(ch);
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn process_pre_content(&self, content: &str) -> Option<String> {
        // Look for <code class="language-XXX">...</code>
        let content = content.trim();

        if !content.starts_with("<code") {
            return None;
        }

        // Extract language from class attribute
        let lang = if let Some(class_start) = content.find("class=\"language-") {
            let lang_start = class_start + "class=\"language-".len();
            if let Some(quote_end) = content[lang_start..].find('"') {
                Some(&content[lang_start..lang_start + quote_end])
            } else {
                None
            }
        } else {
            None
        };

        // Extract code content
        let code_start = content.find('>')? + 1;
        let code_end = content.rfind("</code>")?;
        let code = &content[code_start..code_end];

        // Decode HTML entities
        let decoded_code = Self::decode_html_entities(code);

        // Apply syntax highlighting if language is specified
        if let Some(language) = lang {
            if let Ok(highlighted) = self.highlight_code(&decoded_code, language) {
                // Syntect already wraps in <pre>, so we don't need to add it
                return Some(highlighted);
            }
        }

        // Return None to keep original if highlighting fails
        None
    }

    fn decode_html_entities(html: &str) -> String {
        html.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
    }

    pub fn highlight_code(&self, code: &str, lang: &str) -> Result<String> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use ClassedHTMLGenerator for CSS class-based highlighting
        let mut html_generator = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &self.syntax_set,
            ClassStyle::Spaced,
        );

        for line in LinesWithEndings::from(code) {
            html_generator.parse_html_for_line_which_includes_newline(line)?;
        }

        Ok(format!(
            "<pre class=\"syntax-highlight\"><code>{}</code></pre>",
            html_generator.finalize()
        ))
    }

    /// Generate CSS for syntax highlighting themes
    pub fn generate_theme_css(&self) -> Result<String> {
        let mut css = String::new();

        // CSS Variables for theming
        css.push_str(":root {\n");
        css.push_str("  color-scheme: light dark;\n");
        css.push_str("}\n\n");

        // Light theme (default)
        css.push_str("/* Light theme */\n");
        css.push_str("@media (prefers-color-scheme: light) {\n");
        css.push_str("  :root {\n");
        let light_theme = &self.theme_set.themes["base16-ocean.light"];
        Self::add_theme_variables(&mut css, light_theme, "    ");
        css.push_str("  }\n");
        css.push_str("}\n\n");

        // Dark theme
        css.push_str("/* Dark theme */\n");
        css.push_str("@media (prefers-color-scheme: dark) {\n");
        css.push_str("  :root {\n");
        let dark_theme = &self.theme_set.themes["base16-ocean.dark"];
        Self::add_theme_variables(&mut css, dark_theme, "    ");
        css.push_str("  }\n");
        css.push_str("}\n\n");

        // Generate base CSS for syntax classes using dark theme as reference
        let dark_theme = &self.theme_set.themes["base16-ocean.dark"];
        let theme_css = css_for_theme_with_class_style(dark_theme, ClassStyle::Spaced)?;

        // Convert to CSS variables
        css.push_str(&Self::convert_css_to_variables(&theme_css));

        // Add base styles
        css.push_str("\n.syntax-highlight {\n");
        css.push_str("  background-color: var(--syntax-bg);\n");
        css.push_str("  color: var(--syntax-fg);\n");
        css.push_str("  padding: 1em;\n");
        css.push_str("  overflow-x: auto;\n");
        css.push_str("  border-radius: 4px;\n");
        css.push_str("}\n\n");
        css.push_str(".syntax-highlight code {\n");
        css.push_str("  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;\n");
        css.push_str("  font-size: 0.9em;\n");
        css.push_str("  line-height: 1.2;\n");
        css.push_str("}\n");

        Ok(css)
    }

    fn add_theme_variables(css: &mut String, theme: &Theme, indent: &str) {
        css.push_str(&format!(
            "{}--syntax-bg: {};\n",
            indent,
            Self::color_to_css(
                &theme
                    .settings
                    .background
                    .unwrap_or(syntect::highlighting::Color::WHITE)
            )
        ));
        css.push_str(&format!(
            "{}--syntax-fg: {};\n",
            indent,
            Self::color_to_css(
                &theme
                    .settings
                    .foreground
                    .unwrap_or(syntect::highlighting::Color::BLACK)
            )
        ));
    }

    fn color_to_css(color: &syntect::highlighting::Color) -> String {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    }

    fn convert_css_to_variables(css: &str) -> String {
        // Replace hardcoded colors with CSS variables in the generated CSS
        // This is a simplified version - we'll use the variables defined above
        css.replace(
            "background-color:#",
            "background-color: var(--syntax-bg); /* #",
        )
        .replace("color:#", "color: var(--syntax-fg); /* #")
    }

    /// Write syntax highlighting CSS to file
    pub fn write_syntax_css<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let css = self.generate_theme_css()?;
        fs::write(output_path, css)?;
        Ok(())
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown() {
        let renderer = Renderer::new();
        let md = "# Hello\n\nThis is **bold**.";
        let html = renderer.render_markdown(md);

        assert!(html.contains("<h1>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_render_markdown_with_code() {
        let renderer = Renderer::new();
        let md = "```rust\nfn main() {}\n```";
        let html = renderer.render_markdown(md);

        // Check for class-based highlighting structure
        assert!(html.contains("<pre class=\"syntax-highlight\">"));
        assert!(html.contains("<code>"));
        // Check that code content is present (even if wrapped in spans)
        assert!(html.contains("fn"));
        assert!(html.contains("main"));
    }

    #[test]
    fn test_render_markdown_with_links() {
        let renderer = Renderer::new();
        let md = "[Click here](https://example.com)";
        let html = renderer.render_markdown(md);

        assert!(html.contains("<a href=\"https://example.com\">"));
        assert!(html.contains("Click here"));
    }
}
