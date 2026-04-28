//! Authentication and account management handlers.

use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use std::collections::HashSet;

use crate::kingshot_api;
use crate::parser::load_appointments;
use crate::schedule::{schedule_construction_day, schedule_research_day, schedule_troops_day};

use super::persistence::{save_accounts, schedule_key};
use super::state::{
    generate_friend_code, Account, AppState, CreateAccountRequest, CreateAccountResponse,
    KingshotLookupRequest, LoginRequest, ScheduleData, ServerInfo, UpdateProfileRequest,
};

/// Create account endpoint
pub async fn create_account(
    req: web::Json<CreateAccountRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let account_name = req.account_name.trim().to_lowercase();

    if account_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CreateAccountResponse {
            success: false,
            message: "Account name cannot be empty".to_string(),
            schedule_url: None,
        }));
    }

    let password = req.password.clone().unwrap_or_default();
    if password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CreateAccountResponse {
            success: false,
            message: "Password is required".to_string(),
            schedule_url: None,
        }));
    }

    let mut accounts = state.accounts.lock().unwrap();
    if accounts.contains_key(&account_name) {
        return Ok(HttpResponse::BadRequest().json(CreateAccountResponse {
            success: false,
            message: "Account name already exists".to_string(),
            schedule_url: None,
        }));
    }

    let account = Account {
        account_name: account_name.clone(),
        server_number: req.server_number,
        password,
        in_game_name: req.in_game_name.clone(),
        player_id: req.player_id.clone().and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }),
        oauth_provider: None,
        oauth_id: None,
        admin: false,
        alliance_access: false,
        alliance_id: None,
        alliance_tag: None,
        alliance_name: None,
        friend_code: Some(generate_friend_code()),
    };

    accounts.insert(account_name.clone(), account);

    let mut schedules = state.schedules.lock().unwrap();
    let key = schedule_key(&account_name, req.server_number);
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

    let schedule_url = format!("/{}/{}", account_name, req.server_number);

    Ok(HttpResponse::Ok().json(CreateAccountResponse {
        success: true,
        message: "Account created successfully".to_string(),
        schedule_url: Some(schedule_url),
    }))
}

/// Account login endpoint (for upload authentication)
pub async fn account_login(
    path: web::Path<(String, u32)>,
    req: web::Json<LoginRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();

    let accounts = state.accounts.lock().unwrap();
    if let Some(account) = accounts.get(&account_name) {
        if account.oauth_provider.is_some() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "This account uses OAuth. Sign in via the main login."
            })));
        }
        if account.password == req.password {
            Ok(HttpResponse::Ok().json(serde_json::json!({"success": true})))
        } else {
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Invalid password"
            })))
        }
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Account not found"
        })))
    }
}

/// CSV upload endpoint
pub async fn account_upload(
    path: web::Path<(String, u32)>,
    _req: HttpRequest,
    body: web::Bytes,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();

    let accounts = state.accounts.lock().unwrap();
    let _account = accounts
        .get(&account_name)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Account not found"))?;

    let session_ok = session
        .get::<String>("account_name")
        .ok()
        .flatten()
        .map(|s| s.to_lowercase() == account_name)
        .unwrap_or(false)
        && session
            .get::<u32>("server_number")
            .ok()
            .flatten()
            .map(|n| n == server_number)
            .unwrap_or(false);

    if !session_ok {
        drop(accounts);
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }
    drop(accounts);

    let csv_path = {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "ks_upload_{}_{}_{}.csv",
            account_name,
            server_number,
            uuid::Uuid::new_v4()
        ));
        p
    };
    std::fs::write(&csv_path, &body).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to buffer upload: {}", e))
    })?;

    let result = match load_appointments(&csv_path, None, None, None) {
        Ok(entries) => {
            let construction_schedule = schedule_construction_day(&entries);
            let research_schedule = schedule_research_day(&entries, &construction_schedule);
            let troops_schedule = schedule_troops_day(&entries);

            let mut schedules = state.schedules.lock().unwrap();
            let key = schedule_key(&account_name, server_number);
            let scheduled_ids: Vec<String> = {
                let mut ids = HashSet::new();
                for appt in construction_schedule.appointments.values() {
                    ids.insert(appt.player_id.clone());
                }
                for appt in research_schedule.appointments.values() {
                    ids.insert(appt.player_id.clone());
                }
                for appt in troops_schedule.appointments.values() {
                    ids.insert(appt.player_id.clone());
                }
                ids.into_iter().collect()
            };
            schedules.insert(
                key,
                ScheduleData {
                    construction_schedule: Some(construction_schedule),
                    research_schedule: Some(research_schedule),
                    troops_schedule: Some(troops_schedule),
                    entries: Some(entries),
                    scheduled_player_ids: Some(scheduled_ids),
                },
            );

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Schedule generated successfully"
            })))
        }
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to process CSV: {}", e)
        }))),
    };
    let _ = std::fs::remove_file(&csv_path);
    result
}

