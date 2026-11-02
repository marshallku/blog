use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde_json::json;

use crate::{env::state::AppState, utils::comment_guard::issue_token};

/// Issued when the comment form loads; `create` refuses comments sent too soon after it.
pub async fn get(State(state): State<AppState>) -> impl IntoResponse {
    let token = issue_token(&state.jwt_secret, Utc::now().timestamp());

    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({ "token": token })),
    )
}
