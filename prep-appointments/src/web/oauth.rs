//! OAuth2 authentication (Discord and Google) using a stateless, signed
//! `oauth_pending` cookie. The cookie carries the PKCE verifier between
//! `/api/auth/{provider}` and `/api/auth/callback`, so the flow survives pod
//! restarts and works across replicas without any server-side state.

use actix_session::Session;
use actix_web::cookie::time::Duration as CookieDuration;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpRequest, HttpResponse, Result};
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};

use super::oauth_signed::{self, OAuthPending, OAUTH_PENDING_COOKIE, OAUTH_PENDING_TTL_SECS};
use super::persistence::{save_accounts, schedule_key};
use super::state::{Account, AppState, ScheduleData};

fn generate_random_account_name(accounts: &std::collections::HashMap<String, Account>) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    loop {
        let digits: String = (0..9).map(|_| rng.gen_range(0..=9).to_string()).collect();
        let name = format!("lord{}", digits);
        if !accounts.contains_key(&name) {
            return name;
        }
    }
}

fn base_url(req: &HttpRequest) -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| {
        let conn_info = req.connection_info();
        let scheme = conn_info.scheme();
        let host = conn_info.host();
        format!("{}://{}", scheme, host)
    })
}

fn app_url() -> Option<String> {
    std::env::var("FRONTEND_URL").ok().filter(|s| !s.is_empty())
}

fn cookie_secure(req: &HttpRequest) -> bool {
    req.connection_info().scheme() == "https"
}

fn build_pending_cookie<'a>(value: String, secure: bool) -> Cookie<'a> {
    Cookie::build(OAUTH_PENDING_COOKIE, value)
        .path("/api/auth")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(OAUTH_PENDING_TTL_SECS as i64))
        .finish()
}

fn clear_pending_cookie<'a>(secure: bool) -> Cookie<'a> {
    Cookie::build(OAUTH_PENDING_COOKIE, "")
        .path("/api/auth")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(0))
        .finish()
}

fn discord_client(req: &HttpRequest) -> Result<BasicClient, HttpResponse> {
    let client_id = std::env::var("OAUTH_DISCORD_CLIENT_ID")
        .map_err(|_| HttpResponse::InternalServerError().body("OAuth not configured"))?;
    let client_secret = std::env::var("OAUTH_DISCORD_CLIENT_SECRET")
        .map_err(|_| HttpResponse::InternalServerError().body("OAuth not configured"))?;
    let base = base_url(req);
    let redirect = format!("{}/api/auth/callback?provider=discord", base);

    let auth_url =
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).map_err(|e| {
            HttpResponse::InternalServerError().body(format!("Invalid auth URL: {}", e))
        })?;
    let token_url =
        TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).map_err(|e| {
            HttpResponse::InternalServerError().body(format!("Invalid token URL: {}", e))
        })?;
    let redirect_url = RedirectUrl::new(redirect).map_err(|e| {
        HttpResponse::InternalServerError().body(format!("Invalid redirect: {}", e))
    })?;

    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url);

    Ok(client)
}

fn google_client(req: &HttpRequest) -> Result<BasicClient, HttpResponse> {
    let client_id = std::env::var("OAUTH_GOOGLE_CLIENT_ID")
        .map_err(|_| HttpResponse::InternalServerError().body("OAuth not configured"))?;
    let client_secret = std::env::var("OAUTH_GOOGLE_CLIENT_SECRET")
        .map_err(|_| HttpResponse::InternalServerError().body("OAuth not configured"))?;
    let base = base_url(req);
    let redirect = format!("{}/api/auth/callback?provider=google", base);

    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .map_err(|e| {
            HttpResponse::InternalServerError().body(format!("Invalid auth URL: {}", e))
        })?;
    let token_url =
        TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).map_err(|e| {
            HttpResponse::InternalServerError().body(format!("Invalid token URL: {}", e))
        })?;
    let redirect_url = RedirectUrl::new(redirect).map_err(|e| {
        HttpResponse::InternalServerError().body(format!("Invalid redirect: {}", e))
    })?;

    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url);

    Ok(client)
}

/// Initiate OAuth flow - redirect to provider after stashing a signed pending cookie
pub async fn oauth_initiate(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let provider = path.into_inner();
    let (client, scopes) = match provider.as_str() {
        "discord" => {
            let c = match discord_client(&req) {
                Ok(c) => c,
                Err(e) => return Ok(e),
            };
            (
                c,
                vec![
                    Scope::new("identify".to_string()),
                    Scope::new("email".to_string()),
                ],
            )
        }
        "google" => {
            let c = match google_client(&req) {
                Ok(c) => c,
                Err(e) => return Ok(e),
            };
            (
                c,
                vec![
                    Scope::new("openid".to_string()),
                    Scope::new("profile".to_string()),
                    Scope::new("email".to_string()),
                ],
            )
        }
        _ => return Ok(HttpResponse::BadRequest().body("Unknown provider")),
    };

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes)
        .set_pkce_challenge(pkce_challenge)
        .url();

    let pending = OAuthPending::new(
        csrf_token.secret().clone(),
        pkce_verifier.secret().clone(),
        provider.clone(),
    );
    let token = match oauth_signed::sign(&pending, &state.oauth_hmac_key) {
        Some(t) => t,
        None => return Ok(HttpResponse::InternalServerError().body("Failed to sign OAuth state")),
    };

    let cookie = build_pending_cookie(token, cookie_secure(&req));
    Ok(HttpResponse::Found()
        .cookie(cookie)
        .append_header(("Location", auth_url.to_string()))
        .finish())
}

