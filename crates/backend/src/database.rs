use mongodb::{Client, Database};
use std::env;

use crate::utils::encode::url_encode;

pub async fn init_db() -> mongodb::error::Result<Database> {
    let host = env::var("MONGO_HOST").expect("MONGO_HOST must be set");
    let port = env::var("MONGO_PORT").expect("MONGO_PORT must be set");
    let username = env::var("MONGO_USERNAME").expect("MONGO_USERNAME must be set");
    let password = env::var("MONGO_PASSWORD").expect("MONGO_PASSWORD must be set");
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
        env::var("MONGO_CONNECTION_NAME").expect("MONGO_CONNECTION_NAME must be set");

    let client = Client::with_uri_str(&uri).await?;
    Ok(client.database(database_name.as_str()))
}
