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
        let mut configs = HashMap::new();

        Self::add_language_config(
            &mut configs,
            "rust",
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "javascript",
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            "",
            tree_sitter_javascript::LOCALS_QUERY,
        )?;

        Self::add_language_config(
            &mut configs,
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_typescript::LOCALS_QUERY,
        )?;

        Self::add_language_config(
            &mut configs,
            "tsx",
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_typescript::LOCALS_QUERY,
        )?;

        Self::add_language_config(
            &mut configs,
            "python",
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "go",
            tree_sitter_go::LANGUAGE.into(),
            tree_sitter_go::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "c",
            tree_sitter_c::LANGUAGE.into(),
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "cpp",
            tree_sitter_cpp::LANGUAGE.into(),
            tree_sitter_cpp::HIGHLIGHT_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "java",
            tree_sitter_java::LANGUAGE.into(),
            tree_sitter_java::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "json",
            tree_sitter_json::LANGUAGE.into(),
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "bash",
            tree_sitter_bash::LANGUAGE.into(),
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "html",
            tree_sitter_html::language().into(),
            tree_sitter_html::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        Self::add_language_config(
            &mut configs,
            "css",
            tree_sitter_css::LANGUAGE.into(),
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
            "",
        )?;

        // Add aliases by creating new configs
        Self::add_language_config(
            &mut configs,
            "ts",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_typescript::LOCALS_QUERY,
        )?;

        Self::add_language_config(
            &mut configs,
            "jsx",
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            "",
            tree_sitter_javascript::LOCALS_QUERY,
        )?;

        Self::add_language_config(
            &mut configs,
            "sh",
            tree_sitter_bash::LANGUAGE.into(),
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        )?;

        Ok(Self {
            highlighter: Highlighter::new(),
            configs,
            highlight_names: HIGHLIGHT_NAMES.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn add_language_config(
        configs: &mut HashMap<&'static str, HighlightConfiguration>,
        name: &'static str,
        language: tree_sitter::Language,
        highlights_query: &str,
        injections_query: &str,
        locals_query: &str,
    ) -> Result<()> {
        let mut config = HighlightConfiguration::new(
            language,
            name,
            highlights_query,
            injections_query,
            locals_query,
        )?;
        config.configure(HIGHLIGHT_NAMES);
        configs.insert(name, config);
        Ok(())
    }

    pub fn highlight(&mut self, code: &str, language: &str) -> Result<String> {
        let config = self
            .configs
            .get(language)
            .or_else(|| self.configs.get("javascript")) // fallback
            .ok_or_else(|| anyhow::anyhow!("Unsupported language: {}", language))?;

        let highlights = self
            .highlighter
            .highlight(config, code.as_bytes(), None, |_| None)?;

        let mut html = String::with_capacity(code.len() * 2);
        let mut current_highlight: Option<&str> = None;

        for event in highlights {
            match event? {
                HighlightEvent::Source { start, end } => {
                    let text = &code[start..end];
                    html.push_str(&Self::escape_html(text));
                }
                HighlightEvent::HighlightStart(s) => {
                    let class = self.highlight_names.get(s.0).map(|s| s.as_str());
                    if let Some(class) = class {
                        html.push_str(&format!("<span class=\"{}\">", class));
                        current_highlight = Some(class);
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
