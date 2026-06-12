use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Shortcode handler function type
pub type ShortcodeHandler =
    Box<dyn Fn(&HashMap<String, String>, Option<&str>) -> Result<String> + Send + Sync>;

/// Registry for shortcode handlers
pub struct ShortcodeRegistry {
    handlers: HashMap<String, ShortcodeHandler>,
}

impl ShortcodeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };

        // Register built-in shortcodes
        registry.register_builtin();

        registry
    }

    fn register_builtin(&mut self) {
        // Figure shortcode: [figure src="..." alt="..." caption="..."]
        self.register(
            "figure",
            Box::new(|attrs, _content| {
                let src = attrs.get("src").map(|s| s.as_str()).unwrap_or("");
                let alt = attrs.get("alt").map(|s| s.as_str()).unwrap_or("");
                let caption = attrs.get("caption").map(|s| s.as_str()).unwrap_or("");

                let mut html = format!(
                    r#"<figure><img src="{}" alt="{}" loading="lazy""#,
                    escape_html(src),
                    escape_html(alt)
                );

                if let Some(width) = attrs.get("width") {
                    html.push_str(&format!(r#" width="{}""#, escape_html(width)));
                }
                if let Some(height) = attrs.get("height") {
                    html.push_str(&format!(r#" height="{}""#, escape_html(height)));
                }

                html.push_str(" />");

                if !caption.is_empty() {
                    html.push_str(&format!(
                        "<figcaption>{}</figcaption>",
                        escape_html(caption)
                    ));
                }

                html.push_str("</figure>");

                Ok(html)
            }),
        );

        // Callout shortcode: [callout type="info"]content[/callout]
        self.register(
            "callout",
            Box::new(|attrs, content| {
                let callout_type = attrs.get("type").map(|s| s.as_str()).unwrap_or("info");
                let title = attrs.get("title").map(|s| s.as_str());
                let content = content.unwrap_or("");

                let mut html = format!(
                    r#"<div class="callout callout-{}">"#,
                    escape_html(callout_type)
                );

                if let Some(t) = title {
                    html.push_str(&format!(
                        r#"<div class="callout-title">{}</div>"#,
                        escape_html(t)
                    ));
                }

                html.push_str(&format!(
                    r#"<div class="callout-content">{}</div></div>"#,
                    content
                ));

                Ok(html)
            }),
        );

        // YouTube shortcode: [youtube id="..."]
        self.register("youtube", Box::new(|attrs, _content| {
            let id = attrs.get("id").map(|s| s.as_str()).unwrap_or("");
            let title = attrs.get("title").map(|s| s.as_str()).unwrap_or("YouTube video");

            if id.is_empty() {
                return Err(anyhow!("YouTube shortcode requires 'id' attribute"));
            }

            Ok(format!(
                r#"<div class="video-container"><iframe src="https://www.youtube.com/embed/{}" title="{}" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen loading="lazy"></iframe></div>"#,
                escape_html(id),
                escape_html(title)
            ))
        }));

        // Code block with filename: [code lang="..." filename="..."]
        self.register(
            "code",
            Box::new(|attrs, content| {
                let lang = attrs.get("lang").map(|s| s.as_str()).unwrap_or("");
                let filename = attrs.get("filename").map(|s| s.as_str());
                let content = content.unwrap_or("");

                let mut html = String::new();

                if let Some(f) = filename {
                    html.push_str(&format!(
                        r#"<div class="code-block"><div class="code-filename">{}</div>"#,
                        escape_html(f)
                    ));
                }

                html.push_str(&format!(
                    r#"<pre><code class="language-{}">{}</code></pre>"#,
                    escape_html(lang),
                    escape_html(content)
                ));

                if filename.is_some() {
                    html.push_str("</div>");
                }

                Ok(html)
            }),
        );

        // React island shortcode: [react component="..." data="..." title="..."]
        // Generates a placeholder div that will be hydrated by React on the client
        // All attributes except 'component' and 'loading' are passed as props
        self.register(
            "react",
            Box::new(|attrs, content| {
                let component = attrs
                    .get("component")
                    .ok_or_else(|| anyhow!("React shortcode requires 'component' attribute"))?;
                let loading = attrs.get("loading").map(|s| s.as_str()).unwrap_or("lazy");

                // Build props JSON from all other attributes
                let mut entries: Vec<_> = attrs.iter().collect();
                entries.sort_by_key(|(key, _)| key.as_str());

                let mut props_parts: Vec<String> = Vec::new();
                for (key, value) in entries {
                    if key != "component" && key != "loading" {
                        // Escape the value for JSON string
                        let escaped_value = value
                            .replace('\\', "\\\\")
                            .replace('"', "\\\"");
                        props_parts.push(format!(r#""{}":"{}""#, key, escaped_value));
                    }
                }
                let props_json = format!("{{{}}}", props_parts.join(","));

                let mut html = format!(
                    r#"<div class="react-island" data-component="{}" data-props='{}' data-loading="{}">"#,
                    escape_html(component),
                    escape_html(&props_json),
                    escape_html(loading)
                );

                if let Some(fallback) = content {
                    html.push_str(&format!(
                        r#"<div class="react-island__fallback">{}</div>"#,
                        fallback
                    ));
                }

                html.push_str(
                    r#"<noscript>Interactive component requires JavaScript</noscript></div>"#,
                );

                Ok(html)
            }),
        );
    }

    /// Register a custom shortcode handler
    pub fn register(&mut self, name: &str, handler: ShortcodeHandler) {
        self.handlers.insert(name.to_string(), handler);
    }

    /// Process all shortcodes in content. Fenced code blocks and inline code
    /// spans are masked first so shortcode syntax can be shown as code
    /// without being executed.
    pub fn process(&self, content: &str) -> Result<String> {
        let (masked, code_regions) = mask_code_regions(content);

        let mut result = self.process_block_shortcodes(&masked)?;
        result = self.process_inline_shortcodes(&result)?;

        Ok(restore_code_regions(result, &code_regions))
    }

    fn process_block_shortcodes(&self, content: &str) -> Result<String> {
        static OPEN_RE: OnceLock<Regex> = OnceLock::new();
        let open_re = OPEN_RE
            .get_or_init(|| Regex::new(r#"\[(\w+)((?:[^\]"']|"[^"]*"|'[^']*')*)\]"#).unwrap());

        let mut result = content.to_string();
        let mut processed = true;

        // Keep processing until no more block shortcodes found
        while processed {
            processed = false;
            let content_clone = result.clone();

            for cap in open_re.captures_iter(&content_clone) {
                let open_match = cap.get(0).unwrap();
                let name = cap.get(1).unwrap().as_str();
                let attrs_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                if !self.handlers.contains_key(name) {
                    continue;
                }

                let close_tag = format!("[/{}]", name);
                let search_start = open_match.end();

                if let Some(close_pos) = content_clone[search_start..].find(&close_tag) {
                    let inner_content = &content_clone[search_start..search_start + close_pos];
                    let full_end = search_start + close_pos + close_tag.len();

                    if let Some(handler) = self.handlers.get(name) {
                        let attrs = parse_attributes(attrs_str);
                        let replacement = handler(&attrs, Some(inner_content.trim()))?;

                        result = format!(
                            "{}{}{}",
                            &content_clone[..open_match.start()],
                            replacement,
                            &content_clone[full_end..]
                        );
                        processed = true;
                        break;
                    }
                }
            }
        }

        Ok(result)
    }

    fn process_inline_shortcodes(&self, content: &str) -> Result<String> {
        static INLINE_RE: OnceLock<Regex> = OnceLock::new();
        let re = INLINE_RE
            .get_or_init(|| Regex::new(r#"\[(\w+)((?:[^\]"']|"[^"]*"|'[^']*')*)\]"#).unwrap());

        let mut result = content.to_string();
        let mut offset = 0i64;

        for cap in re.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let name = cap.get(1).unwrap().as_str();
            let attrs_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if content[full_match.end()..].starts_with('(') {
                continue;
            }

            if name.starts_with('/') {
                continue;
            }

            if let Some(handler) = self.handlers.get(name) {
                let attrs = parse_attributes(attrs_str);
                let replacement = handler(&attrs, None)?;

                let start = (full_match.start() as i64 + offset) as usize;
                let end = (full_match.end() as i64 + offset) as usize;

                result.replace_range(start..end, &replacement);
                offset += replacement.len() as i64 - full_match.len() as i64;
            }
        }

        Ok(result)
    }
}

impl Default for ShortcodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse shortcode attributes: key="value" key2='value2'
fn parse_attributes(s: &str) -> HashMap<String, String> {
    static ATTR_RE: OnceLock<Regex> = OnceLock::new();
    let re = ATTR_RE.get_or_init(|| Regex::new(r#"(\w+)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap());

    let mut attrs = HashMap::new();

    for cap in re.captures_iter(s) {
        let key = cap.get(1).unwrap().as_str();
        let value = cap
            .get(2)
            .or_else(|| cap.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");
        attrs.insert(key.to_string(), value.to_string());
    }

    attrs
}

/// Escape HTML special characters
// Private-use-area sentinel; effectively impossible in real markdown content
const CODE_PLACEHOLDER_MARK: char = '\u{E000}';

fn code_placeholder(index: usize) -> String {
    format!(
        "{}{}{}",
        CODE_PLACEHOLDER_MARK, index, CODE_PLACEHOLDER_MARK
    )
}

/// Replaces fenced code blocks (``` / ~~~) and inline code spans with
/// placeholders so shortcode processing never touches code content.
/// Indented (4-space) code blocks are not masked — this blog uses fences.
fn mask_code_regions(content: &str) -> (String, Vec<String>) {
    let mut regions: Vec<String> = Vec::new();
    let mut out = String::with_capacity(content.len());
    let mut open_fence: Option<(char, usize)> = None;
    let mut fence_buf = String::new();
    // Inline spans may cross single newlines, so they are masked per
    // paragraph (blank-line boundaries), not per physical line
    let mut paragraph_buf = String::new();

    for line in content.split_inclusive('\n') {
        if let Some((marker, open_len)) = open_fence {
            fence_buf.push_str(line);
            if closes_fence(line, marker, open_len) {
                out.push_str(&code_placeholder(regions.len()));
                regions.push(std::mem::take(&mut fence_buf));
                open_fence = None;
            }
            continue;
        }

        if let Some(fence) = parse_fence_open(line) {
            out.push_str(&mask_inline_code(
                &std::mem::take(&mut paragraph_buf),
                &mut regions,
            ));
            open_fence = Some(fence);
            fence_buf.push_str(line);
            continue;
        }

        if line.trim().is_empty() {
            out.push_str(&mask_inline_code(
                &std::mem::take(&mut paragraph_buf),
                &mut regions,
            ));
            out.push_str(line);
        } else {
            paragraph_buf.push_str(line);
        }
    }

    out.push_str(&mask_inline_code(&paragraph_buf, &mut regions));

    // Unclosed fence: markdown treats the rest of the document as code
    if !fence_buf.is_empty() {
        out.push_str(&code_placeholder(regions.len()));
        regions.push(fence_buf);
    }

    (out, regions)
}

/// CommonMark fence opening: up to 3 spaces of indentation, then a run of
/// at least 3 backticks or tildes. Returns the marker char and run length.
fn parse_fence_open(line: &str) -> Option<(char, usize)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }

    let rest = &line[indent..];
    let marker = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run_len = rest.chars().take_while(|c| *c == marker).count();

    (run_len >= 3).then_some((marker, run_len))
}

/// CommonMark fence closing: same marker char, run at least as long as the
/// opening run, nothing but whitespace afterwards.
fn closes_fence(line: &str, marker: char, open_len: usize) -> bool {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return false;
    }

    let rest = &line[indent..];
    let run_len = rest.chars().take_while(|c| *c == marker).count();

    run_len >= open_len && rest[run_len..].trim().is_empty()
}

/// Masks inline code spans per CommonMark: an opening run of N backticks is
/// closed by the next run of exactly N backticks.
fn mask_inline_code(line: &str, regions: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;

    while let Some(start) = rest.find('`') {
        let run_len = rest[start..].chars().take_while(|c| *c == '`').count();
        let after_open = start + run_len;

        let mut close_end = None;
        let mut search = after_open;
        while let Some(offset) = rest[search..].find('`') {
            let close_start = search + offset;
            let close_len = rest[close_start..]
                .chars()
                .take_while(|c| *c == '`')
                .count();
            if close_len == run_len {
                close_end = Some(close_start + close_len);
                break;
            }
            search = close_start + close_len;
        }

        match close_end {
            Some(end) => {
                out.push_str(&rest[..start]);
                out.push_str(&code_placeholder(regions.len()));
                regions.push(rest[start..end].to_string());
                rest = &rest[end..];
            }
            None => {
                out.push_str(&rest[..after_open]);
                rest = &rest[after_open..];
            }
        }
    }

    out.push_str(rest);
    out
}

fn restore_code_regions(mut content: String, regions: &[String]) -> String {
    for (index, region) in regions.iter().enumerate() {
        content = content.replace(&code_placeholder(index), region);
    }
    content
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attributes() {
        let attrs = parse_attributes(r#"src="image.jpg" alt="Test" width='100'"#);
        assert_eq!(attrs.get("src"), Some(&"image.jpg".to_string()));
        assert_eq!(attrs.get("alt"), Some(&"Test".to_string()));
        assert_eq!(attrs.get("width"), Some(&"100".to_string()));
    }

    #[test]
    fn test_figure_shortcode() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[figure src="test.jpg" alt="Test image" caption="My caption"]"#)
            .unwrap();
        assert!(result.contains("<figure>"));
        assert!(result.contains(r#"src="test.jpg""#));
        assert!(result.contains("<figcaption>My caption</figcaption>"));
    }

    #[test]
    fn test_callout_shortcode() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[callout type="warning" title="Note"]This is important[/callout]"#)
            .unwrap();
        assert!(result.contains("callout-warning"));
        assert!(result.contains("This is important"));
    }

    #[test]
    fn test_youtube_shortcode() {
        let registry = ShortcodeRegistry::new();
        let result = registry.process(r#"[youtube id="dQw4w9WgXcQ"]"#).unwrap();
        assert!(result.contains("youtube.com/embed/dQw4w9WgXcQ"));
    }

    #[test]
    fn test_markdown_links_not_processed() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[link text](https://example.com)"#)
            .unwrap();
        // Should remain unchanged since it's markdown syntax
        assert_eq!(result, r#"[link text](https://example.com)"#);
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_react_shortcode() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[react component="Chart" data="1,2,3" title="Test"]"#)
            .unwrap();
        assert!(result.contains("react-island"));
        assert!(result.contains(r#"data-component="Chart""#));
        assert!(result.contains(r#"data-props='"#));
        // Props are HTML-escaped in the output
        assert!(result.contains(r#"&quot;data&quot;:&quot;1,2,3&quot;"#));
        assert!(result.contains(r#"&quot;title&quot;:&quot;Test&quot;"#));
        assert!(result.contains(r#"data-loading="lazy""#));
    }

    #[test]
    fn test_react_shortcode_with_fallback() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[react component="CodeEditor" lang="ts"]const x = 1;[/react]"#)
            .unwrap();
        assert!(result.contains("react-island__fallback"));
        assert!(result.contains("const x = 1;"));
        // Props are HTML-escaped in the output
        assert!(result.contains(r#"&quot;lang&quot;:&quot;ts&quot;"#));
    }

    #[test]
    fn test_react_shortcode_eager_loading() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[react component="Chart" loading="eager"]"#)
            .unwrap();
        assert!(result.contains(r#"data-loading="eager""#));
    }

    #[test]
    fn test_react_shortcode_missing_component() {
        let registry = ShortcodeRegistry::new();
        let result = registry.process(r#"[react data="1,2,3"]"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcode_inside_fenced_block_untouched() {
        let registry = ShortcodeRegistry::new();
        let input = "Before\n\n```\n[figure src=\"a.jpg\" alt=\"x\"]\n```\n\nAfter";
        let result = registry.process(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_shortcode_inside_inline_code_untouched() {
        let registry = ShortcodeRegistry::new();
        let input = "Use `[youtube id=\"abc\"]` to embed.";
        let result = registry.process(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_shortcode_outside_code_still_processed() {
        let registry = ShortcodeRegistry::new();
        let input = "```\n[figure src=\"doc.jpg\" alt=\"doc\"]\n```\n\n[youtube id=\"abc\"]";
        let result = registry.process(input).unwrap();
        assert!(
            result.contains("[figure src=\"doc.jpg\" alt=\"doc\"]"),
            "code block stays raw"
        );
        assert!(
            result.contains("youtube.com/embed/abc"),
            "real shortcode executes"
        );
    }

    #[test]
    fn test_tilde_fence_and_unclosed_fence() {
        let registry = ShortcodeRegistry::new();
        let tilde = "~~~\n[youtube id=\"abc\"]\n~~~\n";
        assert_eq!(registry.process(tilde).unwrap(), tilde);

        let unclosed = "```\n[youtube id=\"abc\"]";
        assert_eq!(registry.process(unclosed).unwrap(), unclosed);
    }

    #[test]
    fn test_double_backtick_span_untouched() {
        let registry = ShortcodeRegistry::new();
        let input = "Span ``[callout]`x`[/callout]`` stays.";
        assert_eq!(registry.process(input).unwrap(), input);
    }

    #[test]
    fn test_longer_fence_documents_shorter_fence() {
        let registry = ShortcodeRegistry::new();
        let input = "`````markdown\n```\n[youtube id=\"abc\"]\n```\n`````\n";
        assert_eq!(registry.process(input).unwrap(), input);
    }

    #[test]
    fn test_multiline_inline_code_span_untouched() {
        let registry = ShortcodeRegistry::new();
        let input = "Use `[youtube\nid=\"abc\"]` to embed.";
        assert_eq!(registry.process(input).unwrap(), input);
    }

    #[test]
    fn test_indented_backticks_do_not_open_fence() {
        let registry = ShortcodeRegistry::new();
        let input = "    ```\n\n[youtube id=\"abc\"]";
        let result = registry.process(input).unwrap();
        assert!(
            result.contains("youtube.com/embed/abc"),
            "shortcode after indented backticks must run"
        );
    }

    #[test]
    fn test_backticks_across_paragraphs_do_not_mask_shortcode() {
        let registry = ShortcodeRegistry::new();
        let input = "stray ` tick\n\n[youtube id=\"abc\"]\n\nanother ` tick";
        let result = registry.process(input).unwrap();
        assert!(
            result.contains("youtube.com/embed/abc"),
            "paragraph boundary must stop span search"
        );
    }

    #[test]
    fn test_bracket_inside_quoted_attribute() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[figure src="image[1].jpg" alt="x"]"#)
            .unwrap();
        assert!(result.contains(r#"src="image[1].jpg""#), "got: {}", result);
    }

    #[test]
    fn test_bracket_inside_single_quoted_attribute() {
        let registry = ShortcodeRegistry::new();
        let result = registry
            .process(r#"[figure src='image[1].jpg' alt='x']"#)
            .unwrap();
        assert!(result.contains(r#"src="image[1].jpg""#), "got: {}", result);
    }
}
