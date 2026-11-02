//! Invisible bot checks for the public comment form.
//!
//! Two signals, neither of which a human ever notices:
//! - a honeypot field hidden from people with CSS, so only form-filling bots fill it
//! - a signed form token issued when the form loads; a comment sent too soon after it
//!   (or without one) did not come from someone who read the post and typed a reply

use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha256;

/// Nobody reads a post and writes a comment within this many seconds of the form loading.
pub const MIN_FORM_AGE_SECONDS: i64 = 3;
/// Long enough for a tab left open for days, short enough that a harvested token goes stale.
pub const MAX_FORM_AGE_SECONDS: i64 = 7 * 24 * 60 * 60;

const PURPOSE: &str = "comment-form";

#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    Honeypot,
    MissingToken,
    InvalidToken,
    TooFast,
    Expired,
}

impl Rejection {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Honeypot => "honeypot filled",
            Self::MissingToken => "missing form token",
            Self::InvalidToken => "invalid form token",
            Self::TooFast => "submitted too fast",
            Self::Expired => "form token expired",
        }
    }
}

/// Kept apart from the auth JWT key so a form token can never pass as a session.
fn signing_key(secret: &str) -> Hmac<Sha256> {
    Hmac::new_from_slice(format!("{secret}:{PURPOSE}").as_bytes())
        .expect("HMAC accepts any key length")
}

pub fn issue_token(secret: &str, now: i64) -> String {
    let mut claims = BTreeMap::new();
    claims.insert("purpose", PURPOSE.to_string());
    claims.insert("iat", now.to_string());

    claims
        .sign_with_key(&signing_key(secret))
        .expect("signing a static claim set cannot fail")
}

pub struct Submission<'a> {
    pub honeypot: Option<&'a str>,
    pub token: Option<&'a str>,
}

pub fn check(submission: &Submission, secret: &str, now: i64) -> Result<(), Rejection> {
    if submission
        .honeypot
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(Rejection::Honeypot);
    }

    let token = submission
        .token
        .filter(|token| !token.is_empty())
        .ok_or(Rejection::MissingToken)?;
    let claims: BTreeMap<String, String> = token
        .verify_with_key(&signing_key(secret))
        .map_err(|_| Rejection::InvalidToken)?;

    if claims.get("purpose").map(String::as_str) != Some(PURPOSE) {
        return Err(Rejection::InvalidToken);
    }
    let issued_at = claims
        .get("iat")
        .and_then(|iat| iat.parse::<i64>().ok())
        .ok_or(Rejection::InvalidToken)?;

    let age = now - issued_at;
    if age < MIN_FORM_AGE_SECONDS {
        return Err(Rejection::TooFast);
    }
    if age > MAX_FORM_AGE_SECONDS {
        return Err(Rejection::Expired);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret";
    const NOW: i64 = 1_800_000_000;

    fn submit(honeypot: Option<&str>, token: Option<&str>) -> Result<(), Rejection> {
        check(&Submission { honeypot, token }, SECRET, NOW)
    }

    #[test]
    fn accepts_a_token_old_enough() {
        let token = issue_token(SECRET, NOW - 30);
        assert_eq!(submit(None, Some(&token)), Ok(()));
        assert_eq!(submit(Some(""), Some(&token)), Ok(()));
    }

    #[test]
    fn rejects_a_filled_honeypot() {
        let token = issue_token(SECRET, NOW - 30);
        assert_eq!(
            submit(Some("https://spam.example"), Some(&token)),
            Err(Rejection::Honeypot)
        );
    }

    #[test]
    fn rejects_a_missing_token() {
        assert_eq!(submit(None, None), Err(Rejection::MissingToken));
        assert_eq!(submit(None, Some("")), Err(Rejection::MissingToken));
    }

    #[test]
    fn rejects_a_token_signed_with_another_key() {
        let token = issue_token("other-secret", NOW - 30);
        assert_eq!(submit(None, Some(&token)), Err(Rejection::InvalidToken));
        assert_eq!(submit(None, Some("garbage")), Err(Rejection::InvalidToken));
    }

    #[test]
    fn rejects_a_token_for_another_purpose() {
        let mut claims = BTreeMap::new();
        claims.insert("purpose", "session".to_string());
        claims.insert("iat", (NOW - 30).to_string());
        let token = claims.sign_with_key(&signing_key(SECRET)).unwrap();

        assert_eq!(submit(None, Some(&token)), Err(Rejection::InvalidToken));
    }

    #[test]
    fn enforces_the_age_window() {
        let fresh = issue_token(SECRET, NOW - MIN_FORM_AGE_SECONDS + 1);
        assert_eq!(submit(None, Some(&fresh)), Err(Rejection::TooFast));

        let stale = issue_token(SECRET, NOW - MAX_FORM_AGE_SECONDS - 1);
        assert_eq!(submit(None, Some(&stale)), Err(Rejection::Expired));
    }
}
