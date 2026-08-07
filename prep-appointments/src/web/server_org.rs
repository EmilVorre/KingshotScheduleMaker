//! Server Organisation: named workspaces per Kingshot server #, Tyrant form, co-admins via friend-code invites.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::kingshot_api;

use super::persistence::{
    self, generate_form_code, is_postgres_backend, load_domain_doc, save_domain_doc,
};
use super::state::AppState;

static ORG_JSON_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
struct OrgBundle {
    workspaces: Vec<WorkspaceRecord>,
    members: Vec<MemberRecord>,
    invites: Vec<InviteRecord>,
    tyrant_forms: Vec<TyrantFormRecord>,
    tyrant_submissions: Vec<TyrantSubmissionRecord>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct WorkspaceRecord {
    id: String,
    display_name: String,
    kingshot_server_number: i32,
    owner_account_key: String,
    created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct MemberRecord {
    workspace_id: String,
    account_key: String,
    role: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct InviteRecord {
    id: String,
    workspace_id: String,
    from_account_key: String,
    to_friend_code: String,
    status: String,
    created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct TyrantFormRecord {
    id: String,
    workspace_id: String,
    public_code: String,
    #[serde(default)]
    config: Value,
    created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct TyrantSubmissionRecord {
    id: i64,
    workspace_id: String,
    form_id: Option<String>,
    public_code: String,
    player_id: String,
    payload: Value,
    created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TyrantTroopBands {
    pub level_band: String,
    pub tg_band: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TyrantSubmissionPayload {
    pub alliance: String,
    pub archer: TyrantTroopBands,
    pub cavalry: TyrantTroopBands,
    pub infantry: TyrantTroopBands,
    #[serde(default)]
    pub utc_slots: Vec<String>,
    #[serde(default)]
    pub participate_full_five_hours: bool,
    /// In-game name from lookup at submit time (older rows omit this).
    #[serde(default)]
    pub player_name: Option<String>,
    /// Whether the player expects auto-help and/or monthly card to be active for Tyrant.
    #[serde(default)]
    pub auto_help_month_card_active: Option<bool>,
}

fn valid_band_level(s: &str) -> bool {
    matches!(s, "level_1_9" | "level_10" | "level_11")
}

fn valid_band_tg(s: &str) -> bool {
    matches!(s, "below_tg5" | "tg5" | "tg6" | "tg7" | "tg8")
}

async fn bundle_load(state: &AppState) -> OrgBundle {
    if is_postgres_backend() {
        return OrgBundle::default();
    }
    load_domain_doc::<OrgBundle>(&state.data_dir, "server_org", "bundle")
        .await
        .unwrap_or_default()
}

async fn bundle_save(state: &AppState, b: &OrgBundle) -> std::io::Result<()> {
    save_domain_doc(&state.data_dir, "server_org", "bundle", b).await
}

/// Latest Tyrant form public code for a workspace (same rule as `ensure_tyrant_form`).
fn latest_tyrant_public_code(bundle: &OrgBundle, workspace_id: &str) -> Option<String> {
    bundle
        .tyrant_forms
        .iter()
        .filter(|f| f.workspace_id == workspace_id)
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
        .map(|f| f.public_code.clone())
}

fn next_submission_id(bundle: &OrgBundle) -> i64 {
    bundle
        .tyrant_submissions
        .iter()
        .map(|s| s.id)
        .max()
        .unwrap_or(0)
        + 1
}

/// True if account is member of any workspace (for session flag).
pub async fn account_has_any_server_org(state: &AppState, account_key: &str) -> bool {
    let key = account_key.to_lowercase();
    if is_postgres_backend() {
        return pg_member_any(state, &key).await.unwrap_or(false);
    }
    let b = bundle_load(state).await;
    b.members
        .iter()
        .any(|m| m.account_key.to_lowercase() == key)
}

async fn pg_member_any(
    _state: &AppState,
    account_key: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let pool = persistence::pg_pool().ok_or("no pool")?;
    let client = pool.client().await?;
    let row = client
        .query_opt(
            "SELECT 1 FROM server_workspace_members WHERE lower(account_key) = lower($1) LIMIT 1",
            &[&account_key],
        )
        .await?;
    Ok(row.is_some())
}

async fn workspace_access(state: &AppState, workspace_id: &str, account: &str) -> bool {
    let ac = account.to_lowercase();
    if is_postgres_backend() {
        return pg_has_access(state, workspace_id, &ac)
            .await
            .unwrap_or(false);
    }
    let b = bundle_load(state).await;
    b.members
        .iter()
        .any(|m| m.workspace_id == workspace_id && m.account_key.to_lowercase() == ac)
}

async fn pg_has_access(
    _state: &AppState,
    workspace_id: &str,
    account_key: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let pool = persistence::pg_pool().ok_or("no pool")?;
    let client = pool.client().await?;
    let row = client
        .query_opt(
            "SELECT 1 FROM server_workspace_members WHERE workspace_id = $1 AND lower(account_key) = lower($2)",
            &[&workspace_id, &account_key],
        )
        .await?;
    Ok(row.is_some())
}

#[derive(Deserialize)]
pub struct CreateWorkspaceBody {
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct InviteBody {
    pub friend_code: String,
}

/// POST .../api/server-org/workspaces
pub async fn create_workspace(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<CreateWorkspaceBody>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    let name = body.display_name.trim();
    if name.is_empty() || name.len() > 200 {
        return Ok(HttpResponse::BadRequest()
            .json(json!({"success": false, "error": "Invalid display name"})));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    if is_postgres_backend() {
        let pool = match persistence::pg_pool() {
            Some(p) => p,
            None => {
                return Ok(HttpResponse::InternalServerError()
                    .json(json!({"success": false, "error": "Database not configured"})))
            }
        };
        let client = pool
            .client()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("db: {e}")))?;
        let srv = server as i32;
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] =
            &[&id, &name, &srv, &session_account];
        client
            .execute(
                "INSERT INTO server_workspaces (id, display_name, kingshot_server_number, owner_account_key) VALUES ($1, $2, $3, $4)",
                params,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        let mparams: &[&(dyn tokio_postgres::types::ToSql + Sync)] =
            &[&id, &session_account, &"owner".to_string()];
        client
            .execute(
                "INSERT INTO server_workspace_members (workspace_id, account_key, role) VALUES ($1, $2, $3)",
                mparams,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
    } else {
        let _guard = ORG_JSON_LOCK.lock().await;
        let mut b = bundle_load(state.get_ref()).await;
        b.workspaces.push(WorkspaceRecord {
            id: id.clone(),
            display_name: name.to_string(),
            kingshot_server_number: server as i32,
            owner_account_key: session_account.clone(),
            created_at: now.clone(),
        });
        b.members.push(MemberRecord {
            workspace_id: id.clone(),
            account_key: session_account.clone(),
            role: "owner".to_string(),
        });
        bundle_save(state.get_ref(), &b)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    }

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "workspace_id": id,
    })))
}

/// GET .../api/server-org/workspaces
pub async fn list_my_workspaces(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    let list = if is_postgres_backend() {
        pg_list_workspaces_for_account(state.get_ref(), &session_account)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
    } else {
        let b = bundle_load(state.get_ref()).await;
        let keys: std::collections::HashSet<String> = b
            .members
            .iter()
            .filter(|m| m.account_key.to_lowercase() == session_account.to_lowercase())
            .map(|m| m.workspace_id.clone())
            .collect();
        b.workspaces
            .iter()
            .filter(|w| keys.contains(&w.id))
            .map(|w| {
                let tyrant_public_code = latest_tyrant_public_code(&b, &w.id);
                json!({
                    "id": w.id,
                    "display_name": w.display_name,
                    "kingshot_server_number": w.kingshot_server_number,
                    "owner_account_key": w.owner_account_key,
                    "created_at": w.created_at,
                    "tyrant_public_code": tyrant_public_code,
                })
            })
            .collect::<Vec<_>>()
    };

    Ok(HttpResponse::Ok().json(json!({ "success": true, "workspaces": list })))
}

async fn pg_list_workspaces_for_account(
    _state: &AppState,
    account: &str,
) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
    let pool = persistence::pg_pool().ok_or("no pool")?;
    let client = pool.client().await?;
    let rows = client
        .query(
            "SELECT w.id, w.display_name, w.kingshot_server_number, w.owner_account_key, w.created_at::text, \
                    tf.public_code \
             FROM server_workspaces w \
             INNER JOIN server_workspace_members m ON m.workspace_id = w.id \
             LEFT JOIN LATERAL ( \
                 SELECT public_code FROM tyrant_forms \
                 WHERE workspace_id = w.id \
                 ORDER BY created_at DESC \
                 LIMIT 1 \
             ) tf ON true \
             WHERE lower(m.account_key) = lower($1) \
             ORDER BY w.display_name ASC",
            &[&account],
        )
        .await?;
    let mut out = Vec::new();
    for r in rows {
        let code: Option<String> = r.get(5);
        out.push(json!({
            "id": r.get::<_, String>(0),
            "display_name": r.get::<_, String>(1),
            "kingshot_server_number": r.get::<_, i32>(2),
            "owner_account_key": r.get::<_, String>(3),
            "created_at": r.get::<_, String>(4),
            "tyrant_public_code": code,
        }));
    }
    Ok(out)
}

/// POST .../api/server-org/workspaces/{workspace_id}/invites
pub async fn create_workspace_invite(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<InviteBody>,
) -> Result<HttpResponse> {
    let (url_account, server, workspace_id) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    if !workspace_access(state.get_ref(), &workspace_id, &session_account).await {
        return Ok(HttpResponse::Forbidden().json(json!({"success": false, "error": "No access"})));
    }

    let to_friend_code = body.friend_code.trim().replace(' ', "");
    if to_friend_code.len() != 12 || !to_friend_code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "Friend code must be 12 letters and numbers"
        })));
    }

    let my_fc = session_friend_code(state.get_ref(), &session_account);
    if to_friend_code.eq_ignore_ascii_case(&my_fc) {
        return Ok(HttpResponse::BadRequest()
            .json(json!({"success": false, "error": "Cannot invite yourself"})));
    }

    let invited_exists = resolve_friend_code_account(state.get_ref(), &to_friend_code).is_some();
    if !invited_exists {
        return Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "No account found with that friend code"
        })));
    }

    let id = format!("srvinv_{}", rand::thread_rng().gen::<u32>());
    let now = chrono::Utc::now().to_rfc3339();

    if is_postgres_backend() {
        let pool = persistence::pg_pool()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("no pool"))?;
        let client = pool
            .client()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("db: {e}")))?;
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[
            &id,
            &workspace_id,
            &session_account,
            &to_friend_code,
            &"pending".to_string(),
        ];
        let dup = client
            .query_opt(
                "SELECT 1 FROM server_workspace_invites WHERE workspace_id = $1 AND lower(from_account_key) = lower($2) AND lower(to_friend_code) = lower($3) AND status = 'pending'",
                &[&workspace_id, &session_account, &to_friend_code],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        if dup.is_some() {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Invite already sent"
            })));
        }
        client
            .execute(
                "INSERT INTO server_workspace_invites (id, workspace_id, from_account_key, to_friend_code, status) VALUES ($1, $2, $3, $4, $5)",
                params,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
    } else {
        let _guard = ORG_JSON_LOCK.lock().await;
        let mut b = bundle_load(state.get_ref()).await;
        let dup = b.invites.iter().any(|i| {
            i.workspace_id == workspace_id
                && i.from_account_key == session_account
                && i.to_friend_code.eq_ignore_ascii_case(&to_friend_code)
                && i.status == "pending"
        });
        if dup {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Invite already sent"
            })));
        }
        b.invites.push(InviteRecord {
            id: id.clone(),
            workspace_id,
            from_account_key: session_account.clone(),
            to_friend_code,
            status: "pending".to_string(),
            created_at: now,
        });
        bundle_save(state.get_ref(), &b)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    }

    Ok(HttpResponse::Ok().json(json!({"success": true, "invite_id": id})))
}

