use anyhow::Result;
use pulldown_cmark::{html, Options, Parser as MdParser};
use std::collections::HashMap;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use tera::{Context, Tera};

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

        html_output
    }

    pub fn render_markdown_with_components(
        &self,
        markdown: &str,
        tera: &Tera,
    ) -> Result<String> {
        let options = Options::all();
        let parser = MdParser::new_ext(markdown, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        Self::post_process_components(&html_output, tera)
    }

    fn post_process_components(html: &str, tera: &Tera) -> Result<String> {
        let mut result = html.to_string();

        let tag_patterns = vec![
            "img", "code", "pre", "blockquote", "table", "a", "h1", "h2", "h3",
            "h4", "h5", "h6", "p", "ul", "ol", "li", "strong", "em", "del"
        ];

        for tag_name in tag_patterns {
            let template_name = format!("components/{}.html", tag_name);

            if tera.get_template(&template_name).is_err() {
                continue;
            }

            result = Self::replace_tag(&result, tag_name, tera, &template_name)?;
        }

        Ok(result)
    }

    fn replace_tag(
        html: &str,
        tag_name: &str,
        tera: &Tera,
        template_name: &str,
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
                    || tag_content == format!("<{}>", tag_name) {

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
                                    || potential_tag == format!("<{}>", tag_name) {
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
                    for (key, value) in attrs {
                        context.insert(&key, &value);
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

        let tag = tag.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
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

    #[allow(dead_code)]
    pub fn highlight_code(&self, code: &str, lang: &str) -> Result<String> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self.get_theme();

        Ok(highlighted_html_for_string(
            code,
            &self.syntax_set,
            syntax,
            theme,
        )?)
    }

    fn get_theme(&self) -> &Theme {
        &self.theme_set.themes["base16-ocean.dark"]
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

        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
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
