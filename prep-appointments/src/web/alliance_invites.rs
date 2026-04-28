//! Alliance invites: share alliance edit access via friend codes.
//! Invitee must accept before gaining access (prevents overwriting their own alliance).

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::persistence::{load_domain_doc, save_accounts, save_domain_doc};
use super::state::{generate_friend_code, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceInvite {
    pub id: String,
    pub from_account: String,
    pub from_server: u32,
    pub alliance_slug: String,
    pub alliance_name: String,
    pub to_friend_code: String,
    pub status: String, // "pending" | "accepted" | "rejected" | "revoked"
    pub created_at: String,
}

pub async fn load_invites(data_dir: &str) -> HashMap<String, AllianceInvite> {
    load_domain_doc(data_dir, "alliance_invites", "all")
        .await
        .unwrap_or_default()
}

pub async fn save_invites(
    data_dir: &str,
    invites: &HashMap<String, AllianceInvite>,
) -> std::io::Result<()> {
    save_domain_doc(data_dir, "alliance_invites", "all", invites).await
}

fn generate_invite_id() -> String {
    use rand::Rng;
    format!("inv_{}", rand::thread_rng().gen::<u32>())
}

/// Load accepted invites for a user (alliances they can edit)
pub async fn load_invites_for_user(state: &AppState, session_account: &str) -> Vec<AllianceInvite> {
    let invites = load_invites(&state.data_dir).await;
    let my_friend_code = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(session_account)
            .and_then(|a| a.friend_code.as_ref())
            .cloned()
            .unwrap_or_default()
    };
    invites
        .values()
        .filter(|i| {
            i.to_friend_code.eq_ignore_ascii_case(&my_friend_code) && i.status == "accepted"
        })
        .cloned()
        .collect()
}

/// GET /{account}/{server}/api/friend-code - Get or create my friend code
pub async fn get_friend_code(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };
    let session_server: u32 = match session.get("server_number") {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let (friend_code, snapshot_to_save) = {
        let mut accounts = state.accounts.lock().unwrap();
        let need_save = match accounts.get_mut(&session_account) {
            Some(a) => {
                if a.friend_code.is_none() {
                    a.friend_code = Some(generate_friend_code());
                    true
                } else {
                    false
                }
            }
            None => {
                return Ok(HttpResponse::NotFound()
                    .json(serde_json::json!({"error": "Account not found"})))
            }
        };
        let fc = accounts
            .get(&session_account)
            .and_then(|a| a.friend_code.clone())
            .unwrap_or_default();
        let snap = if need_save {
            Some(accounts.clone())
        } else {
            None
        };
        (fc, snap)
    };
    if let Some(snap) = snapshot_to_save {
        save_accounts(&state.data_dir, &snap).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "friend_code": friend_code
    })))
}

#[derive(Deserialize)]
pub struct InviteByFriendCodeRequest {
    pub friend_code: String,
}

/// POST /{account}/{server}/api/alliance-invites - Invite someone by friend code
pub async fn create_invite(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<InviteByFriendCodeRequest>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };
    let session_server: u32 = match session.get("server_number") {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let to_friend_code = body.friend_code.trim().replace(' ', "");
    if to_friend_code.len() != 12 || !to_friend_code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Friend code must be 12 letters and numbers"
        })));
    }

    let (alliance_slug, alliance_name, my_friend_code) = {
        let accounts = state.accounts.lock().unwrap();
        let acc = match accounts.get(&session_account) {
            Some(a) => a,
            None => {
                return Ok(HttpResponse::NotFound()
                    .json(serde_json::json!({"error": "Account not found"})))
            }
        };
        if !acc.alliance_access {
            return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                "success": false,
                "error": "Alliance access required"
            })));
        }
        let alliance_name = acc.alliance_name.as_deref().unwrap_or("").to_string();
        if alliance_name.is_empty() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "No alliance assigned"
            })));
        }
        let slug = super::alliances::alliance_to_slug(&alliance_name);
        let my_fc = acc.friend_code.as_deref().unwrap_or("").to_string();
        (slug, alliance_name, my_fc)
    };

    if to_friend_code.eq_ignore_ascii_case(&my_friend_code) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Cannot invite yourself"
        })));
    }

    let invited_account = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .values()
            .find(|a| {
                a.friend_code
                    .as_deref()
                    .map(|fc| fc.eq_ignore_ascii_case(&to_friend_code))
                    .unwrap_or(false)
            })
            .map(|a| a.account_name.clone())
    };

    if invited_account.is_none() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No account found with that friend code"
        })));
    }

    let mut invites = load_invites(&state.data_dir).await;
    let has_pending = invites.values().any(|i| {
        i.from_account == session_account
            && i.from_server == server
            && i.alliance_slug == alliance_slug
            && i.to_friend_code.eq_ignore_ascii_case(&to_friend_code)
            && i.status == "pending"
    });
    if has_pending {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invite already sent to this user"
        })));
    }

    let id = generate_invite_id();
    let invite = AllianceInvite {
        id: id.clone(),
        from_account: session_account.clone(),
        from_server: server,
        alliance_slug: alliance_slug.clone(),
        alliance_name: alliance_name.clone(),
        to_friend_code: to_friend_code.clone(),
        status: "pending".to_string(),
        created_at: chrono::Local::now().to_rfc3339(),
    };
    invites.insert(id, invite);
    save_invites(&state.data_dir, &invites).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Invite sent"
    })))
}