fn session_friend_code(state: &AppState, session_account: &str) -> String {
    state
        .accounts
        .lock()
        .unwrap()
        .get(session_account)
        .and_then(|a| a.friend_code.clone())
        .unwrap_or_default()
}

fn resolve_friend_code_account(state: &AppState, fc: &str) -> Option<String> {
    let accounts = state.accounts.lock().unwrap();
    accounts
        .values()
        .find(|a| {
            a.friend_code
                .as_deref()
                .map(|c| c.eq_ignore_ascii_case(fc))
                .unwrap_or(false)
        })
        .map(|a| a.account_name.clone())
}

/// GET .../api/server-org/invites
pub async fn list_server_org_invites(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    let my_fc = session_friend_code(state.get_ref(), &session_account);

    if is_postgres_backend() {
        let pool = persistence::pg_pool()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("no pool"))?;
        let client = pool
            .client()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("db: {e}")))?;
        let sent = client
            .query(
                "SELECT id, workspace_id, to_friend_code, status, created_at::text FROM server_workspace_invites WHERE lower(from_account_key) = lower($1)",
                &[&session_account],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        let received = client
            .query(
                "SELECT id, workspace_id, from_account_key, status, created_at::text FROM server_workspace_invites WHERE lower(to_friend_code) = lower($1) AND status = 'pending'",
                &[&my_fc],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;

        let sent_json: Vec<Value> = sent
            .iter()
            .map(|r| {
                json!({
                    "id": r.get::<_, String>(0),
                    "workspace_id": r.get::<_, String>(1),
                    "to_friend_code": r.get::<_, String>(2),
                    "status": r.get::<_, String>(3),
                    "created_at": r.get::<_, String>(4),
                })
            })
            .collect();
        let rec_json: Vec<Value> = received
            .iter()
            .map(|r| {
                json!({
                    "id": r.get::<_, String>(0),
                    "workspace_id": r.get::<_, String>(1),
                    "from_account": r.get::<_, String>(2),
                    "status": r.get::<_, String>(3),
                    "created_at": r.get::<_, String>(4),
                })
            })
            .collect();

        return Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "sent": sent_json,
            "received": rec_json
        })));
    }

    let b = bundle_load(state.get_ref()).await;
    let sent: Vec<Value> = b
        .invites
        .iter()
        .filter(|i| i.from_account_key == session_account)
        .map(|i| {
            json!({
                "id": i.id,
                "workspace_id": i.workspace_id,
                "to_friend_code": i.to_friend_code,
                "status": i.status,
                "created_at": i.created_at,
            })
        })
        .collect();
    let received: Vec<Value> = b
        .invites
        .iter()
        .filter(|i| i.to_friend_code.eq_ignore_ascii_case(&my_fc) && i.status == "pending")
        .map(|i| {
            json!({
                "id": i.id,
                "workspace_id": i.workspace_id,
                "from_account": i.from_account_key,
                "status": i.status,
                "created_at": i.created_at,
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "sent": sent,
        "received": received
    })))
}