/// OAuth callback - verify pending cookie, exchange code, find/create account
pub async fn oauth_callback(
    req: HttpRequest,
    query: web::Query<OAuthCallbackQuery>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let secure = cookie_secure(&req);

    let code = query
        .code
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing code"))?;
    let state_param = query
        .state
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing state"))?;
    let provider = query.provider.as_deref().unwrap_or("discord");

    let cookie_value = req
        .cookie(OAUTH_PENDING_COOKIE)
        .map(|c| c.value().to_string());
    let pending = cookie_value
        .as_deref()
        .and_then(|v| oauth_signed::verify(v, &state.oauth_hmac_key));

    let pending = match pending {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::BadRequest()
                .cookie(clear_pending_cookie(secure))
                .body("Invalid or expired OAuth state"))
        }
    };

    if pending.state.as_str() != state_param.as_str() {
        return Ok(HttpResponse::BadRequest()
            .cookie(clear_pending_cookie(secure))
            .body("OAuth state mismatch"));
    }
    if pending.provider != provider {
        return Ok(HttpResponse::BadRequest()
            .cookie(clear_pending_cookie(secure))
            .body("OAuth provider mismatch"));
    }

    let (client, user_info_url) = match provider {
        "discord" => (
            match discord_client(&req) {
                Ok(c) => c,
                Err(e) => return Ok(e),
            },
            "https://discord.com/api/users/@me",
        ),
        "google" => (
            match google_client(&req) {
                Ok(c) => c,
                Err(e) => return Ok(e),
            },
            "https://www.googleapis.com/oauth2/v2/userinfo",
        ),
        _ => {
            return Ok(HttpResponse::BadRequest()
                .cookie(clear_pending_cookie(secure))
                .body("Unknown provider"))
        }
    };

    let token_result = client
        .exchange_code(AuthorizationCode::new(code.clone()))
        .set_pkce_verifier(PkceCodeVerifier::new(pending.pkce_verifier.clone()))
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Token exchange failed: {}", e))
        })?;

    let access_token = token_result.access_token().secret();

    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let user_resp = http_client
        .get(user_info_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    if !user_resp.status().is_success() {
        return Ok(HttpResponse::InternalServerError()
            .cookie(clear_pending_cookie(secure))
            .body("Failed to fetch user info"));
    }

    let user_json: serde_json::Value = user_resp
        .json()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let (oauth_id, username) = match provider {
        "discord" => {
            let id = user_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = user_json
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("User")
                .to_string();
            (id, name)
        }
        "google" => {
            let id = user_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    user_json
                        .get("email")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                })
                .to_string();
            let name = user_json
                .get("name")
                .and_then(|v| v.as_str())
                .or_else(|| user_json.get("email").and_then(|v| v.as_str()))
                .unwrap_or("User")
                .to_string();
            (id, name)
        }
        _ => {
            return Ok(HttpResponse::BadRequest()
                .cookie(clear_pending_cookie(secure))
                .body("Unknown provider"))
        }
    };

    let base = base_url(&req);

    // Either find an existing OAuth-linked account or create a fresh one.
    // Hold the lock only as briefly as possible; perform any persistence
    // outside the lock so it can `.await`.
    enum LookupOutcome {
        Existing(Account),
        Created {
            account: Account,
            accounts_snapshot: std::collections::HashMap<String, Account>,
        },
    }

    let outcome = {
        let mut accounts = state.accounts.lock().unwrap();
        let existing = accounts
            .values()
            .find(|a| {
                a.oauth_provider.as_deref() == Some(provider)
                    && a.oauth_id.as_ref() == Some(&oauth_id)
            })
            .cloned();
        if let Some(acc) = existing {
            LookupOutcome::Existing(acc)
        } else {
            let account_name = generate_random_account_name(&accounts);
            let account = Account {
                account_name: account_name.clone(),
                server_number: 1,
                password: String::new(),
                in_game_name: username.clone(),
                player_id: None,
                oauth_provider: Some(provider.to_string()),
                oauth_id: Some(oauth_id.clone()),
                admin: false,
                alliance_access: false,
                alliance_id: None,
                alliance_tag: None,
                alliance_name: None,
                friend_code: Some(super::state::generate_friend_code()),
            };
            accounts.insert(account_name.clone(), account.clone());

            {
                let mut schedules = state.schedules.lock().unwrap();
                let key = schedule_key(&account_name, 1);
                schedules.insert(
                    key,
                    ScheduleData {
                        construction_schedule: None,
                        research_schedule: None,
                        troops_schedule: None,
                        entries: None,
                        scheduled_player_ids: None,
                    },
                );
            }

            let snapshot = accounts.clone();
            LookupOutcome::Created {
                account,
                accounts_snapshot: snapshot,
            }
        }
    };

    let account = match outcome {
        LookupOutcome::Existing(acc) => acc,
        LookupOutcome::Created {
            account,
            accounts_snapshot,
        } => {
            save_accounts(&state.data_dir, &accounts_snapshot)
                .await
                .map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to save account: {}",
                        e
                    ))
                })?;
            account
        }
    };

    session
        .insert("account_name", &account.account_name)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
    session
        .insert("server_number", account.server_number)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;

    let redirect_base = app_url().unwrap_or(base);
    Ok(HttpResponse::Found()
        .cookie(clear_pending_cookie(secure))
        .append_header((
            "Location",
            format!(
                "{}/dashboard/{}",
                redirect_base.trim_end_matches('/'),
                account.account_name
            ),
        ))
        .finish())
}

#[derive(serde::Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub provider: Option<String>,
}