/// Login endpoint (uses account name + password, sets session cookie)
pub async fn login_api(
    req: web::Json<LoginRequest>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let account_name = req
        .account_name
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Account name required"))?
        .trim()
        .to_lowercase();

    let accounts = state.accounts.lock().unwrap();
    if let Some(account) = accounts.get(&account_name) {
        if account.oauth_provider.is_some() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "This account uses OAuth. Please sign in with Discord or Google."
            })));
        }
        if account.password == req.password {
            session
                .insert("account_name", &account.account_name)
                .map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to set session: {}",
                        e
                    ))
                })?;
            session
                .insert("server_number", account.server_number)
                .map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to set session: {}",
                        e
                    ))
                })?;

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "account_name": account.account_name,
                "server_number": account.server_number
            })))
        } else {
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Invalid password"
            })))
        }
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Account not found"
        })))
    }
}

/// Get session info endpoint
pub async fn get_session_info(
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let account_name: Option<String> = session
        .get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    let server_number: Option<u32> = session
        .get("server_number")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;

    if let (Some(account_name), Some(server_number)) = (account_name, server_number) {
        let (player_id, in_game_name, is_admin, alliance_access, friend_code) = {
            let accounts = state.accounts.lock().unwrap();
            let acc = accounts.get(&account_name);
            let (pid, ign, admin, aaccess) = acc
                .map(|a| {
                    (
                        a.player_id.clone(),
                        a.in_game_name.clone(),
                        a.admin,
                        a.alliance_access,
                    )
                })
                .unwrap_or((None, String::new(), false, false));
            let fc = acc.and_then(|a| a.friend_code.clone()).unwrap_or_default();
            drop(accounts);
            let invites = super::alliance_invites::load_invites(&state.data_dir).await;
            let has_inv = invites.values().any(|i| {
                i.to_friend_code.eq_ignore_ascii_case(&fc)
                    && (i.status == "pending" || i.status == "accepted")
            });
            (pid, ign, admin, aaccess || has_inv, fc)
        };
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "account_name": account_name,
            "server_number": server_number,
            "player_id": player_id,
            "in_game_name": in_game_name,
            "is_admin": is_admin,
            "alliance_access": alliance_access,
            "friend_code": friend_code
        })))
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        })))
    }
}

