use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use tera::Context;

use crate::{
    constants::like::VISITOR_COOKIE_KEY,
    env::state::AppState,
    models::like::Like,
    templates::TEMPLATES,
    utils::{ip::ClientIp, slug::normalize_slug},
};

#[derive(Deserialize)]
pub struct StatusQuery {
    #[serde(rename = "postSlug")]
    pub post_slug: String,
}

pub async fn get(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Query(query): Query<StatusQuery>,
) -> impl IntoResponse {
    let cookie_jar = CookieJar::from_headers(&headers);
    let post_slug = normalize_slug(&query.post_slug).to_string();
    let ip_hash = crate::utils::ip::hash_ip(&ip);
    let visitor_id = cookie_jar
        .get(VISITOR_COOKIE_KEY)
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    let (liked, count) = match Like::get_status(&state.db, &post_slug, &ip_hash, &visitor_id).await
    {
        Ok(result) => result,
        Err(e) => {
            log::error!("Failed to get like status: {}", e);
            (false, 0)
        }
    };

    let mut context = Context::new();
    context.insert("liked", &liked);
    context.insert("count", &count);

    match TEMPLATES.render("likes/button.html", &context) {
        Ok(html) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response(),
        Err(e) => {
            log::error!("Template render error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<button class=\"post-like__button\" disabled>오류</button>".to_string(),
            )
                .into_response()
        }
    }
}