/// POST .../api/server-org/invites/{invite_id}/accept
pub async fn accept_workspace_invite(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server, invite_id) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    let my_fc = session_friend_code(state.get_ref(), &session_account);

    if is_postgres_backend() {
        let pool = persistence::pg_pool()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("no pool"))?;
        let client = pool
            .client()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("db: {e}")))?;

        let row = client
            .query_opt(
                "SELECT workspace_id, to_friend_code, status FROM server_workspace_invites WHERE id = $1",
                &[&invite_id],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        let Some(row) = row else {
            return Ok(HttpResponse::NotFound()
                .json(json!({"success": false, "error": "Invite not found"})));
        };
        let wid: String = row.get(0);
        let to_fc: String = row.get(1);
        let status: String = row.get(2);
        if status != "pending" {
            return Ok(
                HttpResponse::BadRequest().json(json!({"success": false, "error": "Not pending"}))
            );
        }
        if !to_fc.eq_ignore_ascii_case(&my_fc) {
            return Ok(HttpResponse::Forbidden()
                .json(json!({"success": false, "error": "Wrong recipient"})));
        }

        client
            .execute(
                "UPDATE server_workspace_invites SET status = 'accepted' WHERE id = $1",
                &[&invite_id],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;

        let ex = client
            .query_opt(
                "SELECT 1 FROM server_workspace_members WHERE workspace_id = $1 AND lower(account_key) = lower($2)",
                &[&wid, &session_account],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        if ex.is_none() {
            client
                .execute(
                    "INSERT INTO server_workspace_members (workspace_id, account_key, role) VALUES ($1, $2, 'admin')",
                    &[&wid, &session_account],
                )
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        }
    } else {
        let _guard = ORG_JSON_LOCK.lock().await;
        let mut b = bundle_load(state.get_ref()).await;
        let inv = match b.invites.iter_mut().find(|i| i.id == invite_id) {
            Some(i) => i,
            None => {
                return Ok(HttpResponse::NotFound()
                    .json(json!({"success": false, "error": "Invite not found"})));
            }
        };
        if inv.status != "pending" {
            return Ok(
                HttpResponse::BadRequest().json(json!({"success": false, "error": "Not pending"}))
            );
        }
        if !inv.to_friend_code.eq_ignore_ascii_case(&my_fc) {
            return Ok(HttpResponse::Forbidden()
                .json(json!({"success": false, "error": "Wrong recipient"})));
        }
        inv.status = "accepted".to_string();
        let wid = inv.workspace_id.clone();
        let has = b.members.iter().any(|m| {
            m.workspace_id == wid && m.account_key.to_lowercase() == session_account.to_lowercase()
        });
        if !has {
            b.members.push(MemberRecord {
                workspace_id: wid,
                account_key: session_account.clone(),
                role: "admin".to_string(),
            });
        }
        bundle_save(state.get_ref(), &b)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    }

    Ok(HttpResponse::Ok().json(json!({"success": true})))
}

/// POST .../api/server-org/workspaces/{workspace_id}/tyrant-form
pub async fn ensure_tyrant_form(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<Value>,
) -> Result<HttpResponse> {
    let (url_account, server, workspace_id) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    if !workspace_access(state.get_ref(), &workspace_id, &session_account).await {
        return Ok(HttpResponse::Forbidden().json(json!({"success": false, "error": "No access"})));
    }

    let config = body.clone();

    if is_postgres_backend() {
        let pool = persistence::pg_pool()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("no pool"))?;
        let client = pool
            .client()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("db: {e}")))?;

        let existing = client
            .query_opt(
                "SELECT id, public_code FROM tyrant_forms WHERE workspace_id = $1 ORDER BY created_at DESC LIMIT 1",
                &[&workspace_id],
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        if let Some(row) = existing {
            let form_id: String = row.get(0);
            let code: String = row.get(1);
            client
                .execute(
                    "UPDATE tyrant_forms SET config = $1 WHERE id = $2",
                    &[&config, &form_id],
                )
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
            return Ok(HttpResponse::Ok().json(json!({
                "success": true,
                "form_id": form_id,
                "public_code": code,
            })));
        }

        let form_id = Uuid::new_v4().to_string();
        let public_code = generate_form_code();
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] =
            &[&form_id, &workspace_id, &public_code, &config];
        client
            .execute(
                "INSERT INTO tyrant_forms (id, workspace_id, public_code, config) VALUES ($1, $2, $3, $4)",
                params,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
        Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "form_id": form_id,
            "public_code": public_code,
        })))
    } else {
        let _guard = ORG_JSON_LOCK.lock().await;
        let mut b = bundle_load(state.get_ref()).await;
        if let Some(tf) = b
            .tyrant_forms
            .iter_mut()
            .find(|f| f.workspace_id == workspace_id)
        {
            tf.config = config.clone();
            let code = tf.public_code.clone();
            let fid = tf.id.clone();
            bundle_save(state.get_ref(), &b)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
            return Ok(HttpResponse::Ok().json(json!({
                "success": true,
                "form_id": fid,
                "public_code": code,
            })));
        }
        let form_id = Uuid::new_v4().to_string();
        let public_code = generate_form_code();
        let now = chrono::Utc::now().to_rfc3339();
        b.tyrant_forms.push(TyrantFormRecord {
            id: form_id.clone(),
            workspace_id: workspace_id.clone(),
            public_code: public_code.clone(),
            config,
            created_at: now,
        });
        bundle_save(state.get_ref(), &b)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
        Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "form_id": form_id,
            "public_code": public_code,
        })))
    }
}

