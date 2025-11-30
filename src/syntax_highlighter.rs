use anyhow::Result;
use autumnus::formatter::Formatter;
use autumnus::languages::Language;
use autumnus::HtmlLinkedBuilder;

pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn highlight(&self, code: &str, lang: &str) -> Result<String> {
        let language = Language::guess(lang, code);

        let formatter = HtmlLinkedBuilder::new()
            .source(code)
            .lang(language)
            .pre_class(Some("hljs"))
            .build()?;

        let mut output = Vec::new();
        formatter.format(&mut output)?;

        Ok(String::from_utf8(output)?)
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to initialize SyntaxHighlighter")
    }
}
