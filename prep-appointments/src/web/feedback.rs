//! Feedback API: submit feedback (public), list feedback (admin only).

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};

use super::persistence::{load_feedback, save_feedback, FeedbackEntry};
use super::state::AppState;

/// Require admin session. Returns account_name if admin, Err otherwise.
fn require_admin(session: &Session, state: &web::Data<AppState>) -> Result<String, HttpResponse> {
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

#[derive(serde::Deserialize)]
pub struct SubmitFeedbackRequest {
    #[serde(rename = "type")]
    pub feedback_type: String,
    pub text: String,
}

/// POST /api/feedback - Submit feedback (public)
pub async fn submit_feedback(
    state: web::Data<AppState>,
    req: web::Json<SubmitFeedbackRequest>,
) -> Result<HttpResponse> {
    let feedback_type = req.feedback_type.as_str();
    if !["bug", "feature", "general"].contains(&feedback_type) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid type: must be bug, feature, or general"
        })));
    }

    let text = req.text.trim();
    if text.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Feedback text is required"
        })));
    }

    let id = super::persistence::generate_form_code();
    let created_at = chrono::Utc::now().to_rfc3339();

    let mut feedback = load_feedback(&state.data_dir).await;
    feedback.push(FeedbackEntry {
        id: id.clone(),
        r#type: feedback_type.to_string(),
        text: text.to_string(),
        created_at: created_at.clone(),
        archived: false,
    });

    if let Err(e) = save_feedback(&state.data_dir, &feedback).await {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save: {}", e)
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "id": id
    })))
}

/// GET /api/admin/feedback - List all feedback (admin only)
pub async fn list_feedback(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let feedback = load_feedback(&state.data_dir).await;
    let list: Vec<serde_json::Value> = feedback
        .iter()
        .filter(|f| !f.archived)
        .rev()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "type": f.r#type,
                "text": f.text,
                "created_at": f.created_at
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "feedback": list
    })))
}

/// POST /api/admin/feedback/{id}/archive - Archive feedback (admin only)
pub async fn archive_feedback(
    path: web::Path<String>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let _admin = match require_admin(&session, &state) {
        Ok(n) => n,
        Err(resp) => return Ok(resp),
    };

    let id = path.into_inner();
    let mut feedback = load_feedback(&state.data_dir).await;

    if let Some(f) = feedback.iter_mut().find(|e| e.id == id) {
        f.archived = true;
    } else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Feedback not found"
        })));
    }

    if let Err(e) = save_feedback(&state.data_dir, &feedback).await {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save: {}", e)
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}