/// GET /{account}/{server}/api/alliance-invites - List invites (sent + received)
pub async fn list_invites(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };
    let session_server: u32 = match session.get("server_number") {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let my_friend_code = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(&session_account)
            .and_then(|a| a.friend_code.as_ref())
            .cloned()
            .unwrap_or_default()
    };

    let invites = load_invites(&state.data_dir).await;
    let accounts = state.accounts.lock().unwrap();
    let sent: Vec<_> = invites
        .values()
        .filter(|i| i.from_account == session_account && i.from_server == server)
        .map(|i| {
            let to_account = accounts
                .values()
                .find(|a| {
                    a.friend_code
                        .as_deref()
                        .map(|fc| fc.eq_ignore_ascii_case(&i.to_friend_code))
                        .unwrap_or(false)
                })
                .map(|a| a.account_name.clone())
                .unwrap_or_else(|| "?".to_string());
            serde_json::json!({
                "id": i.id,
                "type": "sent",
                "to_friend_code": i.to_friend_code,
                "to_account": to_account,
                "alliance_name": i.alliance_name,
                "status": i.status,
                "created_at": i.created_at
            })
        })
        .collect();
    drop(accounts);
    let received: Vec<_> = invites
        .values()
        .filter(|i| i.to_friend_code.eq_ignore_ascii_case(&my_friend_code) && i.status == "pending")
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "type": "received",
                "from_account": i.from_account,
                "alliance_name": i.alliance_name,
                "status": i.status,
                "created_at": i.created_at
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "sent": sent,
        "received": received
    })))
}

/// POST /{account}/{server}/api/alliance-invites/{id}/accept
pub async fn accept_invite(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server, id) = path.into_inner();
    let _ = (url_account, server);

    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };
    let my_friend_code = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(&session_account)
            .and_then(|a| a.friend_code.as_ref())
            .cloned()
            .unwrap_or_default()
    };

    let mut invites = load_invites(&state.data_dir).await;
    let invite = match invites.get_mut(&id) {
        Some(i) => i,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Invite not found"
            })))
        }
    };

    if invite.status != "pending" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invite is no longer pending"
        })));
    }
    if !invite.to_friend_code.eq_ignore_ascii_case(&my_friend_code) {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "This invite was sent to a different user"
        })));
    }

    invite.status = "accepted".to_string();
    save_invites(&state.data_dir, &invites).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Invite accepted"
    })))
}

/// POST /{account}/{server}/api/alliance-invites/{id}/reject
pub async fn reject_invite(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server, id) = path.into_inner();
    let _ = (url_account, server);

    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };
    let my_friend_code = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(&session_account)
            .and_then(|a| a.friend_code.as_ref())
            .cloned()
            .unwrap_or_default()
    };

    let mut invites = load_invites(&state.data_dir).await;
    let invite = match invites.get_mut(&id) {
        Some(i) => i,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Invite not found"
            })))
        }
    };

    if invite.status != "pending" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invite is no longer pending"
        })));
    }
    if !invite.to_friend_code.eq_ignore_ascii_case(&my_friend_code) {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "This invite was sent to a different user"
        })));
    }

    invite.status = "rejected".to_string();
    save_invites(&state.data_dir, &invites).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Invite rejected"
    })))
}

/// POST /{account}/{server}/api/alliance-invites/{id}/revoke - Revoke admin access (super admin only)
pub async fn revoke_invite(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server, id) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    let session_server: u32 = match session.get("server_number") {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Ok(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let mut invites = load_invites(&state.data_dir).await;
    let invite = match invites.get_mut(&id) {
        Some(i) => i,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Invite not found"
            })))
        }
    };

    if invite.status != "accepted" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Can only revoke accepted invites"
        })));
    }

    if !invite.from_account.eq_ignore_ascii_case(&session_account)
        || invite.from_server != session_server
    {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Only the alliance owner (super admin) can remove admins"
        })));
    }

    invite.status = "revoked".to_string();
    save_invites(&state.data_dir, &invites).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Admin removed"
    })))
}

/// Check if session_account has access to edit (owner_account, owner_server, alliance_slug)
pub async fn has_alliance_access(
    state: &AppState,
    session_account: &str,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
) -> bool {
    if session_account.eq_ignore_ascii_case(owner_account) {
        return true;
    }
    let invites = load_invites(&state.data_dir).await;
    let my_friend_code = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(session_account)
            .and_then(|a| a.friend_code.as_ref())
            .cloned()
            .unwrap_or_default()
    };
    invites.values().any(|i| {
        i.from_account.eq_ignore_ascii_case(owner_account)
            && i.from_server == owner_server
            && i.alliance_slug == alliance_slug
            && i.to_friend_code.eq_ignore_ascii_case(&my_friend_code)
            && i.status == "accepted"
    })
}
