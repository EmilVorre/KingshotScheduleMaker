//! OAuth state caches (CSRF/PKCE and pending account creation).
//!
//! These caches are consulted on every login round-trip. They are kept in
//! process memory for speed, and additionally write-through to the
//! `domain_documents` table when running on Postgres so that an in-flight
//! login flow survives a backend pod restart (otherwise users see
//! "Invalid or expired state" whenever the deployment rolls).

use oauth2::PkceCodeVerifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::persistence::{
    delete_domain_doc, is_postgres_backend, load_domain_doc, save_domain_doc,
};

const OAUTH_STATE_DOMAIN: &str = "oauth_state";
const OAUTH_STATE_TTL_SECS: u64 = 600;

const PENDING_OAUTH_DOMAIN: &str = "pending_oauth";
const PENDING_OAUTH_TTL_SECS: u64 = 900;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct OAuthStateCache {
    pub inner: Mutex<HashMap<String, PendingOAuthState>>,
}

pub struct PendingOAuthState {
    pub pkce_verifier: PkceCodeVerifier,
    pub provider: String,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize)]
struct PendingOAuthStateRecord {
    provider: String,
    pkce_verifier: String,
    created_at: u64,
}

impl OAuthStateCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, state: String, pkce_verifier: PkceCodeVerifier, provider: String) {
        let created = now_secs();
        let secret = pkce_verifier.secret().clone();

        {
            let mut g = self.inner.lock().unwrap();
            g.insert(
                state.clone(),
                PendingOAuthState {
                    pkce_verifier: PkceCodeVerifier::new(secret.clone()),
                    provider: provider.clone(),
                    created_at: created,
                },
            );
        }

        if is_postgres_backend() {
            let record = PendingOAuthStateRecord {
                provider,
                pkce_verifier: secret,
                created_at: created,
            };
            if let Err(e) = save_domain_doc("", OAUTH_STATE_DOMAIN, &state, &record) {
                eprintln!("Failed to persist oauth state: {e}");
            }
        }
    }

    pub fn take(&self, state: &str) -> Option<PendingOAuthState> {
        let now = now_secs();

        let mem_entry = {
            let mut g = self.inner.lock().unwrap();
            let entry = g.remove(state);
            g.retain(|_, v| now.saturating_sub(v.created_at) < OAUTH_STATE_TTL_SECS);
            entry
        };

        if let Some(entry) = mem_entry {
            if is_postgres_backend() {
                let _ = delete_domain_doc("", OAUTH_STATE_DOMAIN, state);
            }
            return Some(entry);
        }

        if !is_postgres_backend() {
            return None;
        }

        let record: PendingOAuthStateRecord = load_domain_doc("", OAUTH_STATE_DOMAIN, state)?;
        let _ = delete_domain_doc("", OAUTH_STATE_DOMAIN, state);

        if now.saturating_sub(record.created_at) >= OAUTH_STATE_TTL_SECS {
            return None;
        }

        Some(PendingOAuthState {
            pkce_verifier: PkceCodeVerifier::new(record.pkce_verifier),
            provider: record.provider,
            created_at: record.created_at,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PendingOAuthAccount {
    pub provider: String,
    pub oauth_id: String,
    pub username: String,
    pub created_at: u64,
}

pub struct PendingOAuthCache {
    pub inner: Mutex<HashMap<String, PendingOAuthAccount>>,
}

impl PendingOAuthCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, token: String, data: PendingOAuthAccount) {
        {
            let mut g = self.inner.lock().unwrap();
            g.insert(token.clone(), data.clone());
        }
        if is_postgres_backend() {
            if let Err(e) = save_domain_doc("", PENDING_OAUTH_DOMAIN, &token, &data) {
                eprintln!("Failed to persist pending oauth account: {e}");
            }
        }
    }

    pub fn take(&self, token: &str) -> Option<PendingOAuthAccount> {
        let now = now_secs();

        let mem_entry = {
            let mut g = self.inner.lock().unwrap();
            let entry = g.remove(token);
            g.retain(|_, v| now.saturating_sub(v.created_at) < PENDING_OAUTH_TTL_SECS);
            entry
        };

        if let Some(entry) = mem_entry {
            if is_postgres_backend() {
                let _ = delete_domain_doc("", PENDING_OAUTH_DOMAIN, token);
            }
            return Some(entry);
        }

        if !is_postgres_backend() {
            return None;
        }

        let record: PendingOAuthAccount = load_domain_doc("", PENDING_OAUTH_DOMAIN, token)?;
        let _ = delete_domain_doc("", PENDING_OAUTH_DOMAIN, token);

        if now.saturating_sub(record.created_at) >= PENDING_OAUTH_TTL_SECS {
            return None;
        }

        Some(record)
    }

    pub fn get(&self, token: &str) -> Option<PendingOAuthAccount> {
        if let Some(v) = {
            let g = self.inner.lock().unwrap();
            g.get(token).cloned()
        } {
            return Some(v);
        }

        if !is_postgres_backend() {
            return None;
        }

        let record: PendingOAuthAccount = load_domain_doc("", PENDING_OAUTH_DOMAIN, token)?;
        if now_secs().saturating_sub(record.created_at) >= PENDING_OAUTH_TTL_SECS {
            return None;
        }
        Some(record)
    }
}