/// GET /tyrant-form/{code}/api/config
pub async fn tyrant_public_config(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    let form = find_tyrant_form_by_code(state.get_ref(), &code).await;
    let Some((ws_id, config)) = form else {
        return Ok(
            HttpResponse::NotFound().json(json!({"success": false, "error": "Form not found"}))
        );
    };
    Ok(HttpResponse::Ok().json(json!({
        "workspace_id": ws_id,
        "config": config,
    })))
}

async fn find_tyrant_form_by_code(state: &AppState, code: &str) -> Option<(String, Value)> {
    if is_postgres_backend() {
        return pg_find_tyrant_form(state, code).await.ok().flatten();
    }
    let b = bundle_load(state).await;
    b.tyrant_forms
        .iter()
        .find(|f| f.public_code == code)
        .map(|f| (f.workspace_id.clone(), f.config.clone()))
}

async fn pg_find_tyrant_form(
    _state: &AppState,
    code: &str,
) -> Result<Option<(String, Value)>, Box<dyn std::error::Error + Send + Sync>> {
    let pool = persistence::pg_pool().ok_or("no pool")?;
    let client = pool.client().await?;
    let row = client
        .query_opt(
            "SELECT workspace_id, config FROM tyrant_forms WHERE public_code = $1",
            &[&code],
        )
        .await?;
    Ok(row.map(|r| (r.get::<_, String>(0), r.get::<_, Value>(1))))
}

