//! Stateless, signed OAuth pending payload.
//!
//! The `state` parameter we send to the OAuth provider is the same one
//! returned in the callback. We sign a JSON envelope `{state, pkce, provider,
//! exp}` with HMAC-SHA256 (`OAUTH_HMAC_KEY`/derived from `SESSION_SECRET`),
//! base64url-encode it, and stash the result in an HttpOnly cookie scoped to
//! `/api/auth`. On the callback we verify the signature, check that the echoed
//! `state` matches the cookie's `state`, check the expiry, and use the embedded
//! PKCE verifier to exchange the code. No server-side cache, no DB hit, no
//! cross-pod state to lose.
//!
//! This intentionally re-implements only what we need rather than pulling in
//! a JWT crate: tiny payload, single signing key, single verifier.

use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

pub const OAUTH_PENDING_COOKIE: &str = "oauth_pending";
pub const OAUTH_PENDING_TTL_SECS: u64 = 600;

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthPending {
    pub state: String,
    pub pkce_verifier: String,
    pub provider: String,
    pub exp: u64,
}

impl OAuthPending {
    pub fn new(state: String, pkce_verifier: String, provider: String) -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + OAUTH_PENDING_TTL_SECS;
        Self {
            state,
            pkce_verifier,
            provider,
            exp,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.exp
    }
}

fn b64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .ok()
}

/// Encode a payload, signing it with `key`. Output is `<base64-payload>.<base64-mac>`.
pub fn sign(payload: &OAuthPending, key: &[u8]) -> Option<String> {
    let json = serde_json::to_vec(payload).ok()?;
    let payload_b64 = b64_encode(&json);

    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).ok()?;
    mac.update(payload_b64.as_bytes());
    let tag = mac.finalize().into_bytes();
    let tag_b64 = b64_encode(&tag);

    Some(format!("{payload_b64}.{tag_b64}"))
}

/// Verify the cookie value, returning the parsed payload on success.
pub fn verify(token: &str, key: &[u8]) -> Option<OAuthPending> {
    let (payload_b64, tag_b64) = token.split_once('.')?;
    let tag = b64_decode(tag_b64)?;

    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).ok()?;
    mac.update(payload_b64.as_bytes());
    mac.verify_slice(&tag).ok()?;

    let payload_bytes = b64_decode(payload_b64)?;
    let pending: OAuthPending = serde_json::from_slice(&payload_bytes).ok()?;
    if pending.is_expired() {
        return None;
    }
    Some(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_signed_payload() {
        let key = b"super-secret-test-key-at-least-32-bytes-long";
        let payload = OAuthPending::new("state".into(), "verifier".into(), "discord".into());
        let token = sign(&payload, key).expect("sign");
        let parsed = verify(&token, key).expect("verify");
        assert_eq!(parsed.state, "state");
        assert_eq!(parsed.pkce_verifier, "verifier");
        assert_eq!(parsed.provider, "discord");
    }

    #[test]
    fn rejects_modified_payload() {
        let key = b"super-secret-test-key-at-least-32-bytes-long";
        let payload = OAuthPending::new("state".into(), "verifier".into(), "discord".into());
        let token = sign(&payload, key).expect("sign");
        // flip a character in the payload section
        let mut bytes = token.into_bytes();
        bytes[0] = if bytes[0] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(verify(&tampered, key).is_none());
    }

    #[test]
    fn rejects_wrong_key() {
        let payload = OAuthPending::new("s".into(), "v".into(), "discord".into());
        let token = sign(&payload, b"key-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        assert!(verify(&token, b"key-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").is_none());
    }
}
