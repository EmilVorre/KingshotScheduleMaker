//! Admin API: list accounts, manage admin privileges.
//! Only accessible to accounts with admin: true.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};

use super::persistence::save_accounts;
use super::state::AppState;

/// Require admin session. Returns account_name if admin, Err otherwise.
pub fn require_admin(
    session: &Session,
    state: &web::Data<AppState>,
) -> Result<String, HttpResponse> {
    let account_name: String = session
        .get("account_name")
        .map_err(|_| {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Session error"}))
        })?
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not authenticated"
            }))
        })?;

    let is_admin = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(&account_name)
            .map(|a| a.admin)
            .unwrap_or(false)
    };

    if !is_admin {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Admin access required"
        })));
    }

    Ok(account_name)
}

/// GET /api/admin/accounts - List all accounts with admin flag (admin only)
pub async fn list_accounts(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let accounts = state.accounts.lock().unwrap();
    let list: Vec<serde_json::Value> = accounts
        .values()
        .map(|a| {
            serde_json::json!({
                "account_name": a.account_name,
                "server_number": a.server_number,
                "in_game_name": a.in_game_name,
                "admin": a.admin,
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "accounts": list
    })))
}

#[derive(serde::Deserialize)]
pub struct SetAdminRequest {
    pub admin: bool,
}

/// POST /api/admin/accounts/{account_name}/admin - Set admin flag (admin only)
pub async fn set_admin(
    path: web::Path<String>,
    req: Option<web::Json<SetAdminRequest>>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let target_account = path.into_inner().trim().to_lowercase();
    let admin = req.as_ref().map(|r| r.admin).unwrap_or(true);

    let snapshot = {
        let mut accounts = state.accounts.lock().unwrap();
        match accounts.get_mut(&target_account) {
            Some(a) => {
                a.admin = admin;
            }
            None => {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "error": "Account not found"
                })))
            }
        }
        accounts.clone()
    };

    if let Err(e) = save_accounts(&state.data_dir, &snapshot).await {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save: {}", e)
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "account_name": target_account,
        "admin": admin
    })))
}
