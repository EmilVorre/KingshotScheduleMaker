//! Authentication and account management handlers.

use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use std::collections::HashSet;

use crate::parser::load_appointments;
use crate::schedule::{schedule_construction_day, schedule_research_day, schedule_troops_day};

use super::persistence::{save_accounts, schedule_key};
use super::state::{
    Account, AppState, CreateAccountRequest, CreateAccountResponse, LoginRequest, ScheduleData,
    ServerInfo,
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
        password: req.password.clone(),
        in_game_name: req.in_game_name.clone(),
    };

    accounts.insert(account_name.clone(), account);
    save_accounts(&state.data_dir, &accounts).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save account: {}", e))
    })?;

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
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();

    let password = req
        .headers()
        .get("X-Password")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let accounts = state.accounts.lock().unwrap();
    let account = accounts
        .get(&account_name)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Account not found"))?;

    if account.password != password || account.server_number != server_number {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }
    drop(accounts);

    std::fs::create_dir_all(&state.data_dir)?;
    let csv_path = format!("{}/{}_{}.csv", state.data_dir, account_name, server_number);
    std::fs::write(&csv_path, &body).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save file: {}", e))
    })?;

    match load_appointments(&csv_path, None, None, None) {
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
    }
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
pub async fn get_session_info(session: Session) -> Result<HttpResponse> {
    let account_name: Option<String> = session
        .get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    let server_number: Option<u32> = session
        .get("server_number")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;

    if let (Some(account_name), Some(server_number)) = (account_name, server_number) {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "account_name": account_name,
            "server_number": server_number
        })))
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        })))
    }
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
