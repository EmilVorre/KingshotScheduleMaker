//! Tri Alliance: two legions with member assignment and attendance tracking.
//! Same structure as Swordland but for a different event.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::alliance_invites;
use super::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Legion {
    pub name: String,
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegionAttendance {
    pub attended: Vec<String>,
    pub absent: Vec<String>,
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
struct TriAllianceFile {
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
        },
        Legion {
            name: "Legion 2".to_string(),
            member_ids: vec![],
        },
    ]
}

fn tri_alliance_path(
    data_dir: &str,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
) -> std::path::PathBuf {
    Path::new(data_dir).join("tri_alliance").join(format!(
        "{}_{}_{}.json",
        owner_account.to_lowercase(),
        owner_server,
        alliance_slug
    ))
}

fn load_tri_alliance(
    data_dir: &str,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
) -> TriAllianceFile {
    let path = tri_alliance_path(data_dir, owner_account, owner_server, alliance_slug);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<TriAllianceFile>(&content) {
                let mut data = data;
                if data.legions.len() < 2 {
                    data.legions = default_legions();
                }
                return data;
            }
        }
    }
    TriAllianceFile {
        legions: default_legions(),
        attendance_records: vec![],
    }
}

fn save_tri_alliance(
    data_dir: &str,
    owner_account: &str,
    owner_server: u32,
    alliance_slug: &str,
    data: &TriAllianceFile,
) -> std::io::Result<()> {
    let path = tri_alliance_path(data_dir, owner_account, owner_server, alliance_slug);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(data)?;
    fs::write(path, content)
}

fn auth_check_alliance(
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
    ) {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Alliance access required"
        })));
    }

    Ok((session_account, owner_server))
}

/// GET /{account}/{server}/api/alliances/{slug}/tri-alliance - Get legions and attendance records (alliance-scoped)
pub async fn get_tri_alliance(
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
    ) {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let data = load_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    );

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

/// PUT /{account}/{server}/api/alliances/{slug}/tri-alliance - Save legion member assignments (alliance-scoped)
pub async fn set_tri_alliance(
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
    ) {
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
    }
    let mut seen = std::collections::HashSet::new();
    for legion in &mut legions {
        legion.member_ids.retain(|id| seen.insert(id.clone()));
    }

    let mut data = load_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    );
    data.legions = legions;

    save_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
        &data,
    )
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
    pub legion_2_attended: Vec<String>,
    pub legion_2_absent: Vec<String>,
}

/// POST /{account}/{server}/api/alliances/{slug}/tri-alliance/attendance - Add attendance record (alliance-scoped)
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
    ) {
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
        },
        legion_2: LegionAttendance {
            attended: body.legion_2_attended.clone(),
            absent: body.legion_2_absent.clone(),
        },
    };

    let mut data = load_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    );
    data.attendance_records.push(record.clone());

    save_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
        &data,
    )
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "record": record
    })))
}

/// PUT /{account}/{server}/api/alliances/{slug}/tri-alliance/attendance/{id} - Update attendance record (alliance-scoped)
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
    ) {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let mut data = load_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
    );
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
        },
        legion_2: LegionAttendance {
            attended: body.legion_2_attended.clone(),
            absent: body.legion_2_absent.clone(),
        },
    };

    data.attendance_records[idx] = record.clone();

    save_tri_alliance(
        &state.data_dir,
        &owner_account,
        owner_server,
        &alliance_slug,
        &data,
    )
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "record": record
    })))
}
