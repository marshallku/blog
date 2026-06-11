use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub struct ClientIp(pub String);

const DEFAULT_TRUSTED_HEADER: &str = "x-real-ip";

fn trusted_ip_header() -> &'static str {
    static HEADER: OnceLock<String> = OnceLock::new();
    HEADER.get_or_init(|| {
        std::env::var("CLIENT_IP_HEADER")
            .map(|h| h.trim().to_lowercase())
            .ok()
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| DEFAULT_TRUSTED_HEADER.to_string())
    })
}

/// Extracts the client IP from the header set by our own reverse proxy
/// (CLIENT_IP_HEADER, default `x-real-ip`; use `cf-connecting-ip` behind
/// Cloudflare). The first entry of `X-Forwarded-For` is client-controlled
/// and trivially spoofable, so only the last entry — appended by the
/// nearest proxy — is used as a fallback.
fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get(trusted_ip_header())
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(',').next_back())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[async_trait]
impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(extract_client_ip(&parts.headers)))
    }
}

pub fn hash_ip(ip: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ip.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (key, value) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    #[test]
    fn prefers_trusted_header_over_forwarded_for() {
        let headers = headers(&[
            ("x-real-ip", "203.0.113.7"),
            ("x-forwarded-for", "6.6.6.6, 203.0.113.7"),
        ]);

        assert_eq!(extract_client_ip(&headers), "203.0.113.7");
    }

    #[test]
    fn ignores_spoofable_first_forwarded_for_entry() {
        let headers = headers(&[("x-forwarded-for", "6.6.6.6, 198.51.100.2")]);

        assert_eq!(extract_client_ip(&headers), "198.51.100.2");
    }

    #[test]
    fn falls_back_to_unknown_without_headers() {
        assert_eq!(extract_client_ip(&HeaderMap::new()), "unknown");
    }

    #[test]
    fn hash_ip_is_stable() {
        assert_eq!(hash_ip("1.2.3.4"), hash_ip("1.2.3.4"));
        assert_ne!(hash_ip("1.2.3.4"), hash_ip("1.2.3.5"));
    }
}
