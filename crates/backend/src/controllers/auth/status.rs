use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::auth::guard::AuthUser;

pub async fn get(AuthUser { user }: AuthUser) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "name": user.name,
            "role": user.role,
            "createdAt": user.created_at.to_rfc3339(),
            "updatedAt": user.updated_at.to_rfc3339(),
        })),
    )
}
