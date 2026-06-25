use anyhow::{Context, Result};
use bson::{doc, Bson};
use futures::TryStreamExt;
use mongodb::{options::IndexOptions, Client, Collection, Database, IndexModel};
use std::env;
use std::time::Duration;

use crate::models::like::{is_duplicate_key_error, Like};
use crate::models::view::{ViewEvent, EVENT_COLLECTION};
use crate::utils::encode::url_encode;

/// View events are kept for slightly longer than the popular-window so a
/// trailing-30-day ranking always has its full window of data.
const VIEW_EVENT_TTL: Duration = Duration::from_secs(35 * 24 * 60 * 60);

const INDEX_RETRY_BACKOFF: Duration = Duration::from_secs(2);
const INDEX_RETRY_BACKOFF_MAX: Duration = Duration::from_secs(60);

pub async fn init_db() -> Result<Database> {
    let host = env::var("MONGO_HOST").context("MONGO_HOST must be set")?;
    let port = env::var("MONGO_PORT").context("MONGO_PORT must be set")?;
    let username = env::var("MONGO_USERNAME").context("MONGO_USERNAME must be set")?;
    let password = env::var("MONGO_PASSWORD").context("MONGO_PASSWORD must be set")?;
    let auth_source = env::var("MONGO_AUTH_SOURCE").ok();

    let encoded_username = url_encode(&username);
    let encoded_password = url_encode(&password);

    let uri = match auth_source {
        Some(source) => format!(
            "mongodb://{}:{}@{}:{}/?authSource={}",
            encoded_username, encoded_password, host, port, source
        ),
        None => format!(
            "mongodb://{}:{}@{}:{}",
            encoded_username, encoded_password, host, port
        ),
    };
    let database_name =
        env::var("MONGO_CONNECTION_NAME").context("MONGO_CONNECTION_NAME must be set")?;

    let client = Client::with_uri_str(&uri).await?;
    let db = client.database(database_name.as_str());

    // Wait briefly so a reachable database has its indexes before any request
    // is served; an unreachable database doesn't stall startup, and the spawned
    // tasks keep retrying in the background after the timeout. The view unique
    // index in particular must exist before `View::record` runs, or concurrent
    // same-day views could insert duplicates and inflate the rollup.
    let like_index_task = tokio::spawn(ensure_indexes(db.clone()));
    let view_index_task = tokio::spawn(ensure_view_indexes(db.clone()));

    let indexes_ready = async {
        let _ = like_index_task.await;
        let _ = view_index_task.await;
    };
    if tokio::time::timeout(Duration::from_secs(5), indexes_ready)
        .await
        .is_err()
    {
        log::warn!("Index creation still pending after 5s; continuing startup");
    }

    Ok(db)
}

