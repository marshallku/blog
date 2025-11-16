use anyhow::Result;
use pulldown_cmark::{html, Event, Options, Parser as MdParser, Tag};
use std::borrow::Cow;
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
        cdn_url: Option<&str>,
    ) -> Result<String> {
        let options = Options::all();
        let parser = MdParser::new_ext(markdown, options);

        let mut events = Vec::new();
        let mut image_data: Option<(Cow<str>, Cow<str>)> = None;

        for event in parser {
            match event {
                Event::Start(Tag::Image(_, dest_url, _)) => {
                    image_data = Some((dest_url.clone().into(), Cow::Borrowed("")));
                }
                Event::Text(ref text) if image_data.is_some() => {
                    if let Some((url, _)) = image_data.take() {
                        let alt = text.to_string();
                        let src = if let Some(cdn) = cdn_url {
                            if url.starts_with("http") || url.starts_with("//") {
                                url.to_string()
                            } else {
                                format!("{}{}", cdn, url)
                            }
                        } else {
                            url.to_string()
                        };

                        if tera.get_template("components/img.html").is_ok() {
                            let mut context = Context::new();
                            context.insert("src", &src);
                            context.insert("alt", &alt);

                            if let Ok(rendered) = tera.render("components/img.html", &context) {
                                events.push(Event::Html(rendered.into()));
                                continue;
                            }
                        }

                        let default_html = format!(r#"<img src="{}" alt="{}">"#, src, alt);
                        events.push(Event::Html(default_html.into()));
                    }
                }
                Event::End(Tag::Image(..)) => {
                    continue;
                }
                _ => {
                    events.push(event);
                }
            }
        }

        let mut html_output = String::new();
        html::push_html(&mut html_output, events.into_iter());

        Ok(html_output)
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
