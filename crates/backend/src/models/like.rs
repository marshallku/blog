use bson::doc;
use chrono::{DateTime, Utc};
use mongodb::{bson::oid::ObjectId, error::Error, Database};
use serde::{Deserialize, Serialize};

const COLLECTION_NAME: &str = "like";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Like {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "postSlug")]
    pub post_slug: String,

    #[serde(rename = "ipHash")]
    pub ip_hash: String,

    #[serde(rename = "visitorId")]
    pub visitor_id: String,

    #[serde(
        rename = "createdAt",
        with = "bson::serde_helpers::chrono_datetime_as_bson_datetime"
    )]
    pub created_at: DateTime<Utc>,
}

impl Like {
    pub async fn toggle(
        db: &Database,
        post_slug: &str,
        ip_hash: &str,
        visitor_id: &str,
    ) -> Result<(bool, u64), Error> {
        let collection = db.collection::<Self>(COLLECTION_NAME);

        let filter = doc! {
            "postSlug": post_slug,
            "ipHash": ip_hash,
            "visitorId": visitor_id
        };

        let existing = collection.find_one(filter.clone()).await?;

        let liked = if existing.is_some() {
            collection.delete_one(filter).await?;
            false
        } else {
            let like = Like {
                id: None,
                post_slug: post_slug.to_string(),
                ip_hash: ip_hash.to_string(),
                visitor_id: visitor_id.to_string(),
                created_at: Utc::now(),
            };
            collection.insert_one(like).await?;
            true
        };

        let count = collection
            .count_documents(doc! { "postSlug": post_slug })
            .await?;

        Ok((liked, count))
    }

    pub async fn get_status(
        db: &Database,
        post_slug: &str,
        ip_hash: &str,
        visitor_id: &str,
    ) -> Result<(bool, u64), Error> {
        let collection = db.collection::<Self>(COLLECTION_NAME);

        let count = collection
            .count_documents(doc! { "postSlug": post_slug })
            .await?;

        // Check if user has liked using OR: visitorId match OR ipHash match
        // This handles cases where IP changes or cookie is lost
        let mut conditions = vec![doc! { "ipHash": ip_hash }];
        if !visitor_id.is_empty() {
            conditions.push(doc! { "visitorId": visitor_id });
        }

        let filter = doc! {
            "postSlug": post_slug,
            "$or": conditions
        };

        let liked = collection.find_one(filter).await?.is_some();

        Ok((liked, count))
    }
}
