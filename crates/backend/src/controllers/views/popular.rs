use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::{
    env::state::AppState,
    models::view::{PopularPost, View},
};

const CACHE_TTL: Duration = Duration::from_secs(300);
const MAX_LIMIT: i64 = 20;

#[derive(Deserialize)]
pub struct PopularQuery {
    #[serde(default = "default_days")]
    pub days: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_days() -> i64 {
    30
}

fn default_limit() -> i64 {
    5
}

/// The home page hits this on every visit, so a short-lived in-process cache
/// keeps the trailing-window aggregation off Mongo's hot path.
struct CacheEntry {
    fetched_at: Instant,
    key: (i64, i64),
    value: Vec<PopularPost>,
}

static CACHE: OnceLock<Mutex<Option<CacheEntry>>> = OnceLock::new();

pub async fn get(
    State(state): State<AppState>,
    Query(query): Query<PopularQuery>,
) -> impl IntoResponse {
    let days = query.days.clamp(1, 365);
    let limit = query.limit.clamp(1, MAX_LIMIT);
    let key = (days, limit);

    if let Some(cached) = read_cache(key) {
        return (StatusCode::OK, Json(json!({ "posts": cached }))).into_response();
    }

    let popular = match View::get_popular(&state.db, days, limit).await {
        Ok(posts) => posts,
        Err(e) => {
            log::error!("Failed to get popular posts: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to get popular posts" })),
            )
                .into_response();
        }
    };

    write_cache(key, &popular);

    (StatusCode::OK, Json(json!({ "posts": popular }))).into_response()
}

fn read_cache(key: (i64, i64)) -> Option<Vec<PopularPost>> {
    let guard = CACHE.get_or_init(|| Mutex::new(None)).lock().ok()?;
    let entry = guard.as_ref()?;

    (entry.key == key && entry.fetched_at.elapsed() < CACHE_TTL).then(|| entry.value.clone())
}

fn write_cache(key: (i64, i64), value: &[PopularPost]) {
    if let Ok(mut guard) = CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *guard = Some(CacheEntry {
            fetched_at: Instant::now(),
            key,
            value: value.to_vec(),
        });
    }
}
