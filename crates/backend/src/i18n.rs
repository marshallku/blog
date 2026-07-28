use std::collections::HashMap;

/// UI strings for the server-rendered comment/like fragments, selected by the
/// requesting page's language (passed by the frontend as a `lang` param). Kept
/// self-contained here since the backend renders its own HTML fragments that are
/// injected into pages served in either language.
pub fn ui_strings(lang: &str) -> HashMap<&'static str, &'static str> {
    let en = lang == "en";
    HashMap::from([
        (
            "avatar_alt_suffix",
            if en { "'s avatar" } else { " 님의 아바타" },
        ),
        (
            "date_format",
            if en { "%b %d, %Y" } else { "%Y년 %m월 %d일" },
        ),
        ("reply", if en { "Reply" } else { "답글" }),
        (
            "no_comments",
            if en {
                "No comments yet."
            } else {
                "아직 댓글이 없습니다."
            },
        ),
        ("like", if en { "Like" } else { "좋아요" }),
    ])
}

/// Normalizes an incoming `lang` value to a supported code, defaulting to the
/// site default (`ko`) for anything unknown or missing.
pub fn normalize_lang(lang: Option<&str>) -> &'static str {
    match lang {
        Some("en") => "en",
        _ => "ko",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_lang() {
        assert_eq!(normalize_lang(Some("en")), "en");
        assert_eq!(normalize_lang(Some("ko")), "ko");
        assert_eq!(normalize_lang(Some("fr")), "ko");
        assert_eq!(normalize_lang(None), "ko");
    }

    #[test]
    fn test_ui_strings_selects_language() {
        assert_eq!(ui_strings("en")["reply"], "Reply");
        assert_eq!(ui_strings("ko")["reply"], "답글");
        assert_eq!(ui_strings("en")["date_format"], "%b %d, %Y");
    }

    /// Renders the actual server templates with the i18n context to confirm the
    /// fragments the backend injects into pages are localized end-to-end.
    #[test]
    fn test_templates_render_localized() {
        // Resolve templates relative to the crate, not the process CWD, so the
        // test is deterministic regardless of where `cargo test` is invoked.
        let glob = format!("{}/templates/**/*.html", env!("CARGO_MANIFEST_DIR"));
        let tera = tera::Tera::new(&glob).expect("load backend templates");

        // Like button
        let mut ctx = tera::Context::new();
        ctx.insert("liked", &false);
        ctx.insert("count", &0);
        ctx.insert("t", &ui_strings("en"));
        let en = tera.render("likes/button.html", &ctx).unwrap();
        assert!(en.contains(r#"aria-label="Like""#), "en like: {en}");
        ctx.insert("t", &ui_strings("ko"));
        let ko = tera.render("likes/button.html", &ctx).unwrap();
        assert!(ko.contains(r#"aria-label="좋아요""#), "ko like: {ko}");

        // Empty comment list
        let mut ctx = tera::Context::new();
        ctx.insert("comments", &Vec::<serde_json::Value>::new());
        ctx.insert("t", &ui_strings("en"));
        let en = tera.render("comments/list.html", &ctx).unwrap();
        assert!(en.contains("No comments yet."), "en list: {en}");
        ctx.insert("t", &ui_strings("ko"));
        let ko = tera.render("comments/list.html", &ctx).unwrap();
        assert!(ko.contains("아직 댓글이 없습니다."), "ko list: {ko}");
    }
}