/// The unique compound index is the backstop that makes `Like::toggle`
/// race-safe; its `postSlug` prefix also serves the per-post count queries.
///
/// Self-healing, retried until it succeeds: duplicates (pre-existing data, or
/// toggles racing before the index lands on the very first boot) make creation
/// fail with a duplicate-key error — they are removed (oldest kept) and
/// creation is retried, so the collection always converges to "unique index,
/// no duplicates". Connectivity errors retry with capped backoff; while the
/// database is unreachable, like inserts fail too, so no duplicates can form.
async fn ensure_indexes(db: Database) {
    let collection = db.collection::<Like>("like");

    let index = IndexModel::builder()
        .keys(doc! { "postSlug": 1, "ipHash": 1, "visitorId": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();

    let mut backoff = INDEX_RETRY_BACKOFF;

    loop {
        match collection.create_index(index.clone()).await {
            Ok(_) => return,
            Err(e) if is_duplicate_key_error(&e) => {
                log::warn!("Duplicate likes block unique index creation; deduplicating");
                if let Err(e) = remove_duplicate_likes(&collection).await {
                    log::error!("Failed to deduplicate likes: {}", e);
                    tokio::time::sleep(backoff).await;
                }
            }
            Err(e) => {
                log::warn!(
                    "Like index creation failed (retrying in {:?}): {}",
                    backoff,
                    e
                );
                tokio::time::sleep(backoff).await;
            }
        }

        backoff = (backoff * 2).min(INDEX_RETRY_BACKOFF_MAX);
    }
}

async fn remove_duplicate_likes(collection: &Collection<Like>) -> mongodb::error::Result<()> {
    let pipeline = vec![
        doc! { "$group": {
            "_id": { "postSlug": "$postSlug", "ipHash": "$ipHash", "visitorId": "$visitorId" },
            "ids": { "$push": "$_id" },
            "count": { "$sum": 1 },
        }},
        doc! { "$match": { "count": { "$gt": 1 } } },
    ];

    let mut cursor = collection.aggregate(pipeline).await?;
    while let Some(group) = cursor.try_next().await? {
        let Ok(ids) = group.get_array("ids") else {
            continue;
        };
        let extras: Vec<Bson> = ids.iter().skip(1).cloned().collect();
        if !extras.is_empty() {
            collection
                .delete_many(doc! { "_id": { "$in": extras } })
                .await?;
        }
    }

    Ok(())
}

/// Creates the `view_event` indexes: a TTL index that bounds the collection and
/// a unique compound index `(postSlug, ipHash, visitorId, bucket)` enforcing
/// "one view per visitor/post/day". Self-healing on the same principle as the
/// like index — the per-post count is derived from these events, so removing a
/// stray duplicate is all that's needed to keep counts correct.
async fn ensure_view_indexes(db: Database) {
    let events = db.collection::<ViewEvent>(EVENT_COLLECTION);

    let ttl_index = IndexModel::builder()
        .keys(doc! { "createdAt": 1 })
        .options(IndexOptions::builder().expire_after(VIEW_EVENT_TTL).build())
        .build();

    let unique_index = IndexModel::builder()
        .keys(doc! { "postSlug": 1, "ipHash": 1, "visitorId": 1, "bucket": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();

    let mut backoff = INDEX_RETRY_BACKOFF;

    loop {
        let ttl_result = events.create_index(ttl_index.clone()).await;
        let unique_result = events.create_index(unique_index.clone()).await;

        match (ttl_result, unique_result) {
            (Ok(_), Ok(_)) => return,
            (_, Err(e)) if is_duplicate_key_error(&e) => {
                log::warn!("Duplicate view events block unique index creation; deduplicating");
                if let Err(e) = remove_duplicate_view_events(&events).await {
                    log::error!("Failed to deduplicate view events: {}", e);
                    tokio::time::sleep(backoff).await;
                }
            }
            (ttl, unique) => {
                if let Err(e) = ttl {
                    log::warn!(
                        "View TTL index creation failed (retrying in {:?}): {}",
                        backoff,
                        e
                    );
                }
                if let Err(e) = unique {
                    log::warn!(
                        "View unique index creation failed (retrying in {:?}): {}",
                        backoff,
                        e
                    );
                }
                tokio::time::sleep(backoff).await;
            }
        }

        backoff = (backoff * 2).min(INDEX_RETRY_BACKOFF_MAX);
    }
}

async fn remove_duplicate_view_events(
    collection: &Collection<ViewEvent>,
) -> mongodb::error::Result<()> {
    let pipeline = vec![
        doc! { "$group": {
            "_id": {
                "postSlug": "$postSlug",
                "ipHash": "$ipHash",
                "visitorId": "$visitorId",
                "bucket": "$bucket",
            },
            "ids": { "$push": "$_id" },
            "count": { "$sum": 1 },
        }},
        doc! { "$match": { "count": { "$gt": 1 } } },
    ];

    let mut cursor = collection.aggregate(pipeline).await?;
    while let Some(group) = cursor.try_next().await? {
        let Ok(ids) = group.get_array("ids") else {
            continue;
        };
        let extras: Vec<Bson> = ids.iter().skip(1).cloned().collect();
        if !extras.is_empty() {
            collection
                .delete_many(doc! { "_id": { "$in": extras } })
                .await?;
        }
    }

    Ok(())
}
