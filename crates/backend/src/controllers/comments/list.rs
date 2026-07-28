use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use tera::Context;

use crate::{
    env::state::AppState, models::comment::Comment, templates::TEMPLATES,
    utils::slug::normalize_slug,
};

#[derive(Deserialize)]
pub struct ListCommentsQuery {
    #[serde(rename = "postSlug")]
    pub slug: String,
    #[serde(default)]
    pub lang: Option<String>,
}

pub async fn get(
    State(state): State<AppState>,
    Query(query): Query<ListCommentsQuery>,
) -> impl IntoResponse {
    let slug = normalize_slug(&query.slug);
    let comments = match Comment::get_by_slug(&state.db, slug).await {
        Ok(comments) => comments,
        Err(e) => {
            log::error!("Failed to get comments: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<p class=\"comment-list__error\">댓글을 불러오지 못했습니다.</p>".to_string(),
            )
                .into_response();
        }
    };

    let mut context = Context::new();
    context.insert("comments", &comments);
    context.insert(
        "t",
        &crate::i18n::ui_strings(crate::i18n::normalize_lang(query.lang.as_deref())),
    );

    match TEMPLATES.render("comments/list.html", &context) {
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
                "<p class=\"comment-list__error\">댓글을 불러오지 못했습니다.</p>".to_string(),
            )
                .into_response()
        }
    }
}
