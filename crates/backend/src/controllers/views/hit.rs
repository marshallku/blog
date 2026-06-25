use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use cookie::{Cookie, SameSite};
use serde::Deserialize;
use serde_json::json;
use time::Duration;
use uuid::Uuid;
use validator::Validate;

use crate::{
    constants::like::VISITOR_COOKIE_KEY,
    env::state::AppState,
    models::view::View,
    utils::{ip::ClientIp, slug::normalize_slug, validator::ValidatedJson},
};

#[derive(Deserialize, Validate)]
pub struct ViewHitPayload {
    #[serde(rename = "postSlug")]
    #[validate(length(min = 1))]
    pub post_slug: String,
}

pub async fn post(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<ViewHitPayload>,
) -> impl IntoResponse {
    let cookie_jar = CookieJar::from_headers(&headers);
    let post_slug = normalize_slug(&payload.post_slug).to_string();

    // `min(1)` validation runs before normalization, so a slug like "/" passes
    // it but normalizes to empty — reject those here rather than record a view
    // for a non-post.
    if post_slug.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid postSlug" })),
        )
            .into_response();
    }

    let ip_hash = crate::utils::ip::hash_ip(&ip);

    let (visitor_id, is_new_visitor) = match cookie_jar.get(VISITOR_COOKIE_KEY) {
        Some(cookie) => (cookie.value().to_string(), false),
        None => (Uuid::new_v4().to_string(), true),
    };

    let count = match View::record(&state.db, &post_slug, &ip_hash, &visitor_id).await {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to record view: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to record view" })),
            )
                .into_response();
        }
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
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

    (
        StatusCode::OK,
        response_headers,
        Json(json!({ "count": count })),
    )
        .into_response()
}
