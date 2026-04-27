//! Alliance application: users apply for alliance access; admins approve.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::admin::require_admin;
use super::persistence::{load_domain_doc, save_accounts, save_domain_doc};
use super::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceApplication {
    pub id: String,
    pub account_name: String,
    pub alliance_tag: String,
    pub alliance_name: String,
    pub contact_player_id: String,
    pub server_number: u32,
    pub status: String, // "pending" | "approved" | "rejected"
    pub submitted_at: String,
}

fn load_applications(data_dir: &str) -> HashMap<String, AllianceApplication> {
    load_domain_doc(data_dir, "alliance_applications", "all")
        .unwrap_or_default()
}

fn save_applications(
    data_dir: &str,
    apps: &HashMap<String, AllianceApplication>,
) -> std::io::Result<()> {
    save_domain_doc(data_dir, "alliance_applications", "all", apps)
}

fn generate_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("app_{}", rng.gen::<u32>())
}

/// Generate a 6-character alphanumeric alliance ID
fn generate_alliance_id() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

#[derive(Deserialize)]
pub struct SubmitApplicationRequest {
    pub alliance_tag: String,
    pub alliance_name: String,
    pub contact_player_id: String,
    pub server_number: u32,
}

/// POST /api/alliance-application - Submit an application (logged-in user)
pub async fn submit_application(
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<SubmitApplicationRequest>,
) -> Result<HttpResponse> {
    let account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })))
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    let alliance_tag = body.alliance_tag.trim().to_string();
    let alliance_name = body.alliance_name.trim().to_string();
    let contact_player_id = body.contact_player_id.trim().to_string();
    let server_number = body.server_number;

    if alliance_tag.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Alliance tag is required"
        })));
    }
    if alliance_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Alliance name is required"
        })));
    }
    if contact_player_id.is_empty() || !contact_player_id.chars().all(|c| c.is_ascii_digit()) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Valid contact person player ID (digits only) is required"
        })));
    }

    let mut apps = load_applications(&state.data_dir);
    let has_pending = apps.values().any(|a| {
        a.account_name == account_name && (a.status == "pending" || a.status == "approved")
    });
    if has_pending {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "You already have a pending or approved application"
        })));
    }

    let id = generate_id();
    let app = AllianceApplication {
        id: id.clone(),
        account_name: account_name.clone(),
        alliance_tag,
        alliance_name,
        contact_player_id,
        server_number,
        status: "pending".to_string(),
        submitted_at: chrono::Local::now().to_rfc3339(),
    };
    apps.insert(id, app);
    save_applications(&state.data_dir, &apps).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Application submitted successfully"
    })))
}

/// GET /api/alliance-application - Get my application status
pub async fn get_my_application(
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })))
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    let apps = load_applications(&state.data_dir);
    let mine = apps
        .values()
        .find(|a| a.account_name == account_name)
        .cloned();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "application": mine
    })))
}

/// GET /api/admin/alliance-applications - List all applications (admin only)
pub async fn list_applications_admin(
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let apps = load_applications(&state.data_dir);
    let list: Vec<serde_json::Value> = apps
        .values()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "account_name": a.account_name,
                "alliance_tag": a.alliance_tag,
                "alliance_name": a.alliance_name,
                "contact_player_id": a.contact_player_id,
                "server_number": a.server_number,
                "status": a.status,
                "submitted_at": a.submitted_at
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "applications": list
    })))
}

/// POST /api/admin/alliance-applications/{id}/approve - Approve application (admin only)
pub async fn approve_application(
    path: web::Path<String>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let id = path.into_inner();
    let mut apps = load_applications(&state.data_dir);

    let (account_name, alliance_tag, alliance_name) = {
        let app = apps
            .get_mut(&id)
            .ok_or_else(|| actix_web::error::ErrorNotFound("Application not found"))?;

        if app.status != "pending" {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Application is not pending"
            })));
        }

        app.status = "approved".to_string();
        (
            app.account_name.clone(),
            app.alliance_tag.clone(),
            app.alliance_name.clone(),
        )
    };

    save_applications(&state.data_dir, &apps).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    let alliance_id = generate_alliance_id();

    let mut accounts = state.accounts.lock().unwrap();
    if let Some(acc) = accounts.get_mut(&account_name) {
        acc.alliance_access = true;
        acc.alliance_id = Some(alliance_id.clone());
        acc.alliance_tag = Some(alliance_tag.clone());
        acc.alliance_name = Some(alliance_name.clone());
        save_accounts(&state.data_dir, &accounts).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save accounts: {}", e))
        })?;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Application approved"
    })))
}

/// POST /api/admin/alliance-applications/{id}/reject - Reject application (admin only)
pub async fn reject_application(
    path: web::Path<String>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let id = path.into_inner();
    let mut apps = load_applications(&state.data_dir);

    let app = apps
        .get_mut(&id)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Application not found"))?;

    if app.status != "pending" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Application is not pending"
        })));
    }

    app.status = "rejected".to_string();
    save_applications(&state.data_dir, &apps).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Application rejected"
    })))
}
