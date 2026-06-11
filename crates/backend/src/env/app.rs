use anyhow::{Context, Result};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct Env {
    pub port: u16,
    pub host: Cow<'static, str>,
    pub jwt_secret: Cow<'static, str>,
    pub cookie_domain: Cow<'static, str>,
}

impl Env {
    pub fn new() -> Result<Self> {
        let port = match std::env::var("PORT") {
            Ok(port) => port.parse().unwrap_or(8080),
            Err(_) => 8080,
        };
        let host = match std::env::var("HOST") {
            Ok(host) => Cow::Owned(host),
            Err(_) => Cow::Owned("http://localhost/".to_string()),
        };
        let jwt_secret = std::env::var("JWT_SECRET")
            .map(Cow::Owned)
            .context("JWT_SECRET must be set")?;
        let cookie_domain = match std::env::var("COOKIE_DOMAIN") {
            Ok(cookie_domain) => Cow::Owned(cookie_domain),
            Err(_) => Cow::Owned("localhost".to_string()),
        };

        Ok(Self {
            port,
            host,
            jwt_secret,
            cookie_domain,
        })
    }
}
