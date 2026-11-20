//! Alliance applications grant access automatically after validation.

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

async fn load_applications(data_dir: &str) -> HashMap<String, AllianceApplication> {
    load_domain_doc(data_dir, "alliance_applications", "all")
        .await
        .unwrap_or_default()
}

async fn save_applications(
    data_dir: &str,
    apps: &HashMap<String, AllianceApplication>,
) -> std::io::Result<()> {
    save_domain_doc(data_dir, "alliance_applications", "all", apps).await
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

async fn grant_alliance_access(state: &AppState, app: &AllianceApplication) -> Result<()> {
    let snapshot = {
        let mut accounts = state.accounts.lock().unwrap();
        let account = accounts
            .get_mut(&app.account_name)
            .ok_or_else(|| actix_web::error::ErrorNotFound("Account not found"))?;
        account.alliance_access = true;
        account.alliance_id.get_or_insert_with(generate_alliance_id);
        account.alliance_tag = Some(app.alliance_tag.clone());
        account.alliance_name = Some(app.alliance_name.clone());
        accounts.clone()
    };
    save_accounts(&state.data_dir, &snapshot)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save accounts: {}", e))
        })?;
    Ok(())
}

/// Approve applications left pending before automatic approval was enabled.
pub async fn approve_pending_applications(state: &AppState) -> std::io::Result<()> {
    let mut apps = load_applications(&state.data_dir).await;
    let mut changed = false;
    for app in apps.values_mut().filter(|app| app.status == "pending") {
        if !state
            .accounts
            .lock()
            .unwrap()
            .contains_key(&app.account_name)
        {
            continue;
        }
        grant_alliance_access(state, app)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        app.status = "approved".to_string();
        changed = true;
    }
    if changed {
        save_applications(&state.data_dir, &apps).await?;
    }
    Ok(())
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

    let mut apps = load_applications(&state.data_dir).await;
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
        status: "approved".to_string(),
        submitted_at: chrono::Local::now().to_rfc3339(),
    };
    grant_alliance_access(&state, &app).await?;
    apps.insert(id, app);
    save_applications(&state.data_dir, &apps)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Application approved",
        "status": "approved"
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

    let apps = load_applications(&state.data_dir).await;
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

    let apps = load_applications(&state.data_dir).await;
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
    let mut apps = load_applications(&state.data_dir).await;

    let app = apps
        .get_mut(&id)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Application not found"))?;
    if app.status != "pending" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Application is not pending"
        })));
    }
    grant_alliance_access(&state, app).await?;
    app.status = "approved".to_string();
    save_applications(&state.data_dir, &apps)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

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
    let mut apps = load_applications(&state.data_dir).await;

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
    save_applications(&state.data_dir, &apps)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Application rejected"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_session::{storage::CookieSessionStore, SessionMiddleware};
    use actix_web::{cookie::Key, test, App};
    use std::sync::Mutex;

    fn fixture() -> web::Data<AppState> {
        let dir = std::env::temp_dir().join(format!("alliance-approval-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let account = serde_json::from_value(serde_json::json!({
            "account_name": "applicant", "server_number": 1, "in_game_name": "Applicant"
        }))
        .unwrap();
        web::Data::new(AppState {
            accounts: Mutex::new(HashMap::from([("applicant".to_string(), account)])),
            schedules: Mutex::new(HashMap::new()),
            forms: Mutex::new(HashMap::new()),
            current_forms: Mutex::new(HashMap::new()),
            data_dir: dir.to_string_lossy().into_owned(),
            pg: None,
            oauth_hmac_key: vec![0; 32],
        })
    }

    async fn login(session: Session) -> HttpResponse {
        session.insert("account_name", "applicant").unwrap();
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn submission_grants_and_persists_access_immediately() {
        let state = fixture();
        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .wrap(SessionMiddleware::new(
                    CookieSessionStore::default(),
                    Key::generate(),
                ))
                .route("/login", web::get().to(login))
                .route("/apply", web::post().to(submit_application)),
        )
        .await;
        let body = serde_json::json!({
            "alliance_tag": " TEST ", "alliance_name": " Test Alliance ",
            "contact_player_id": "12345", "server_number": 1
        });
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/apply")
                .set_json(&body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), 401);
        let response =
            test::call_service(&app, test::TestRequest::get().uri("/login").to_request()).await;
        let cookie = response.response().cookies().next().unwrap().into_owned();
        let mut invalid = body.clone();
        invalid["contact_player_id"] = "invalid".into();
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/apply")
                .cookie(cookie.clone())
                .set_json(&invalid)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), 400);
        assert!(!state.accounts.lock().unwrap()["applicant"].alliance_access);
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/apply")
                .cookie(cookie.clone())
                .set_json(&body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), 200);
        let apps = load_applications(&state.data_dir).await;
        assert_eq!(apps.len(), 1);
        assert_eq!(apps.values().next().unwrap().status, "approved");
        let accounts = super::super::persistence::load_accounts(&state.data_dir).await;
        let account = &accounts["applicant"];
        assert!(account.alliance_access);
        assert_eq!(account.alliance_tag.as_deref(), Some("TEST"));
        assert_eq!(account.alliance_name.as_deref(), Some("Test Alliance"));
        assert_eq!(account.alliance_id.as_ref().unwrap().len(), 6);
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/apply")
                .cookie(cookie)
                .set_json(&body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), 400);
        std::fs::remove_dir_all(&state.data_dir).unwrap();
    }

    #[actix_web::test]
    async fn existing_pending_applications_are_approved_once() {
        let state = fixture();
        let application = AllianceApplication {
            id: "pending".into(),
            account_name: "applicant".into(),
            alliance_tag: "TEST".into(),
            alliance_name: "Test Alliance".into(),
            contact_player_id: "12345".into(),
            server_number: 1,
            status: "pending".into(),
            submitted_at: "2026-01-01".into(),
        };
        save_applications(
            &state.data_dir,
            &HashMap::from([("pending".into(), application)]),
        )
        .await
        .unwrap();
        approve_pending_applications(&state).await.unwrap();
        let alliance_id = state.accounts.lock().unwrap()["applicant"]
            .alliance_id
            .clone();
        assert!(state.accounts.lock().unwrap()["applicant"].alliance_access);
        assert_eq!(
            load_applications(&state.data_dir).await["pending"].status,
            "approved"
        );
        approve_pending_applications(&state).await.unwrap();
        assert_eq!(
            state.accounts.lock().unwrap()["applicant"].alliance_id,
            alliance_id
        );
        std::fs::remove_dir_all(&state.data_dir).unwrap();
    }
}
