use bson::{doc, Bson};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use mongodb::{bson::oid::ObjectId, error::Error, Database};
use serde::{Deserialize, Serialize};

use crate::models::like::is_duplicate_key_error;

pub const EVENT_COLLECTION: &str = "view_event";

/// One deduplicated view: at most one document per (post, visitor, day) thanks
/// to the unique index on `(postSlug, ipHash, visitorId, bucket)`. A TTL index
/// on `createdAt` keeps the collection bounded so windowed popularity stays cheap.
/// The per-post count is derived by counting these events (self-correcting if a
/// duplicate ever slips in before the index lands), not from a stored rollup.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ViewEvent {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "postSlug")]
    pub post_slug: String,

    #[serde(rename = "ipHash")]
    pub ip_hash: String,

    #[serde(rename = "visitorId")]
    pub visitor_id: String,

    /// UTC day key (`YYYY-MM-DD`) — the dedup window: one view per visitor/post/day.
    pub bucket: String,

    #[serde(
        rename = "createdAt",
        with = "bson::serde_helpers::chrono_datetime_as_bson_datetime"
    )]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PopularPost {
    #[serde(rename = "postSlug")]
    pub post_slug: String,
    pub views: u64,
}

pub struct View;

impl View {
    /// Record a view and return the post's deduplicated view count (within the
    /// TTL window). A repeated view by the same visitor on the same day is a
    /// no-op (duplicate-key) and does not inflate the count. The count is read
    /// back from the events, so it stays correct even if a duplicate slipped in
    /// before the unique index existed (the index self-heal removes it).
    pub async fn record(
        db: &Database,
        post_slug: &str,
        ip_hash: &str,
        visitor_id: &str,
    ) -> Result<u64, Error> {
        let now = Utc::now();
        let event = ViewEvent {
            id: None,
            post_slug: post_slug.to_string(),
            ip_hash: ip_hash.to_string(),
            visitor_id: visitor_id.to_string(),
            bucket: now.format("%Y-%m-%d").to_string(),
            created_at: now,
        };

        let events = db.collection::<ViewEvent>(EVENT_COLLECTION);
        match events.insert_one(event).await {
            Ok(_) => {}
            Err(e) if is_duplicate_key_error(&e) => {}
            Err(e) => return Err(e),
        }

        events.count_documents(doc! { "postSlug": post_slug }).await
    }

    /// Most-viewed posts over a trailing window, ranked by deduplicated view
    /// count (unique visitor-days). Ties broken by slug for deterministic output.
    pub async fn get_popular(
        db: &Database,
        days: i64,
        limit: i64,
    ) -> Result<Vec<PopularPost>, Error> {
        let cutoff = Bson::DateTime(bson::DateTime::from_chrono(
            Utc::now() - Duration::days(days),
        ));

        let pipeline = vec![
            doc! { "$match": { "createdAt": { "$gte": cutoff } } },
            doc! { "$group": { "_id": "$postSlug", "views": { "$sum": 1_i64 } } },
            doc! { "$sort": { "views": -1, "_id": 1 } },
            doc! { "$limit": limit },
        ];

        let mut cursor = db
            .collection::<bson::Document>(EVENT_COLLECTION)
            .aggregate(pipeline)
            .await?;

        let mut popular = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            let post_slug = doc.get_str("_id").unwrap_or_default().to_string();
            if post_slug.is_empty() {
                continue;
            }
            popular.push(PopularPost {
                post_slug,
                views: bson_count(&doc, "views"),
            });
        }

        Ok(popular)
    }
}

/// `$inc`/`$sum` may surface as either Int32 or Int64 depending on the driver
/// path; read both and clamp negatives to zero.
fn bson_count(doc: &bson::Document, key: &str) -> u64 {
    match doc.get(key) {
        Some(Bson::Int32(v)) => (*v).max(0) as u64,
        Some(Bson::Int64(v)) => (*v).max(0) as u64,
        _ => 0,
    }
}
