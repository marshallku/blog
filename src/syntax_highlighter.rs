use anyhow::Result;
use std::collections::HashMap;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

pub struct TreeSitterHighlighter {
    highlighter: Highlighter,
    configs: HashMap<&'static str, HighlightConfiguration>,
    highlight_names: Vec<String>,
}

const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "constant",
    "function.builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "comment",
];

impl TreeSitterHighlighter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            highlighter: Highlighter::new(),
            configs: HashMap::new(),
            highlight_names: HIGHLIGHT_NAMES.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn ensure_language_loaded(&mut self, language: &str) -> Option<&'static str> {
        let canonical: &'static str = match language {
            "ts" => "typescript",
            "jsx" => "javascript",
            "sh" => "bash",
            "rust" => "rust",
            "javascript" => "javascript",
            "typescript" => "typescript",
            "tsx" => "tsx",
            "python" => "python",
            "go" => "go",
            "c" => "c",
            "cpp" => "cpp",
            "java" => "java",
            "json" => "json",
            "bash" => "bash",
            "html" => "html",
            "css" => "css",
            _ => return None,
        };

        if !self.configs.contains_key(canonical) {
            let _ = self.load_language(canonical);
        }

        if self.configs.contains_key(canonical) {
            Some(canonical)
        } else {
            None
        }
    }

    fn load_language(&mut self, name: &str) -> Result<()> {
        let config = match name {
            "rust" => Self::create_config(
                name,
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            "javascript" => Self::create_config(
                name,
                tree_sitter_javascript::LANGUAGE.into(),
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                "",
                tree_sitter_javascript::LOCALS_QUERY,
            )?,
            "typescript" => Self::create_config(
                name,
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_typescript::LOCALS_QUERY,
            )?,
            "tsx" => Self::create_config(
                name,
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "",
                tree_sitter_typescript::LOCALS_QUERY,
            )?,
            "python" => Self::create_config(
                name,
                tree_sitter_python::LANGUAGE.into(),
                tree_sitter_python::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            "go" => Self::create_config(
                name,
                tree_sitter_go::LANGUAGE.into(),
                tree_sitter_go::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            "c" => Self::create_config(
                name,
                tree_sitter_c::LANGUAGE.into(),
                tree_sitter_c::HIGHLIGHT_QUERY,
                "",
                "",
            )?,
            "cpp" => Self::create_config(
                name,
                tree_sitter_cpp::LANGUAGE.into(),
                tree_sitter_cpp::HIGHLIGHT_QUERY,
                "",
                "",
            )?,
            "java" => Self::create_config(
                name,
                tree_sitter_java::LANGUAGE.into(),
                tree_sitter_java::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            "json" => Self::create_config(
                name,
                tree_sitter_json::LANGUAGE.into(),
                tree_sitter_json::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            "bash" => Self::create_config(
                name,
                tree_sitter_bash::LANGUAGE.into(),
                tree_sitter_bash::HIGHLIGHT_QUERY,
                "",
                "",
            )?,
            "html" => Self::create_config(
                name,
                tree_sitter_html::language().into(),
                tree_sitter_html::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            "css" => Self::create_config(
                name,
                tree_sitter_css::LANGUAGE.into(),
                tree_sitter_css::HIGHLIGHTS_QUERY,
                "",
                "",
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported language: {}", name)),
        };

        let static_name: &'static str = match name {
            "rust" => "rust",
            "javascript" => "javascript",
            "typescript" => "typescript",
            "tsx" => "tsx",
            "python" => "python",
            "go" => "go",
            "c" => "c",
            "cpp" => "cpp",
            "java" => "java",
            "json" => "json",
            "bash" => "bash",
            "html" => "html",
            "css" => "css",
            _ => return Err(anyhow::anyhow!("Unsupported language: {}", name)),
        };
        self.configs.insert(static_name, config);
        Ok(())
    }

    fn create_config(
        name: &str,
        language: tree_sitter::Language,
        highlights_query: &str,
        injections_query: &str,
        locals_query: &str,
    ) -> Result<HighlightConfiguration> {
        let mut config = HighlightConfiguration::new(
            language,
            name,
            highlights_query,
            injections_query,
            locals_query,
        )?;
        config.configure(HIGHLIGHT_NAMES);
        Ok(config)
    }

    pub fn highlight(&mut self, code: &str, language: &str) -> Result<String> {
        let lang_key = match self.ensure_language_loaded(language) {
            Some(key) => key,
            None => {
                return Ok(format!(
                    "<pre class=\"syntax-highlight\"><code>{}</code></pre>",
                    Self::escape_html(code)
                ));
            }
        };

        let config = match self.configs.get(lang_key) {
            Some(c) => c,
            None => {
                return Ok(format!(
                    "<pre class=\"syntax-highlight\"><code>{}</code></pre>",
                    Self::escape_html(code)
                ));
            }
        };

        let highlight_names = self.highlight_names.clone();

        let highlights = self
            .highlighter
            .highlight(config, code.as_bytes(), None, |_| None)?;

        let mut html = String::with_capacity(code.len() * 2);
        let mut current_highlight: Option<String> = None;

        for event in highlights {
            match event? {
                HighlightEvent::Source { start, end } => {
                    let text = &code[start..end];
                    html.push_str(&Self::escape_html(text));
                }
                HighlightEvent::HighlightStart(s) => {
                    if let Some(class) = highlight_names.get(s.0) {
                        html.push_str(&format!("<span class=\"{}\">", class));
                        current_highlight = Some(class.clone());
                    }
                }
                HighlightEvent::HighlightEnd => {
                    if current_highlight.is_some() {
                        html.push_str("</span>");
                        current_highlight = None;
                    }
                }
            }
        }

        Ok(format!(
            "<pre class=\"syntax-highlight\"><code>{}</code></pre>",
            html
        ))
    }

    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

impl Default for TreeSitterHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to initialize TreeSitterHighlighter")
    }
}
