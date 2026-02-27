//! OAuth state caches (CSRF/pkce and pending account creation).

use oauth2::PkceCodeVerifier;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OAuthStateCache {
    pub inner: Mutex<HashMap<String, PendingOAuthState>>,
}

pub struct PendingOAuthState {
    pub pkce_verifier: PkceCodeVerifier,
    pub provider: String,
    pub created_at: u64,
}

impl OAuthStateCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, state: String, pkce_verifier: PkceCodeVerifier, provider: String) {
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut g = self.inner.lock().unwrap();
        g.insert(state, PendingOAuthState {
            pkce_verifier,
            provider,
            created_at: created,
        });
    }

    pub fn take(&self, state: &str) -> Option<PendingOAuthState> {
        let mut g = self.inner.lock().unwrap();
        let entry = g.remove(state);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        g.retain(|_, v| now - v.created_at < 600);
        entry
    }
}

#[derive(Clone)]
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
        let mut g = self.inner.lock().unwrap();
        g.insert(token, data);
    }

    pub fn take(&self, token: &str) -> Option<PendingOAuthAccount> {
        let mut g = self.inner.lock().unwrap();
        let entry = g.remove(token);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        g.retain(|_, v| now - v.created_at < 900);
        entry
    }

    pub fn get(&self, token: &str) -> Option<PendingOAuthAccount> {
        let g = self.inner.lock().unwrap();
        g.get(token).cloned()
    }
}