#[derive(Deserialize)]
pub struct TyrantSubmitBody {
    pub player_id: String,
    /// Must match a successful player-lookup name for this form (non-empty after trim).
    pub player_name: String,
    pub alliance: String,
    pub archer: TyrantTroopBands,
    pub cavalry: TyrantTroopBands,
    pub infantry: TyrantTroopBands,
    #[serde(default)]
    pub utc_slots: Vec<String>,
    #[serde(default)]
    pub participate_full_five_hours: bool,
    pub auto_help_month_card_active: bool,
}

/// POST /tyrant-form/{code}/api/submit
pub async fn tyrant_public_submit(
    path: web::Path<String>,
    state: web::Data<AppState>,
    body: web::Json<TyrantSubmitBody>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    let form = find_tyrant_form_by_code(state.get_ref(), &code).await;
    let Some((workspace_id, config)) = form else {
        return Ok(
            HttpResponse::NotFound().json(json!({"success": false, "error": "Form not found"}))
        );
    };

    let pid = body.player_id.trim();
    if pid.is_empty() || !pid.chars().all(|c| c.is_ascii_digit()) {
        return Ok(HttpResponse::BadRequest()
            .json(json!({"success": false, "error": "Invalid player id"})));
    }

    if !validate_troop_bands(&body.archer)
        || !validate_troop_bands(&body.cavalry)
        || !validate_troop_bands(&body.infantry)
    {
        return Ok(HttpResponse::BadRequest()
            .json(json!({"success": false, "error": "Invalid troop bands"})));
    }

    let alliance = body.alliance.trim();
    if alliance.is_empty() || alliance.len() > 200 {
        return Ok(
            HttpResponse::BadRequest().json(json!({"success": false, "error": "Invalid alliance"}))
        );
    }

    let pname = body.player_name.trim();
    if pname.is_empty() || pname.len() > 200 {
        return Ok(HttpResponse::BadRequest()
            .json(json!({"success": false, "error": "Invalid player name"})));
    }

    let allowed: Vec<String> = config
        .get("alliances")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let non_of = config
        .get("include_non_of_above")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    const NON: &str = "Non of the above";
    let mut ok_list = allowed.clone();
    if non_of && !ok_list.iter().any(|s| s == NON) {
        ok_list.push(NON.to_string());
    }
    if !allowed.is_empty() && !ok_list.iter().any(|s| s == alliance) {
        return Ok(HttpResponse::BadRequest()
            .json(json!({"success": false, "error": "Alliance not allowed"})));
    }

    let expected_kingdom = config
        .get("kingdom_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let verified_name = match kingshot_api::fetch_player(pid).await {
        Ok(player) => {
            if player.nickname.trim() != pname {
                return Ok(HttpResponse::BadRequest().json(
                    json!({"success": false, "error": "Player name does not match player id"}),
                ));
            }
            if let Some(ref exp) = expected_kingdom {
                if player.kid.trim() != exp.as_str() {
                    return Ok(HttpResponse::BadRequest().json(
                        json!({"success": false, "error": "Player not in kingdom for this form"}),
                    ));
                }
            }
            player.nickname.trim().to_string()
        }
        Err(e) => {
            eprintln!(
                "Kingshot API fetch_player failed: {e}. Falling back to submitted name: {pname}"
            );
            pname.to_string()
        }
    };

    let payload = TyrantSubmissionPayload {
        alliance: alliance.to_string(),
        archer: body.archer.clone(),
        cavalry: body.cavalry.clone(),
        infantry: body.infantry.clone(),
        utc_slots: body.utc_slots.clone(),
        participate_full_five_hours: body.participate_full_five_hours,
        player_name: Some(verified_name),
        auto_help_month_card_active: Some(body.auto_help_month_card_active),
    };
    let payload_v = serde_json::to_value(&payload)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let form_record = find_tyrant_form_row_by_code(state.get_ref(), &code).await;

    if is_postgres_backend() {
        let pool = persistence::pg_pool()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("no pool"))?;
        let client = pool
            .client()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("db: {e}")))?;
        let form_uuid: Option<String> = form_record.map(|f| f.id);
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] =
            &[&workspace_id, &form_uuid, &code, &pid, &payload_v];
        client
            .execute(
                "INSERT INTO tyrant_submissions (workspace_id, form_id, public_code, player_id, payload) VALUES ($1, $2, $3, $4, $5)",
                params,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{e}")))?;
    } else {
        let _guard = ORG_JSON_LOCK.lock().await;
        let mut b = bundle_load(state.get_ref()).await;
        let nid = next_submission_id(&b);
        let form_id = form_record.map(|f| f.id);
        b.tyrant_submissions.push(TyrantSubmissionRecord {
            id: nid,
            workspace_id,
            form_id,
            public_code: code.clone(),
            player_id: pid.to_string(),
            payload: payload_v,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        bundle_save(state.get_ref(), &b)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    }

    Ok(HttpResponse::Ok().json(json!({"success": true})))
}

#[derive(Clone)]
struct FormRowLite {
    id: String,
}

async fn find_tyrant_form_row_by_code(state: &AppState, code: &str) -> Option<FormRowLite> {
    if is_postgres_backend() {
        return pg_form_row_by_code(state, code).await.ok().flatten();
    }
    let b = bundle_load(state).await;
    b.tyrant_forms
        .iter()
        .find(|f| f.public_code == code)
        .map(|f| FormRowLite { id: f.id.clone() })
}

async fn pg_form_row_by_code(
    _state: &AppState,
    code: &str,
) -> Result<Option<FormRowLite>, Box<dyn std::error::Error + Send + Sync>> {
    let pool = persistence::pg_pool().ok_or("no pool")?;
    let client = pool.client().await?;
    let row = client
        .query_opt(
            "SELECT id FROM tyrant_forms WHERE public_code = $1",
            &[&code],
        )
        .await?;
    Ok(row.map(|r| FormRowLite { id: r.get(0) }))
}

fn validate_troop_bands(t: &TyrantTroopBands) -> bool {
    valid_band_level(&t.level_band) && valid_band_tg(&t.tg_band)
}

/// GET /tyrant-form/{code}/api/player-lookup/{player_id}
pub async fn tyrant_player_lookup(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (code, player_id) = path.into_inner();
    let player_id = player_id.trim();
    let form = find_tyrant_form_by_code(state.get_ref(), &code).await;
    let Some((_, config)) = form else {
        return Ok(
            HttpResponse::NotFound().json(json!({"success": false, "error": "Form not found"}))
        );
    };

    let expected_kingdom = config
        .get("kingdom_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match kingshot_api::fetch_player(player_id).await {
        Ok(player) => {
            let castle_level = kingshot_api::stove_lv_to_label(player.stove_lv);
            let kingdom_mismatch = if let Some(ref exp) = expected_kingdom {
                player.kid.trim() != exp.as_str()
            } else {
                false
            };
            Ok(HttpResponse::Ok().json(json!({
                "success": !kingdom_mismatch,
                "name": player.nickname,
                "player_id": player.fid,
                "avatar_image": player.avatar_image,
                "castle_level": castle_level,
                "kingdom": player.kid,
                "kingdom_mismatch": kingdom_mismatch,
                "is_fallback": false,
                "error": if kingdom_mismatch { Some("This player is not in the kingdom this form is for") } else { None::<&str> }
            })))
        }
        Err(_) => {
            let fallback_kingdom = expected_kingdom.clone().unwrap_or_else(|| "0".to_string());
            Ok(HttpResponse::Ok().json(json!({
                "success": true,
                "name": "",
                "player_id": player_id,
                "avatar_image": None::<String>,
                "castle_level": "Level 30",
                "kingdom": fallback_kingdom,
                "kingdom_mismatch": false,
                "is_fallback": true,
                "error": None::<String>
            })))
        }
    }
}

/// GET .../api/server-org/workspaces/{workspace_id}/tyrant-submissions?sort=level_then_tg|tg_then_level
pub async fn list_tyrant_submissions(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
    q: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let (url_account, server, workspace_id) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let Some((session_account, session_server)) = read_session(&session) else {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Not logged in"}))
        );
    };
    if session_account.to_lowercase() != url_account || session_server != server {
        return Ok(
            HttpResponse::Unauthorized().json(json!({"success": false, "error": "Unauthorized"}))
        );
    }

    if !workspace_access(state.get_ref(), &workspace_id, &session_account).await {
        return Ok(HttpResponse::Forbidden().json(json!({"success": false, "error": "No access"})));
    }

    let sort = q.get("sort").map(|s| s.as_str()).unwrap_or("level_then_tg");

    let mut rows = load_submissions_for_workspace(state.get_ref(), &workspace_id).await;
    rows = dedupe_latest_per_player(rows);
    sort_submissions(&mut rows, sort);

    let out: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "player_id": r.player_id,
                "payload": r.payload,
                "created_at": r.created_at,
                "rank_min_level": r.rank_min_level,
                "rank_min_tg": r.rank_min_tg,
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(json!({ "success": true, "submissions": out, "sort": sort })))
}

