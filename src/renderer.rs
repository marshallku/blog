use anyhow::Result;
use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser as MdParser, Tag,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use tera::{Context, Tera};

use crate::image::ImageProcessor;
use crate::syntax_highlighter::SyntaxHighlighter;

const COMPONENT_TAGS: &[&str] = &[
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
    "iframe",
];

const RAW_HTML_TAGS: &'static [&'static str] = &[
    "video", "audio", "source", "iframe", "embed", "object", "track",
];

struct TagReplacementContext<'a> {
    tera: &'a Tera,
    template_name: &'a str,
    category: &'a str,
    base_path: &'a str,
    image_processor: Option<&'a ImageProcessor>,
    content_dir: Option<&'a Path>,
}

pub struct Renderer {
    highlighter: RefCell<SyntaxHighlighter>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            highlighter: RefCell::new(
                SyntaxHighlighter::new().expect("Failed to initialize syntax highlighter"),
            ),
        }
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

        let mut html_output = String::with_capacity(markdown.len() * 2);
        Self::push_html_with_markers(&mut html_output, parser);

        let highlighted = self.highlight_code_blocks(&html_output);
        Self::post_process_components(&highlighted, tera, base_path, cdn_url, content_dir)
    }

    /// Custom HTML writer that adds `data-md` attribute to markdown-generated tags.
    /// This allows component templates to distinguish between markdown syntax
    /// (e.g., `![]()`→`<img>`) and raw HTML tags written directly in markdown.
    fn push_html_with_markers<'a, I>(output: &mut String, iter: I)
    where
        I: Iterator<Item = Event<'a>>,
    {
        let mut in_code_block = false;

        for event in iter {
            match event {
                Event::Start(tag) => Self::write_start_tag(output, &tag, &mut in_code_block),
                Event::End(tag) => Self::write_end_tag(output, &tag, &mut in_code_block),
                Event::Text(text) => {
                    Self::escape_html(output, &text);
                }
                Event::Code(text) => {
                    output.push_str("<code data-md>");
                    Self::escape_html(output, &text);
                    output.push_str("</code>");
                }
                Event::Html(html) => {
                    output.push_str(&html);
                }
                Event::SoftBreak => output.push('\n'),
                Event::HardBreak => output.push_str("<br />\n"),
                Event::Rule => output.push_str("<hr data-md />\n"),
                Event::FootnoteReference(name) => {
                    output.push_str("<sup class=\"footnote-reference\" data-md><a href=\"#");
                    Self::escape_html(output, &name);
                    output.push_str("\">");
                    Self::escape_html(output, &name);
                    output.push_str("</a></sup>");
                }
                Event::TaskListMarker(checked) => {
                    output.push_str("<input type=\"checkbox\" disabled");
                    if checked {
                        output.push_str(" checked");
                    }
                    output.push_str(" />\n");
                }
            }
        }
    }

    fn write_start_tag(output: &mut String, tag: &Tag<'_>, in_code_block: &mut bool) {
        match tag {
            Tag::Paragraph => output.push_str("<p data-md>"),
            Tag::Heading(level, id, _classes) => {
                output.push('<');
                output.push_str(Self::heading_level_str(*level));
                output.push_str(" data-md");
                if let Some(id) = id {
                    output.push_str(" id=\"");
                    Self::escape_html(output, id);
                    output.push('"');
                }
                output.push('>');
            }
            Tag::BlockQuote => output.push_str("<blockquote data-md>\n"),
            Tag::CodeBlock(kind) => {
                *in_code_block = true;
                output.push_str("<pre data-md>");
                match kind {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info.split(' ').next().unwrap_or("");
                        if lang.is_empty() {
                            output.push_str("<code>");
                        } else {
                            output.push_str("<code class=\"language-");
                            Self::escape_html(output, lang);
                            output.push_str("\">");
                        }
                    }
                    CodeBlockKind::Indented => output.push_str("<code>"),
                }
            }
            Tag::List(Some(start)) => {
                if *start != 1 {
                    output.push_str("<ol data-md start=\"");
                    output.push_str(&start.to_string());
                    output.push_str("\">\n");
                } else {
                    output.push_str("<ol data-md>\n");
                }
            }
            Tag::List(None) => output.push_str("<ul data-md>\n"),
            Tag::Item => output.push_str("<li data-md>"),
            Tag::FootnoteDefinition(name) => {
                output.push_str("<div class=\"footnote-definition\" id=\"");
                Self::escape_html(output, name);
                output.push_str("\" data-md><sup class=\"footnote-definition-label\">");
                Self::escape_html(output, name);
                output.push_str("</sup>");
            }
            Tag::Table(_) => output.push_str("<table data-md>"),
            Tag::TableHead => output.push_str("<thead data-md><tr data-md>"),
            Tag::TableRow => output.push_str("<tr data-md>"),
            Tag::TableCell => output.push_str("<td data-md>"),
            Tag::Emphasis => output.push_str("<em data-md>"),
            Tag::Strong => output.push_str("<strong data-md>"),
            Tag::Strikethrough => output.push_str("<del data-md>"),
            Tag::Link(_link_type, dest_url, title) => {
                output.push_str("<a data-md href=\"");
                Self::escape_href(output, dest_url);
                if !title.is_empty() {
                    output.push_str("\" title=\"");
                    Self::escape_html(output, title);
                }
                output.push_str("\">");
            }
            Tag::Image(_link_type, dest_url, _title) => {
                output.push_str("<img data-md src=\"");
                Self::escape_href(output, dest_url);
                output.push_str("\" alt=\"");
                // Alt text will be added by subsequent Text events
            }
        }
    }

    fn write_end_tag(output: &mut String, tag: &Tag<'_>, in_code_block: &mut bool) {
        match tag {
            Tag::Paragraph => output.push_str("</p>\n"),
            Tag::Heading(level, _, _) => {
                output.push_str("</");
                output.push_str(Self::heading_level_str(*level));
                output.push_str(">\n");
            }
            Tag::BlockQuote => output.push_str("</blockquote>\n"),
            Tag::CodeBlock(_) => {
                *in_code_block = false;
                output.push_str("</code></pre>\n");
            }
            Tag::List(Some(_)) => output.push_str("</ol>\n"),
            Tag::List(None) => output.push_str("</ul>\n"),
            Tag::Item => output.push_str("</li>\n"),
            Tag::FootnoteDefinition(_) => output.push_str("</div>\n"),
            Tag::Table(_) => output.push_str("</table>\n"),
            Tag::TableHead => output.push_str("</tr></thead>\n"),
            Tag::TableRow => output.push_str("</tr>\n"),
            Tag::TableCell => output.push_str("</td>"),
            Tag::Emphasis => output.push_str("</em>"),
            Tag::Strong => output.push_str("</strong>"),
            Tag::Strikethrough => output.push_str("</del>"),
            Tag::Link(_, _, _) => output.push_str("</a>"),
            Tag::Image(_, _, _) => {
                // Image alt text was accumulated, now close the tag
                output.push_str("\" />");
            }
        }
    }

    fn heading_level_str(level: HeadingLevel) -> &'static str {
        match level {
            HeadingLevel::H1 => "h1",
            HeadingLevel::H2 => "h2",
            HeadingLevel::H3 => "h3",
            HeadingLevel::H4 => "h4",
            HeadingLevel::H5 => "h5",
            HeadingLevel::H6 => "h6",
        }
    }

    fn escape_html(output: &mut String, text: &str) {
        for c in text.chars() {
            match c {
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '&' => output.push_str("&amp;"),
                '"' => output.push_str("&quot;"),
                _ => output.push(c),
            }
        }
    }

    fn escape_href(output: &mut String, href: &CowStr<'_>) {
        for c in href.chars() {
            match c {
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '&' => output.push_str("&amp;"),
                '"' => output.push_str("&quot;"),
                '\'' => output.push_str("&#x27;"),
                _ => output.push(c),
            }
        }
    }

    fn post_process_components(
        html: &str,
        tera: &Tera,
        base_path: &str,
        cdn_url: Option<&str>,
        content_dir: Option<&Path>,
    ) -> Result<String> {
        let mut result = html.to_string();

        let category = base_path
            .rsplit_once('/')
            .map(|(cat, _slug)| cat)
            .unwrap_or(base_path);

        let image_processor = cdn_url.map(|url| ImageProcessor::new(Some(url.to_string())));

        for &tag_name in COMPONENT_TAGS {
            let template_name = format!("components/{}.html", tag_name);

            if tera.get_template(&template_name).is_err() {
                continue;
            }

            let ctx = TagReplacementContext {
                tera,
                template_name: &template_name,
                category,
                base_path,
                image_processor: image_processor.as_ref(),
                content_dir,
            };
            result = Self::replace_tag(&result, tag_name, &ctx)?;
        }

        result = Self::resolve_raw_html_paths(&result, category);

        Ok(Self::sanitize(&result))
    }

    fn sanitize(html: &str) -> String {
        html.replace(" data-md", "")
    }

    /// Resolve relative paths in raw HTML tags (video, audio, source, etc.)
    fn resolve_raw_html_paths(html: &str, category: &str) -> String {
        let mut result = String::with_capacity(html.len());
        let mut remaining = html;

        while let Some(tag_start) = remaining.find('<') {
            result.push_str(&remaining[..tag_start]);
            remaining = &remaining[tag_start..];

            if let Some(tag_end) = Self::find_tag_end(remaining) {
                let tag = &remaining[..=tag_end];

                // Skip tags with data-md marker (handled by component system)
                if tag.contains(" data-md") {
                    result.push_str(tag);
                } else {
                    let tag_lower = tag.to_lowercase();
                    let needs_processing = RAW_HTML_TAGS
                        .iter()
                        .any(|&t| tag_lower.starts_with(&format!("<{} ", t)));

                    if needs_processing {
                        result.push_str(&Self::resolve_tag_urls(tag, category));
                    } else {
                        result.push_str(tag);
                    }
                }
                remaining = &remaining[tag_end + 1..];
            } else {
                result.push_str(remaining);
                break;
            }
        }
        result.push_str(remaining);
        result
    }

    fn find_tag_end(s: &str) -> Option<usize> {
        let mut in_quotes = false;
        let mut quote_char = ' ';

        for (i, ch) in s.char_indices() {
            if ch == '"' || ch == '\'' {
                if in_quotes && ch == quote_char {
                    in_quotes = false;
                } else if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                }
            } else if ch == '>' && !in_quotes {
                return Some(i);
            }
        }
        None
    }

    fn resolve_tag_urls(tag: &str, category: &str) -> String {
        let mut result = tag.to_string();

        for attr in &["src", "poster", "data"] {
            for quote in &['"', '\''] {
                let pattern = format!("{}={}", attr, quote);
                if let Some(start) = result.find(&pattern) {
                    let value_start = start + pattern.len();
                    if let Some(end_offset) = result[value_start..].find(*quote) {
                        let value_end = value_start + end_offset;
                        let value = &result[value_start..value_end];

                        if value.starts_with("./") || value.starts_with("../") {
                            let resolved = Self::resolve_path(value, category);
                            result = format!(
                                "{}{}{}{}",
                                &result[..value_start],
                                resolved,
                                quote,
                                &result[value_end + 1..]
                            );
                        }
                    }
                }
            }
        }
        result
    }

    fn replace_tag(html: &str, tag_name: &str, ctx: &TagReplacementContext) -> Result<String> {
        let mut result = String::with_capacity(html.len() + html.len() / 10);
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

                // Process markdown-generated tags (data-md marker) or raw HTML tags
                let is_target_tag = tag_content.starts_with(&format!("<{} ", tag_name))
                    || tag_content == format!("<{}>", tag_name);
                let has_md_marker =
                    tag_content.contains(" data-md") || tag_content.contains(" data-md ");
                let is_raw_html_tag = RAW_HTML_TAGS.contains(&tag_name);

                if is_target_tag && (has_md_marker || is_raw_html_tag) {
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
                        if key == "data-md" {
                            continue;
                        }
                        if Self::is_url_attribute(key) {
                            let resolved = Self::resolve_path(value, ctx.category);
                            context.insert(key, &resolved);
                            if key == "src" {
                                original_src = value.clone();
                            }
                        } else {
                            context.insert(key, value);
                        }
                    }

                    if tag_name == "img" {
                        if let (Some(processor), Some(content_path)) =
                            (ctx.image_processor, ctx.content_dir)
                        {
                            let post_content_dir =
                                content_path.join(ctx.category.trim_matches('/'));

                            if let Ok(Some(metadata)) =
                                processor.process_image(&original_src, &post_content_dir, ctx.base_path)
                            {
                                context.insert("cdn_src", &metadata.src);
                                context.insert("lqip", &metadata.lqip);
                                context.insert("sources", &metadata.sources);
                                context.insert("webp_sources", &metadata.webp_sources);
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

                    if let Ok(rendered) = ctx.tera.render(ctx.template_name, &context) {
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

    pub fn resolve_path(path: &str, base_path: &str) -> String {
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

        if let Some(stripped) = trimmed.strip_prefix("./") {
            let mut relative_path = stripped;

            while let Some(next) = relative_path.strip_prefix("./") {
                relative_path = next;
            }

            let relative_path = relative_path.trim_start_matches('/');

            if relative_path.is_empty() {
                return if base_path.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", base_path)
                };
            }

            return if base_path.is_empty() {
                format!("/{}", relative_path)
            } else {
                format!("/{}/{}", base_path, relative_path)
            };
        }

        if trimmed.starts_with("../") {
            let base_parts: Vec<&str> = base_path.split('/').filter(|s| !s.is_empty()).collect();

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
        if base_path.is_empty() {
            format!("/{}", trimmed)
        } else {
            format!("/{}/{}", base_path, trimmed)
        }
    }

    fn highlight_code_blocks(&self, html: &str) -> String {
        let mut result = String::with_capacity(html.len() + html.len() / 5);
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
            content[lang_start..]
                .find('"')
                .map(|quote_end| &content[lang_start..lang_start + quote_end])
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
        self.highlighter.borrow_mut().highlight(code, lang)
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

    impl Renderer {
        fn render_markdown(&self, markdown: &str) -> String {
            let options = Options::all();
            let parser = MdParser::new_ext(markdown, options);
            let mut html_output = String::with_capacity(markdown.len() * 2);
            Self::push_html_with_markers(&mut html_output, parser);
            self.highlight_code_blocks(&html_output)
        }
    }

    #[test]
    fn test_render_markdown() {
        let renderer = Renderer::new();
        let md = "# Hello\n\nThis is **bold**.";
        let html = renderer.render_markdown(md);

        // Note: render_markdown is internal test helper that doesn't strip data-md markers
        // The markers are stripped in post_process_components which is called by the public API
        assert!(html.contains("<h1 data-md>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<strong data-md>bold</strong>"));
    }

    #[test]
    fn test_render_markdown_with_code() {
        let renderer = Renderer::new();
        let md = "```rust\nfn main() {}\n```";
        let html = renderer.render_markdown(md);

        // Check for class-based highlighting structure (autumnus adds "athl hljs" classes)
        assert!(
            html.contains("hljs"),
            "HTML should contain hljs class, got: {}",
            html
        );
        assert!(html.contains("<code"));
        // Check that code content is present (even if wrapped in spans)
        assert!(html.contains("fn"));
        assert!(html.contains("main"));
        // Check that syntax highlighting classes are applied
        assert!(html.contains("keyword"), "Should have keyword highlighting");
        assert!(
            html.contains("function"),
            "Should have function highlighting"
        );
    }

    #[test]
    fn test_render_markdown_with_links() {
        let renderer = Renderer::new();
        let md = "[Click here](https://example.com)";
        let html = renderer.render_markdown(md);

        assert!(html.contains("<a data-md href=\"https://example.com\">"));
        assert!(html.contains("Click here"));
    }

    #[test]
    fn test_markdown_vs_raw_html_distinction() {
        let renderer = Renderer::new();

        // Markdown image should have data-md marker
        let md_image = "![alt text](./image.png)";
        let html = renderer.render_markdown(md_image);
        assert!(
            html.contains("<img data-md"),
            "Markdown image should have data-md marker"
        );

        // Raw HTML image should NOT have data-md marker
        let raw_html_image = r#"<img src="./image.png" alt="raw image">"#;
        let html = renderer.render_markdown(raw_html_image);
        assert!(
            !html.contains("data-md"),
            "Raw HTML image should NOT have data-md marker"
        );
        assert!(
            html.contains(r#"<img src="./image.png" alt="raw image">"#),
            "Raw HTML should pass through unchanged"
        );

        // Markdown link should have data-md marker
        let md_link = "[link](https://example.com)";
        let html = renderer.render_markdown(md_link);
        assert!(
            html.contains("<a data-md"),
            "Markdown link should have data-md marker"
        );

        // Raw HTML link should NOT have data-md marker on the <a> tag itself
        // Note: It will be wrapped in a markdown <p>, but the <a> itself shouldn't have data-md
        let raw_html_link = r#"<a href="https://example.com">raw link</a>"#;
        let html = renderer.render_markdown(raw_html_link);
        assert!(
            !html.contains("<a data-md"),
            "Raw HTML <a> tag should NOT have data-md marker, got: {}",
            html
        );
        // The raw <a> should be preserved as-is (except wrapped in <p>)
        assert!(
            html.contains(r#"<a href="https://example.com">raw link</a>"#),
            "Raw HTML <a> should pass through unchanged"
        );
    }

    #[test]
    fn test_resolve_path_absolute_urls() {
        assert_eq!(
            Renderer::resolve_path("https://example.com/image.png", "chat"),
            "https://example.com/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("http://example.com/image.png", "chat"),
            "http://example.com/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("//cdn.example.com/image.png", "chat"),
            "//cdn.example.com/image.png"
        );
    }

    #[test]
    fn test_resolve_path_absolute_paths() {
        assert_eq!(
            Renderer::resolve_path("/assets/image.png", "chat"),
            "/assets/image.png"
        );
    }

    #[test]
    fn test_resolve_path_current_dir() {
        assert_eq!(
            Renderer::resolve_path("./post/image.png", "chat"),
            "/chat/post/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("./image.png", "dev"),
            "/dev/image.png"
        );
    }

    #[test]
    fn test_resolve_path_current_dir_edge_cases() {
        assert_eq!(Renderer::resolve_path("./", "chat"), "/chat");
        assert_eq!(Renderer::resolve_path("./", ""), "/");
        assert_eq!(
            Renderer::resolve_path("././image.png", "chat"),
            "/chat/image.png"
        );
    }

    #[test]
    fn test_resolve_path_parent_dir() {
        assert_eq!(Renderer::resolve_path("../image.png", "chat"), "/image.png");
        assert_eq!(
            Renderer::resolve_path("../../image.png", "chat"),
            "/image.png"
        );
    }

    #[test]
    fn test_resolve_path_parent_dir_edge_cases() {
        assert_eq!(Renderer::resolve_path("../", "chat"), "/");
        assert_eq!(
            Renderer::resolve_path("../../../image.png", "chat"),
            "/image.png"
        );
    }

    #[test]
    fn test_resolve_path_bare_relative() {
        assert_eq!(
            Renderer::resolve_path("image.png", "chat"),
            "/chat/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("subfolder/image.png", "dev"),
            "/dev/subfolder/image.png"
        );
    }

    #[test]
    fn test_resolve_path_empty() {
        assert_eq!(Renderer::resolve_path("", "chat"), "/");
        assert_eq!(Renderer::resolve_path("  ", "chat"), "/");
    }

    #[test]
    fn test_resolve_path_special_protocols() {
        assert_eq!(Renderer::resolve_path("#anchor", "chat"), "#anchor");
        assert_eq!(
            Renderer::resolve_path("mailto:test@example.com", "chat"),
            "mailto:test@example.com"
        );
        assert_eq!(
            Renderer::resolve_path("data:image/png;base64,abc", "chat"),
            "data:image/png;base64,abc"
        );
    }

    #[test]
    fn test_resolve_path_category_extraction() {
        assert_eq!(
            Renderer::resolve_path("./i-use-arch-btw/image.png", "chat"),
            "/chat/i-use-arch-btw/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("./subdir/image.png", "tutorials"),
            "/tutorials/subdir/image.png"
        );
    }

    #[test]
    fn test_resolve_path_nested_category() {
        // For nested categories, category is passed (without slug)
        // e.g., for post at work/web-application/my-post, category is "work/web-application"
        assert_eq!(
            Renderer::resolve_path("./my-post/image.png", "work/web-application"),
            "/work/web-application/my-post/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("./image.png", "dev/tutorials"),
            "/dev/tutorials/image.png"
        );
        // Parent traversal from nested category
        assert_eq!(
            Renderer::resolve_path("../image.png", "work/apps"),
            "/work/image.png"
        );
        assert_eq!(
            Renderer::resolve_path("../../image.png", "work/apps"),
            "/image.png"
        );
    }

    #[test]
    fn test_resolve_raw_html_paths_video() {
        let html = r#"<video autoPlay playsInline muted loop src="./folder/video.mp4"></video>"#;
        let result = Renderer::resolve_raw_html_paths(html, "dev");
        assert_eq!(
            result,
            r#"<video autoPlay playsInline muted loop src="/dev/folder/video.mp4"></video>"#
        );
    }

    #[test]
    fn test_resolve_raw_html_paths_audio() {
        let html = r#"<audio src="./audio.mp3"></audio>"#;
        let result = Renderer::resolve_raw_html_paths(html, "music");
        assert_eq!(result, r#"<audio src="/music/audio.mp3"></audio>"#);
    }

    #[test]
    fn test_resolve_raw_html_paths_source() {
        let html = r#"<video><source src="./video.webm" type="video/webm"></video>"#;
        let result = Renderer::resolve_raw_html_paths(html, "dev");
        assert_eq!(
            result,
            r#"<video><source src="/dev/video.webm" type="video/webm"></video>"#
        );
    }

    #[test]
    fn test_resolve_raw_html_paths_absolute_url() {
        let html = r#"<video src="https://example.com/video.mp4"></video>"#;
        let result = Renderer::resolve_raw_html_paths(html, "dev");
        assert_eq!(
            result,
            r#"<video src="https://example.com/video.mp4"></video>"#
        );
    }

    #[test]
    fn test_resolve_raw_html_paths_skips_data_md() {
        let html = r#"<img data-md src="./image.png" />"#;
        let result = Renderer::resolve_raw_html_paths(html, "dev");
        assert_eq!(result, r#"<img data-md src="./image.png" />"#);
    }

    #[test]
    fn test_resolve_raw_html_paths_poster_attr() {
        let html = r#"<video src="./video.mp4" poster="./thumb.jpg"></video>"#;
        let result = Renderer::resolve_raw_html_paths(html, "dev");
        assert_eq!(
            result,
            r#"<video src="/dev/video.mp4" poster="/dev/thumb.jpg"></video>"#
        );
    }
}
