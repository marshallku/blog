use anyhow::{Context, Result};
use bson::{doc, Bson};
use futures::TryStreamExt;
use mongodb::{options::IndexOptions, Client, Collection, Database, IndexModel};
use std::env;
use std::time::Duration;

use crate::models::like::{is_duplicate_key_error, Like};
use crate::utils::encode::url_encode;

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

    // Wait briefly so a reachable database has its index before any request
    // is served; an unreachable database doesn't stall startup, and the
    // spawned task keeps retrying in the background after the timeout.
    let index_task = tokio::spawn(ensure_indexes(db.clone()));
    if tokio::time::timeout(Duration::from_secs(5), index_task)
        .await
        .is_err()
    {
        log::warn!("Like index creation still pending after 5s; continuing startup");
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