#[derive(Clone)]
struct RankedRow {
    id: String,
    player_id: String,
    payload: Value,
    created_at: String,
    rank_min_level: i32,
    rank_min_tg: i32,
}

/// Higher is stronger for ranking (minimum across troop types wins weakest link).
fn level_val(s: &str) -> i32 {
    match s {
        "level_11" => 2,
        "level_10" => 1,
        "level_1_9" => 0,
        // Legacy submissions before granular bands
        "below_9" => 0,
        "nine_to_eleven" => 1,
        _ => 0,
    }
}

/// Higher is stronger for ranking (minimum across troop types wins weakest link).
fn tg_val(s: &str) -> i32 {
    match s {
        "tg8" => 4,
        "tg7" => 3,
        "tg6" => 2,
        "tg5" => 1,
        "below_tg5" => 0,
        // Legacy: treat as middle of TG5–TG8 range for sorting
        "tg5_to_tg8" => 2,
        _ => 0,
    }
}

fn payload_scores(p: &TyrantSubmissionPayload) -> (i32, i32) {
    let l = [
        level_val(&p.archer.level_band),
        level_val(&p.cavalry.level_band),
        level_val(&p.infantry.level_band),
    ];
    let t = [
        tg_val(&p.archer.tg_band),
        tg_val(&p.cavalry.tg_band),
        tg_val(&p.infantry.tg_band),
    ];
    (*l.iter().min().unwrap(), *t.iter().min().unwrap())
}

