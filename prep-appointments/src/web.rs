mod admin;
mod alliance_application;
pub mod alliance_invites;
pub mod alliances;
mod auth;
mod avatar;
pub mod db;
mod feedback;
pub mod forms;
mod giftcode_auto;
mod giftcode_recipients;
mod oauth;
mod oauth_signed;
mod persistence;
pub mod schedule;
pub mod server_org;
mod state;
pub mod swordland;
pub mod tri_alliance;

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, middleware, web, App, HttpServer};
use std::collections::HashMap;
use std::sync::Mutex;

// Re-export for external use
pub use db::PgPool;
pub use persistence::*;
pub use state::*;

/// Resolve the master secret used for signed cookies. In production this MUST
/// be set in the `SESSION_SECRET` env var (>=64 raw bytes, hex- or base64-
/// encoded). We tolerate ephemeral keys only when no DATABASE_URL is configured
/// (i.e. local dev with the JSON backend).
fn load_session_secret() -> Vec<u8> {
    if let Ok(raw) = std::env::var("SESSION_SECRET") {
        let trimmed = raw.trim();
        if let Some(bytes) = decode_secret(trimmed) {
            if bytes.len() >= 64 {
                return bytes;
            }
            eprintln!(
                "SESSION_SECRET decoded to {} bytes; need at least 64. Falling back to ephemeral.",
                bytes.len()
            );
        } else {
            eprintln!("SESSION_SECRET is not valid hex or base64. Falling back to ephemeral.");
        }
    } else {
        eprintln!(
            "SESSION_SECRET is not set; using ephemeral key (sessions will not survive restarts)."
        );
    }
    let mut buf = vec![0u8; 64];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

fn decode_secret(s: &str) -> Option<Vec<u8>> {
    if let Ok(bytes) = hex_decode(s) {
        return Some(bytes);
    }
    use base64::Engine;
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(s) {
        return Some(bytes);
    }
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s) {
        return Some(bytes);
    }
    None
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, ()> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(10 + b - b'a'),
        b'A'..=b'F' => Ok(10 + b - b'A'),
        _ => Err(()),
    }
}

fn derive_oauth_hmac_key(session_secret: &[u8]) -> Vec<u8> {
    if let Ok(raw) = std::env::var("OAUTH_HMAC_KEY") {
        if let Some(bytes) = decode_secret(raw.trim()) {
            if bytes.len() >= 32 {
                return bytes;
            }
        }
        eprintln!("OAUTH_HMAC_KEY invalid or too short; deriving from SESSION_SECRET.");
    }
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"kingshot/oauth_hmac_v1");
    hasher.update(session_secret);
    hasher.finalize().to_vec()
}

