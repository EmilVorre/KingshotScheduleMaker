//! Swordland: two legions with member assignment and attendance tracking.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use super::alliance_invites;
use super::persistence::{load_domain_doc, save_domain_doc};
use super::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Legion {
    pub name: String,
    #[serde(default)]
    pub member_ids: Vec<String>,
    #[serde(default)]
    pub filler_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegionAttendance {
    pub attended: Vec<String>,
    pub absent: Vec<String>,
    #[serde(default)]
    pub filler: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: String,
    pub date: String,
    #[serde(default)]
    pub label: Option<String>,
    pub legion_1: LegionAttendance,
    pub legion_2: LegionAttendance,
}

#[derive(Debug, Serialize, Deserialize)]
struct SwordlandFile {
    #[serde(default = "default_legions")]
    legions: Vec<Legion>,
    #[serde(default)]
    attendance_records: Vec<AttendanceRecord>,
}

fn default_legions() -> Vec<Legion> {
    vec![
        Legion {
            name: "Legion 1".to_string(),
            member_ids: vec![],
            filler_ids: vec![],
        },
        Legion {
            name: "Legion 2".to_string(),
            member_ids: vec![],
            filler_ids: vec![],
        },
    ]
}

fn swordland_doc_key(owner_account: &str, owner_server: u32, alliance_slug: &str) -> String {
    format!(
        "{}_{}_{}",
        owner_account.to_lowercase(),
        owner_server,
        alliance_slug
    )
}

async fn load_swordland(
    data_dir: &str,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
) -> SwordlandFile {
    let key = swordland_doc_key(owner_account, owner_server, alliance_slug);
    if let Some(mut data) = load_domain_doc::<SwordlandFile>(data_dir, "swordland", &key).await {
        if data.legions.len() < 2 {
            data.legions = default_legions();
        }
        return data;
    }
    SwordlandFile {
        legions: default_legions(),
        attendance_records: vec![],
    }
}

async fn save_swordland(
    data_dir: &str,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
    data: &SwordlandFile,
) -> std::io::Result<()> {
    let key = swordland_doc_key(owner_account, owner_server, alliance_slug);
    save_domain_doc(data_dir, "swordland", &key, data).await
}

/// Auth: session user must have access to edit this alliance (owner or invited)
async fn auth_check_alliance(
    session: &Session,
    state: &web::Data<AppState>,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
) -> Result<(String, u32), HttpResponse> {
    let session_account: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Err(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Err(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };
    let _session_server: u32 = match session.get("server_number") {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err(
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Not logged in"}))
            )
        }
        Err(_) => {
            return Err(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Session error"})))
        }
    };

    if !alliance_invites::has_alliance_access(
        state.get_ref(),
        &session_account,
        owner_account,
        owner_server,
        alliance_slug,
    )
    .await
    {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Alliance access required"
        })));
    }

    Ok((session_account, owner_server))
}

/// GET /{account}/{server}/api/alliances/{slug}/swordland - Get legions and attendance records (alliance-scoped)
pub async fn get_swordland(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (owner_account, owner_server, alliance_slug) = path.into_inner();
    let owner_account = owner_account.to_lowercase();

    let (_session_account, _) = match auth_check_alliance(
        &session,
        &state,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let data = load_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "legions": data.legions,
        "attendance_records": data.attendance_records
    })))
}

#[derive(Deserialize)]
pub struct SetLegionsRequest {
    pub legions: Vec<Legion>,
}

/// PUT /{account}/{server}/api/alliances/{slug}/swordland - Save legion member assignments (alliance-scoped)
pub async fn set_swordland(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<SetLegionsRequest>,
) -> Result<HttpResponse> {
    let (owner_account, owner_server, alliance_slug) = path.into_inner();
    let owner_account = owner_account.to_lowercase();

    let (_session_account, _) = match auth_check_alliance(
        &session,
        &state,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let mut legions = body.legions.clone();
    if legions.len() < 2 {
        legions = default_legions();
    }
    for legion in &mut legions {
        legion
            .member_ids
            .retain(|id| id.chars().all(|c| c.is_ascii_digit()));
        legion
            .filler_ids
            .retain(|id| id.chars().all(|c| c.is_ascii_digit()));
    }
    // Enforce one legion per player: if in both member_ids and filler_ids, keep only in first
    let mut seen = std::collections::HashSet::new();
    for legion in &mut legions {
        legion.member_ids.retain(|id| seen.insert(id.clone()));
        legion.filler_ids.retain(|id| seen.insert(id.clone()));
    }

    let mut data = load_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await;
    data.legions = legions;

    save_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
        &data,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "legions": data.legions
    })))
}

#[derive(Deserialize)]
pub struct AddAttendanceRequest {
    pub date: String,
    #[serde(default)]
    pub label: Option<String>,
    pub legion_1_attended: Vec<String>,
    pub legion_1_absent: Vec<String>,
    #[serde(default)]
    pub legion_1_filler: Vec<String>,
    pub legion_2_attended: Vec<String>,
    pub legion_2_absent: Vec<String>,
    #[serde(default)]
    pub legion_2_filler: Vec<String>,
}

/// POST /{account}/{server}/api/alliances/{slug}/swordland/attendance - Add attendance record (alliance-scoped)
pub async fn add_attendance(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<AddAttendanceRequest>,
) -> Result<HttpResponse> {
    let (owner_account, owner_server, alliance_slug) = path.into_inner();
    let owner_account = owner_account.to_lowercase();

    let (_session_account, _) = match auth_check_alliance(
        &session,
        &state,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let record = AttendanceRecord {
        id: id.clone(),
        date: body.date.trim().to_string(),
        label: body
            .label
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned(),
        legion_1: LegionAttendance {
            attended: body.legion_1_attended.clone(),
            absent: body.legion_1_absent.clone(),
            filler: body.legion_1_filler.clone(),
        },
        legion_2: LegionAttendance {
            attended: body.legion_2_attended.clone(),
            absent: body.legion_2_absent.clone(),
            filler: body.legion_2_filler.clone(),
        },
    };

    let mut data = load_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await;
    data.attendance_records.push(record.clone());

    save_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
        &data,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "record": record
    })))
}

/// PUT /{account}/{server}/api/alliances/{slug}/swordland/attendance/{id} - Update attendance record (alliance-scoped)
pub async fn update_attendance(
    path: web::Path<(String, u32, String, String)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<AddAttendanceRequest>,
) -> Result<HttpResponse> {
    let (owner_account, owner_server, alliance_slug, record_id) = path.into_inner();
    let owner_account = owner_account.to_lowercase();

    let (_session_account, _) = match auth_check_alliance(
        &session,
        &state,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let mut data = load_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    )
    .await;
    let idx = data
        .attendance_records
        .iter()
        .position(|r| r.id == record_id)
        .ok_or_else(|| {
            actix_web::error::ErrorNotFound(format!("Attendance record {} not found", record_id))
        })?;

    let record = AttendanceRecord {
        id: record_id,
        date: body.date.trim().to_string(),
        label: body
            .label
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned(),
        legion_1: LegionAttendance {
            attended: body.legion_1_attended.clone(),
            absent: body.legion_1_absent.clone(),
            filler: body.legion_1_filler.clone(),
        },
        legion_2: LegionAttendance {
            attended: body.legion_2_attended.clone(),
            absent: body.legion_2_absent.clone(),
            filler: body.legion_2_filler.clone(),
        },
    };

    data.attendance_records[idx] = record.clone();

    save_swordland(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
        &data,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "record": record
    })))
}
