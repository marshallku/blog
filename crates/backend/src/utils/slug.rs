pub fn normalize_slug(slug: &str) -> &str {
    slug.trim_end_matches('/')
}
