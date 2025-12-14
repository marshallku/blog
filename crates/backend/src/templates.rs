use std::{env, sync::LazyLock};
use tera::Tera;

pub static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
    let template_path = env::var("TEMPLATE_PATH")
        .unwrap_or_else(|_| "crates/backend/templates/**/*.html".to_string());
    let mut tera = match Tera::new(&template_path) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Template parsing error: {}", e);
            std::process::exit(1);
        }
    };
    tera.autoescape_on(vec![".html"]);
    tera
});
