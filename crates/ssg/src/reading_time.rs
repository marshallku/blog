use pulldown_cmark::{Event, Options, Parser, Tag};

const CJK_CHARS_PER_MIN: f64 = 500.0;
const WORDS_PER_MIN: f64 = 200.0;

/// Estimate reading time in minutes for mixed Korean/English markdown.
///
/// Korean (and other CJK) text is counted per-character (~500 chars/min);
/// Latin text is counted per-word (~200 words/min). Code blocks are excluded
/// since readers skim rather than read them word-by-word. Always at least 1.
pub fn estimate(markdown: &str) -> u32 {
    let parser = Parser::new_ext(markdown, Options::all());

    let mut in_code_block = false;
    let mut cjk_chars = 0usize;
    let mut words = 0usize;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(Tag::CodeBlock(_)) => in_code_block = false,
            Event::Text(text) if !in_code_block => {
                let (cjk, word_count) = count_text(&text);
                cjk_chars += cjk;
                words += word_count;
            }
            _ => {}
        }
    }

    let minutes = (cjk_chars as f64 / CJK_CHARS_PER_MIN) + (words as f64 / WORDS_PER_MIN);
    (minutes.ceil() as u32).max(1)
}

fn count_text(text: &str) -> (usize, usize) {
    let cjk = text.chars().filter(|c| is_cjk(*c)).count();

    let words = text
        .split_whitespace()
        .filter(|token| token.chars().any(|c| c.is_ascii_alphanumeric()))
        .count();

    (cjk, words)
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{AC00}'..='\u{D7A3}'   // Hangul syllables
        | '\u{1100}'..='\u{11FF}' // Hangul Jamo
        | '\u{3130}'..='\u{318F}' // Hangul compatibility Jamo
        | '\u{4E00}'..='\u{9FFF}' // CJK unified ideographs
        | '\u{3040}'..='\u{30FF}' // Hiragana + Katakana
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_one_minute() {
        assert_eq!(estimate(""), 1);
        assert_eq!(estimate("짧다"), 1);
    }

    #[test]
    fn test_korean_scales_by_chars() {
        let text = "가".repeat(1000);
        assert_eq!(estimate(&text), 2);
    }

    #[test]
    fn test_english_scales_by_words() {
        let text = "word ".repeat(400);
        assert_eq!(estimate(&text), 2);
    }

    #[test]
    fn test_code_blocks_excluded() {
        let with_code = format!("```\n{}\n```", "fn main() {}\n".repeat(200));
        assert_eq!(estimate(&with_code), 1);
    }
}
