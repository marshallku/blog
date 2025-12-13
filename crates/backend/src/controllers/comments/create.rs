use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
};
use bson::oid::ObjectId;
use chrono::Utc;
use serde::Deserialize;
use tera::Context;
use validator::Validate;

use crate::{
    auth::guard::AuthUserOrPublic,
    env::state::AppState,
    models::{comment::Comment, user::UserRole},
    templates::TEMPLATES,
    utils::{
        validator::ValidatedJson,
        webhook::{send_message, DiscordEmbed, DiscordField},
    },
};

#[derive(Deserialize, Validate)]
pub struct AddCommentPayload {
    #[serde(rename = "postSlug")]
    #[validate(length(min = 1, message = "Post slug cannot be empty"))]
    pub post_slug: String,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,

    #[validate(url(message = "Invalid URL format"))]
    pub url: Option<String>,

    #[validate(length(min = 1, message = "Comment body cannot be empty"))]
    pub body: String,

    #[serde(rename = "parentCommentId")]
    pub parent_comment_id: Option<String>,
}

pub async fn post(
    AuthUserOrPublic { user }: AuthUserOrPublic,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<AddCommentPayload>,
) -> impl IntoResponse {
    let is_root = user.is_some() && user.unwrap().role == UserRole::Root;
    let comment = Comment {
        id: None,
        post_slug: payload.post_slug,
        name: payload.name,
        email: payload.email.unwrap_or_default(),
        url: payload.url.unwrap_or_default(),
        body: payload.body,
        parent_comment_id: payload
            .parent_comment_id
            .and_then(|id| ObjectId::parse_str(&id).ok()),
        by_post_author: is_root,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        replies: None,
    };

    let created_comment = match Comment::create(&state.db, comment.clone()).await {
        Ok(comment) => {
            let comment_to_send = comment.clone();

            send_message(DiscordEmbed {
                embed_type: "rich".to_string(),
                title: "New comment added".to_string(),
                description: format!(
                    "New comment added by {} on {}",
                    comment_to_send.name, comment_to_send.post_slug
                ),
                color: None,
                fields: vec![
                    DiscordField {
                        name: "Name".to_string(),
                        value: comment_to_send.name,
                    },
                    DiscordField {
                        name: "URL".to_string(),
                        value: comment_to_send.url,
                    },
                    DiscordField {
                        name: "Content".to_string(),
                        value: comment_to_send.body,
                    },
                ],
                footer: None,
            });

            comment
        }
        Err(e) => {
            log::error!("Failed to create comment: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<p class=\"comment-form__error\">댓글 등록에 실패했습니다.</p>".to_string(),
            )
                .into_response();
        }
    };

    let mut context = Context::new();
    context.insert("comment", &created_comment.to_response());

    match TEMPLATES.render("comments/comment.html", &context) {
        Ok(html) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response(),
        Err(e) => {
            log::error!("Template render error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<p class=\"comment-form__error\">댓글 등록에 실패했습니다.</p>".to_string(),
            )
                .into_response()
        }
    }
}