pub async fn start_server(port: u16) -> std::io::Result<()> {
    let data_dir = "data".to_string();
    std::fs::create_dir_all(&data_dir)?;

    if persistence::is_postgres_backend() {
        let pool = db::PgPool::from_env()
            .map_err(|e| std::io::Error::other(format!("failed to build Postgres pool: {e}")))?;
        persistence::init_pg_pool(pool);
    }

    let accounts = persistence::load_accounts(&data_dir).await;
    let forms = persistence::load_forms(&data_dir).await;
    let mut current_forms = persistence::load_current_forms(&data_dir).await;
    current_forms.retain(|_, code| forms.contains_key(code));
    persistence::save_current_forms(&data_dir, &current_forms)
        .await
        .ok();

    let data_dir_for_task = data_dir.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            giftcode_auto::run_auto_redeem_cycle(&data_dir_for_task).await;
            interval.tick().await;
        }
    });

    let session_secret = load_session_secret();
    let oauth_hmac_key = derive_oauth_hmac_key(&session_secret);

    let app_state = web::Data::new(state::AppState {
        accounts: Mutex::new(accounts),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms),
        current_forms: Mutex::new(current_forms),
        data_dir,
        pg: persistence::pg_pool().cloned(),
        oauth_hmac_key,
    });

    let secret_key = Key::derive_from(&session_secret);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(middleware::Logger::default())
            .route("/api/create-account", web::post().to(auth::create_account))
            .route("/api/login", web::post().to(auth::login_api))
            .route("/api/dev-login", web::get().to(auth::dev_login))
            .route("/api/dev-login-user", web::get().to(auth::dev_login_user))
            .route("/api/logout", web::post().to(auth::logout_api))
            .route("/api/session", web::get().to(auth::get_session_info))
            .route("/api/profile/update", web::put().to(auth::update_profile))
            .route(
                "/api/profile/kingshot-lookup",
                web::post().to(auth::kingshot_lookup_profile),
            )
            .route("/api/auth/callback", web::get().to(oauth::oauth_callback))
            .service(
                web::resource("/api/auth/{provider}").route(web::get().to(oauth::oauth_initiate)),
            )
            .service(
                web::resource("/api/avatar/{player_id}").route(web::get().to(avatar::get_avatar)),
            )
            .route(
                "/api/generate-schedule",
                web::post().to(schedule::generate_schedule_api),
            )
            .route(
                "/api/public-schedule/{account_name}/{form_code}/{day}",
                web::get().to(schedule::get_schedule_by_form_code),
            )
            .route("/api/servers", web::get().to(auth::list_servers))
            .route("/api/admin/accounts", web::get().to(admin::list_accounts))
            .route(
                "/api/admin/accounts/{account_name}/admin",
                web::post().to(admin::set_admin),
            )
            .route("/api/feedback", web::post().to(feedback::submit_feedback))
            .route(
                "/api/admin/feedback",
                web::get().to(feedback::list_feedback),
            )
            .service(
                web::resource("/api/alliance-application")
                    .route(web::get().to(alliance_application::get_my_application))
                    .route(web::post().to(alliance_application::submit_application)),
            )
            .route(
                "/api/admin/alliance-applications",
                web::get().to(alliance_application::list_applications_admin),
            )
            .service(
                web::resource("/api/admin/alliance-applications/{id}/approve")
                    .route(web::post().to(alliance_application::approve_application)),
            )
            .service(
                web::resource("/api/admin/alliance-applications/{id}/reject")
                    .route(web::post().to(alliance_application::reject_application)),
            )
            .service(
                web::resource("/api/admin/feedback/{id}/archive")
                    .route(web::post().to(feedback::archive_feedback)),
            )
            .service(
                web::resource("/form/{code}/api/config")
                    .route(web::get().to(forms::get_form_config_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/check-submission/{player_id}")
                    .route(web::get().to(forms::check_submission_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/player-lookup/{player_id}")
                    .route(web::get().to(forms::player_lookup_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/stats")
                    .route(web::get().to(forms::get_form_stats_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/submit")
                    .route(web::post().to(forms::submit_form_by_code)),
            )
            .service(
                web::resource("/tyrant-form/{code}/api/config")
                    .route(web::get().to(server_org::tyrant_public_config)),
            )
            .service(
                web::resource("/tyrant-form/{code}/api/submit")
                    .route(web::post().to(server_org::tyrant_public_submit)),
            )
            .service(
                web::resource("/tyrant-form/{code}/api/player-lookup/{player_id}")
                    .route(web::get().to(server_org::tyrant_player_lookup)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/create")
                    .route(web::post().to(forms::create_form)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/config")
                    .route(web::put().to(forms::update_form_config)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/current")
                    .route(web::get().to(forms::get_current_form_info)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/old")
                    .route(web::get().to(forms::list_old_forms_api)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/reopen")
                    .route(web::post().to(forms::reopen_form_api)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/previous")
                    .route(web::get().to(forms::get_previous_form_config)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/download-csv")
                    .route(web::get().to(forms::download_form_csv)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/player/{player_id}")
                    .route(web::get().to(forms::get_player_by_id)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/login")
                    .route(web::post().to(auth::account_login)),
            )
            .service(web::resource("/{account_name}/{server}/api/upload").to(auth::account_upload))
            .service(
                web::resource("/{account_name}/{server}/api/stats")
                    .route(web::get().to(schedule::get_stats)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/schedule/clear")
                    .route(web::post().to(schedule::clear_schedule_api)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/schedule/{day}/slot")
                    .route(web::put().to(schedule::update_schedule_slot)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/schedule/{day}")
                    .route(web::get().to(schedule::get_schedule)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/submissions")
                    .route(web::get().to(forms::get_form_submissions)),
            )
            .route(
                "/{account_name}/{server}/api/friend-code",
                web::get().to(alliance_invites::get_friend_code),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliance-invites")
                    .route(web::get().to(alliance_invites::list_invites))
                    .route(web::post().to(alliance_invites::create_invite)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliance-invites/{id}/accept")
                    .route(web::post().to(alliance_invites::accept_invite)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliance-invites/{id}/reject")
                    .route(web::post().to(alliance_invites::reject_invite)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliance-invites/{id}/revoke")
                    .route(web::post().to(alliance_invites::revoke_invite)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliances")
                    .route(web::get().to(alliances::list_alliances)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliances/{alliance_slug}/refresh-names")
                    .route(web::post().to(alliances::refresh_names)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliance-members")
                    .route(web::post().to(alliances::add_player)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliance-members/{alliance_slug}/{player_id}")
                    .route(web::delete().to(alliances::remove_player)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/giftcode-recipients")
                    .route(web::get().to(giftcode_recipients::get_recipients))
                    .route(web::put().to(giftcode_recipients::set_recipients)),
            )
            .route(
                "/{account_name}/{server}/api/redeem-giftcode",
                web::post().to(giftcode_recipients::redeem_giftcode),
            )
            .route(
                "/{account_name}/{server}/api/fetch-giftcodes",
                web::get().to(giftcode_recipients::fetch_giftcodes),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliances/{alliance_slug}/swordland")
                    .route(web::get().to(swordland::get_swordland))
                    .route(web::put().to(swordland::set_swordland)),
            )
            .route(
                "/{account_name}/{server}/api/alliances/{alliance_slug}/swordland/attendance",
                web::post().to(swordland::add_attendance),
            )
            .route(
                "/{account_name}/{server}/api/alliances/{alliance_slug}/swordland/attendance/{id}",
                web::put().to(swordland::update_attendance),
            )
            .service(
                web::resource("/{account_name}/{server}/api/alliances/{alliance_slug}/tri-alliance")
                    .route(web::get().to(tri_alliance::get_tri_alliance))
                    .route(web::put().to(tri_alliance::set_tri_alliance)),
            )
            .route(
                "/{account_name}/{server}/api/alliances/{alliance_slug}/tri-alliance/attendance",
                web::post().to(tri_alliance::add_attendance),
            )
            .route(
                "/{account_name}/{server}/api/alliances/{alliance_slug}/tri-alliance/attendance/{id}",
                web::put().to(tri_alliance::update_attendance),
            )
            .service(
                web::resource("/{account_name}/{server}/api/server-org/workspaces")
                    .route(web::get().to(server_org::list_my_workspaces))
                    .route(web::post().to(server_org::create_workspace)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/server-org/workspaces/{workspace_id}/invites")
                    .route(web::post().to(server_org::create_workspace_invite)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/server-org/invites")
                    .route(web::get().to(server_org::list_server_org_invites)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/server-org/invites/{invite_id}/accept")
                    .route(web::post().to(server_org::accept_workspace_invite)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/server-org/workspaces/{workspace_id}/tyrant-form")
                    .route(web::post().to(server_org::ensure_tyrant_form)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/server-org/workspaces/{workspace_id}/tyrant-submissions")
                    .route(web::get().to(server_org::list_tyrant_submissions)),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