/// Update profile endpoint (account_name, server_number, in_game_name)
pub async fn update_profile(
    req: web::Json<UpdateProfileRequest>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let session_account: Option<String> = session
        .get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    let session_server: Option<u32> = session
        .get("server_number")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;

    let old_account_name = match (&session_account, &session_server) {
        (Some(a), Some(_)) => a.clone(),
        _ => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not authenticated"
            })));
        }
    };

    let mut accounts = state.accounts.lock().unwrap();
    let account = accounts
        .get(&old_account_name)
        .cloned()
        .ok_or_else(|| actix_web::error::ErrorNotFound("Account not found"))?;

    let new_account_name = req
        .account_name
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| account.account_name.clone());
    let new_server = req.server_number.unwrap_or(account.server_number);
    let new_in_game_name = req
        .in_game_name
        .as_ref()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| account.in_game_name.clone());

    if new_account_name != old_account_name {
        if accounts.contains_key(&new_account_name) {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Account name already exists"
            })));
        }
        let mut updated = account.clone();
        updated.account_name = new_account_name.clone();
        updated.server_number = new_server;
        updated.in_game_name = new_in_game_name.clone();
        accounts.remove(&old_account_name);
        accounts.insert(new_account_name.clone(), updated);

        let mut schedules = state.schedules.lock().unwrap();
        let old_key = schedule_key(&old_account_name, account.server_number);
        let new_key = schedule_key(&new_account_name, new_server);
        if let Some(data) = schedules.remove(&old_key) {
            schedules.insert(new_key, data);
        }
        drop(schedules);

        let mut current_forms = state.current_forms.lock().unwrap();
        let old_cf_key = schedule_key(&old_account_name, account.server_number);
        let new_cf_key = schedule_key(&new_account_name, new_server);
        if let Some(code) = current_forms.remove(&old_cf_key) {
            current_forms.insert(new_cf_key, code);
        }
        drop(current_forms);

        let mut forms = state.forms.lock().unwrap();
        for form in forms.values_mut() {
            if form.account_name.to_lowercase() == old_account_name
                && form.server_number == account.server_number
            {
                form.account_name = new_account_name.clone();
                form.server_number = new_server;
            }
        }
        drop(forms);

        session
            .insert("account_name", &new_account_name)
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
        session
            .insert("server_number", new_server)
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
    } else {
        let mut updated = account.clone();
        updated.server_number = new_server;
        updated.in_game_name = new_in_game_name.clone();
        accounts.insert(old_account_name.clone(), updated);

        if new_server != account.server_number {
            let mut schedules = state.schedules.lock().unwrap();
            let old_key = schedule_key(&old_account_name, account.server_number);
            let new_key = schedule_key(&old_account_name, new_server);
            if let Some(data) = schedules.remove(&old_key) {
                schedules.insert(new_key, data);
            }
            drop(schedules);

            let mut current_forms = state.current_forms.lock().unwrap();
            let old_cf_key = schedule_key(&old_account_name, account.server_number);
            let new_cf_key = schedule_key(&old_account_name, new_server);
            if let Some(code) = current_forms.remove(&old_cf_key) {
                current_forms.insert(new_cf_key, code);
            }
            drop(current_forms);

            let mut forms = state.forms.lock().unwrap();
            for form in forms.values_mut() {
                if form.account_name.to_lowercase() == old_account_name
                    && form.server_number == account.server_number
                {
                    form.server_number = new_server;
                }
            }
            drop(forms);

            session.insert("server_number", new_server).map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("Session: {}", e))
            })?;
        } else {
            let mut updated = account.clone();
            updated.server_number = new_server;
            updated.in_game_name = new_in_game_name.clone();
            accounts.insert(old_account_name.clone(), updated);
        }
    }

    let snapshot = accounts.clone();
    drop(accounts);
    save_accounts(&state.data_dir, &snapshot)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;

    let final_name = if new_account_name != old_account_name {
        new_account_name
    } else {
        old_account_name
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "account_name": final_name,
        "server_number": new_server,
        "in_game_name": new_in_game_name
    })))
}

/// Kingshot ID lookup - fetch from API and update profile
pub async fn kingshot_lookup_profile(
    req: web::Json<KingshotLookupRequest>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let session_account: Option<String> = session
        .get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    let session_server: Option<u32> = session
        .get("server_number")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;

    let account_name = match (&session_account, &session_server) {
        (Some(a), Some(_)) => a.clone(),
        _ => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not authenticated"
            })));
        }
    };

    let player_id = req.player_id.trim();
    if player_id.is_empty() || !player_id.chars().all(|c| c.is_ascii_digit()) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid Kingshot ID"
        })));
    }

    let player = match kingshot_api::fetch_player(player_id).await {
        Ok(p) => p,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": e
            })));
        }
    };

    let server_from_kid = player.kid.trim().parse::<u32>().unwrap_or(0);

    let (server_number, player_id, in_game_name, snapshot) = {
        let mut accounts = state.accounts.lock().unwrap();
        let account = accounts
            .get_mut(&account_name)
            .ok_or_else(|| actix_web::error::ErrorNotFound("Account not found"))?;

        account.in_game_name = player.nickname.clone();
        account.player_id = Some(player.fid.clone());
        if server_from_kid > 0 {
            account.server_number = server_from_kid;
        }

        let server_number = account.server_number;
        let player_id = player.fid.clone();
        let in_game_name = player.nickname.clone();
        let snapshot = accounts.clone();
        (server_number, player_id, in_game_name, snapshot)
    };

    if server_from_kid > 0 {
        session
            .insert("server_number", server_from_kid)
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
    }

    save_accounts(&state.data_dir, &snapshot)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "in_game_name": in_game_name,
        "server_number": server_number,
        "player_id": player_id
    })))
}