fn sort_submissions(rows: &mut Vec<RankedRow>, sort: &str) {
    match sort {
        "tg_then_level" => {
            rows.sort_by(|a, b| {
                b.rank_min_tg
                    .cmp(&a.rank_min_tg)
                    .then_with(|| b.rank_min_level.cmp(&a.rank_min_level))
                    .then_with(|| a.player_id.cmp(&b.player_id))
            });
        }
        _ => {
            rows.sort_by(|a, b| {
                b.rank_min_level
                    .cmp(&a.rank_min_level)
                    .then_with(|| b.rank_min_tg.cmp(&a.rank_min_tg))
                    .then_with(|| a.player_id.cmp(&b.player_id))
            });
        }
    }
}

fn dedupe_latest_per_player(mut rows: Vec<RankedRow>) -> Vec<RankedRow> {
    let mut best: HashMap<String, RankedRow> = HashMap::new();
    for r in rows.drain(..) {
        let e = best.entry(r.player_id.clone()).or_insert(r.clone());
        if r.created_at > e.created_at {
            *e = r;
        }
    }
    best.into_values().collect()
}

async fn load_submissions_for_workspace(state: &AppState, workspace_id: &str) -> Vec<RankedRow> {
    let raw = if is_postgres_backend() {
        pg_list_submissions(state, workspace_id)
            .await
            .unwrap_or_default()
    } else {
        let b = bundle_load(state).await;
        b.tyrant_submissions
            .iter()
            .filter(|s| s.workspace_id == workspace_id)
            .map(|s| {
                (
                    s.id.to_string(),
                    s.player_id.clone(),
                    s.payload.clone(),
                    s.created_at.clone(),
                )
            })
            .collect::<Vec<_>>()
    };

    raw.into_iter()
        .filter_map(|(id, pid, payload, created_at)| {
            let p: TyrantSubmissionPayload = serde_json::from_value(payload.clone()).ok()?;
            let (rl, rtg) = payload_scores(&p);
            Some(RankedRow {
                id,
                player_id: pid,
                payload,
                created_at,
                rank_min_level: rl,
                rank_min_tg: rtg,
            })
        })
        .collect()
}

async fn pg_list_submissions(
    _state: &AppState,
    workspace_id: &str,
) -> Result<Vec<(String, String, Value, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let pool = persistence::pg_pool().ok_or("no pool")?;
    let client = pool.client().await?;
    let rows = client
        .query(
            "SELECT id::text, player_id, payload, created_at::text FROM tyrant_submissions WHERE workspace_id = $1 ORDER BY id ASC",
            &[&workspace_id],
        )
        .await?;
    let mut out = Vec::new();
    for r in rows {
        out.push((r.get(0), r.get(1), r.get(2), r.get(3)));
    }
    Ok(out)
}

fn read_session(session: &Session) -> Option<(String, u32)> {
    let account: String = match session.get("account_name") {
        Ok(Some(a)) => a,
        _ => return None,
    };
    let server: u32 = match session.get("server_number") {
        Ok(Some(s)) => s,
        _ => return None,
    };
    Some((account, server))
}
