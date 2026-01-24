use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use cookie::{Cookie, SameSite};
use serde::Deserialize;
use tera::Context;
use time::Duration;
use uuid::Uuid;
use validator::Validate;

use crate::{
    constants::like::VISITOR_COOKIE_KEY,
    env::state::AppState,
    models::like::Like,
    templates::TEMPLATES,
    utils::{ip::ClientIp, slug::normalize_slug, validator::ValidatedJson},
};

#[derive(Deserialize, Validate)]
pub struct ToggleLikePayload {
    #[serde(rename = "postSlug")]
    #[validate(length(min = 1))]
    pub post_slug: String,
}

pub async fn post(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<ToggleLikePayload>,
) -> impl IntoResponse {
    let cookie_jar = CookieJar::from_headers(&headers);
    let post_slug = normalize_slug(&payload.post_slug).to_string();
    let ip_hash = crate::utils::ip::hash_ip(&ip);

    let (visitor_id, is_new_visitor) = match cookie_jar.get(VISITOR_COOKIE_KEY) {
        Some(cookie) => (cookie.value().to_string(), false),
        None => (Uuid::new_v4().to_string(), true),
    };

    let (liked, count) = match Like::toggle(&state.db, &post_slug, &ip_hash, &visitor_id).await {
        Ok(result) => result,
        Err(e) => {
            log::error!("Failed to toggle like: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<button class=\"post-like__button\" disabled>오류</button>".to_string(),
            )
                .into_response();
        }
    };

    let mut context = Context::new();
    context.insert("liked", &liked);
    context.insert("count", &count);

    let html = match TEMPLATES.render("likes/button.html", &context) {
        Ok(html) => html,
        Err(e) => {
            log::error!("Template render error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<button class=\"post-like__button\" disabled>오류</button>".to_string(),
            )
                .into_response();
        }
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8".parse().unwrap(),
    );

    if is_new_visitor {
        let cookie = Cookie::build((VISITOR_COOKIE_KEY, visitor_id))
            .path("/")
            .secure(true)
            .http_only(true)
            .max_age(Duration::days(365 * 2))
            .same_site(SameSite::None)
            .domain(state.cookie_domain.clone());

        if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
            response_headers.insert(header::SET_COOKIE, value);
        }
    }

    (StatusCode::OK, response_headers, html).into_response()
}
