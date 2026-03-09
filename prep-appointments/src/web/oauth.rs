//! OAuth2 authentication (Discord and Google).

use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};

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

/// URL to redirect to after OAuth success (e.g. frontend dev server).
/// When set, post-login redirect goes here instead of BASE_URL/request host.
fn app_url() -> Option<String> {
    std::env::var("FRONTEND_URL").ok().filter(|s| !s.is_empty())
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

/// Initiate OAuth flow - redirect to provider
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

    state
        .oauth_state_cache
        .insert(csrf_token.secret().clone(), pkce_verifier, provider.clone());

    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish())
}

/// OAuth callback - exchange code for token, get user info, create/find account
pub async fn oauth_callback(
    req: HttpRequest,
    query: web::Query<OAuthCallbackQuery>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = query
        .code
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing code"))?;
    let state_param = query
        .state
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing state"))?;
    let provider = query.provider.as_deref().unwrap_or("discord");

    let pending = state
        .oauth_state_cache
        .take(state_param)
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Invalid or expired state"))?;

    if pending.provider != provider {
        return Ok(HttpResponse::BadRequest().body("State mismatch"));
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
        _ => return Ok(HttpResponse::BadRequest().body("Unknown provider")),
    };

    let token_result = client
        .exchange_code(AuthorizationCode::new(code.clone()))
        .set_pkce_verifier(pending.pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Token exchange failed: {}", e))
        })?;

    let access_token = token_result.access_token().secret();

    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let user_resp = http_client
        .get(user_info_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    if !user_resp.status().is_success() {
        return Ok(HttpResponse::InternalServerError().body("Failed to fetch user info"));
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
        _ => return Ok(HttpResponse::BadRequest().body("Unknown provider")),
    };

    let base = base_url(&req);

    let mut accounts = state.accounts.lock().unwrap();
    let existing = accounts
        .values()
        .find(|a| {
            a.oauth_provider.as_deref() == Some(provider) && a.oauth_id.as_ref() == Some(&oauth_id)
        })
        .cloned();

    let account = if let Some(acc) = existing {
        drop(accounts);
        acc
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
        save_accounts(&state.data_dir, &accounts).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save account: {}", e))
        })?;

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
        drop(schedules);
        drop(accounts);
        account
    };

    session
        .insert("account_name", &account.account_name)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
    session
        .insert("server_number", account.server_number)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;

    let redirect_base = app_url().unwrap_or_else(|| base);
    Ok(HttpResponse::Found()
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
