use std::sync::LazyLock;
use tera::Tera;

pub static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = match Tera::new("crates/backend/templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            log::error!("Template parsing error: {}", e);
            std::process::exit(1);
        }
    };
    tera.autoescape_on(vec![".html"]);
    tera
});