/// Dev-only: instant login as normal user. Only works when DEV_MODE=true.
/// Creates "devuser" account if it doesn't exist (non-admin, no alliance access).
pub async fn dev_login_user(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    if std::env::var("DEV_MODE").as_deref() != Ok("true") {
        return Ok(HttpResponse::NotFound().finish());
    }

    const DEV_ACCOUNT: &str = "devuser";
    const DEV_SERVER: u32 = 140;

    let (account, snapshot_to_save) = {
        let mut accounts = state.accounts.lock().unwrap();
        if let Some(acc) = accounts.get(DEV_ACCOUNT) {
            (acc.clone(), None)
        } else {
            let account = Account {
                account_name: DEV_ACCOUNT.to_string(),
                server_number: DEV_SERVER,
                password: String::new(),
                in_game_name: "Dev User".to_string(),
                player_id: None,
                oauth_provider: None,
                oauth_id: None,
                admin: false,
                alliance_access: false,
                alliance_id: None,
                alliance_tag: None,
                alliance_name: None,
                friend_code: Some(generate_friend_code()),
            };
            accounts.insert(DEV_ACCOUNT.to_string(), account.clone());

            let mut schedules = state.schedules.lock().unwrap();
            schedules.insert(
                schedule_key(DEV_ACCOUNT, DEV_SERVER),
                ScheduleData {
                    construction_schedule: None,
                    research_schedule: None,
                    troops_schedule: None,
                    entries: None,
                    scheduled_player_ids: None,
                },
            );
            drop(schedules);

            (account, Some(accounts.clone()))
        }
    };

    if let Some(snap) = snapshot_to_save {
        save_accounts(&state.data_dir, &snap).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;
    }

    session
        .insert("account_name", &account.account_name)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
    session
        .insert("server_number", account.server_number)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;

    let redirect_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let location = format!(
        "{}/dashboard/{}",
        redirect_url.trim_end_matches('/'),
        account.account_name
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", location))
        .finish())
}

/// Dev-only: instant login as test account. Only works when DEV_MODE=true.
/// Creates "devtest" account if it doesn't exist, sets session, redirects to dashboard.
pub async fn dev_login(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    if std::env::var("DEV_MODE").as_deref() != Ok("true") {
        return Ok(HttpResponse::NotFound().finish());
    }

    const DEV_ACCOUNT: &str = "devtest";
    const DEV_SERVER: u32 = 1;

    let (account, snapshot_to_save) = {
        let mut accounts = state.accounts.lock().unwrap();
        if let Some(acc) = accounts.get(DEV_ACCOUNT) {
            (acc.clone(), None)
        } else {
            let account = Account {
                account_name: DEV_ACCOUNT.to_string(),
                server_number: DEV_SERVER,
                password: String::new(),
                in_game_name: "Dev Tester".to_string(),
                player_id: None,
                oauth_provider: None,
                oauth_id: None,
                admin: true,
                alliance_access: true,
                alliance_id: None,
                alliance_tag: None,
                alliance_name: None,
                friend_code: Some(generate_friend_code()),
            };
            accounts.insert(DEV_ACCOUNT.to_string(), account.clone());

            let mut schedules = state.schedules.lock().unwrap();
            schedules.insert(
                schedule_key(DEV_ACCOUNT, DEV_SERVER),
                ScheduleData {
                    construction_schedule: None,
                    research_schedule: None,
                    troops_schedule: None,
                    entries: None,
                    scheduled_player_ids: None,
                },
            );
            drop(schedules);

            (account, Some(accounts.clone()))
        }
    };

    if let Some(snap) = snapshot_to_save {
        save_accounts(&state.data_dir, &snap).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;
    }

    session
        .insert("account_name", &account.account_name)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;
    session
        .insert("server_number", account.server_number)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Session: {}", e)))?;

    let redirect_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let location = format!(
        "{}/dashboard/{}",
        redirect_url.trim_end_matches('/'),
        account.account_name
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", location))
        .finish())
}

/// Logout endpoint
pub async fn logout_api(session: Session) -> Result<HttpResponse> {
    session.purge();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    })))
}

/// List all servers
pub async fn list_servers(state: web::Data<AppState>) -> Result<HttpResponse> {
    let accounts = state.accounts.lock().unwrap();
    let mut servers: Vec<ServerInfo> = accounts
        .values()
        .map(|acc| ServerInfo {
            account_name: acc.account_name.clone(),
            server_number: acc.server_number,
        })
        .collect();
    drop(accounts);

    servers.sort_by(|a, b| {
        a.account_name
            .cmp(&b.account_name)
            .then_with(|| a.server_number.cmp(&b.server_number))
    });

    Ok(HttpResponse::Ok().json(servers))
}
