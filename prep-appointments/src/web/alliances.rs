//! Alliance member management - add/remove players per alliance, stored as JSON files.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::alliance_invites;
use super::persistence::{delete_domain_doc, load_domain_doc, save_domain_doc};
use super::state::AppState;
use crate::kingshot_api::{self, stove_lv_to_label};

/// Alliance name -> filesystem-safe slug (lowercase, non-alphanumeric -> underscore)
pub fn alliance_to_slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            'A'..='Z' => char::from_u32(c as u32 + 32).unwrap_or(c),
            'a'..='z' | '0'..='9' => c,
            _ => '_',
        })
        .collect();
    s.split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .take(64)
        .collect()
}

fn alliance_doc_key(account: &str, server: u32, slug: &str) -> String {
    format!("{}_{}_{}", account.to_lowercase(), server, slug)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlliancePlayer {
    pub player_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub castle_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kingdom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_image: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AllianceFile {
    alliance_name: String,
    #[serde(default)]
    alliance_id: Option<String>,
    #[serde(default)]
    players: Vec<AlliancePlayer>,
}

fn load_alliance_file(
    data_dir: &str,
    account: &str,
    server: u32,
    slug: &str,
) -> Option<AllianceFile> {
    let key = alliance_doc_key(account, server, slug);
    load_domain_doc(data_dir, "alliances", &key)
}

fn save_alliance_file(
    data_dir: &str,
    account: &str,
    server: u32,
    slug: &str,
    data: &AllianceFile,
) -> std::io::Result<()> {
    let key = alliance_doc_key(account, server, slug);
    save_domain_doc(data_dir, "alliances", &key, data)
}

/// List alliances: own + accepted invites
pub async fn list_alliances(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let (session_account, _) = match auth_check(&session, &state, &url_account, server) {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let mut alliances = Vec::new();

    // Own alliance
    let (alliance_id, alliance_tag, alliance_name) = {
        let accounts = state.accounts.lock().unwrap();
        match accounts.get(&session_account) {
            Some(a) => match (&a.alliance_id, &a.alliance_tag, &a.alliance_name) {
                (Some(id), Some(tag), Some(name)) => (id.clone(), tag.clone(), name.clone()),
                _ => (String::new(), String::new(), String::new()),
            },
            None => (String::new(), String::new(), String::new()),
        }
    };

    if !alliance_name.is_empty() {
        let slug = alliance_to_slug(&alliance_name);
        let (players, file_alliance_id) =
            if let Some(f) = load_alliance_file(&state.data_dir, &session_account, server, &slug) {
                (f.players, f.alliance_id)
            } else {
                (vec![], None)
            };
        let aid = file_alliance_id.unwrap_or_else(|| alliance_id.clone());
        alliances.push(serde_json::json!({
            "name": alliance_name,
            "slug": slug,
            "alliance_id": aid,
            "alliance_tag": alliance_tag,
            "players": players,
            "owner_account": session_account,
            "owner_server": server,
            "is_owner": true
        }));
    }

    // Invited alliances (accepted)
    let invites = alliance_invites::load_invites_for_user(&state, &session_account);
    for inv in invites {
        let (players, file_alliance_id) = load_alliance_file(
            &state.data_dir,
            &inv.from_account,
            inv.from_server,
            &inv.alliance_slug,
        )
        .map(|f| (f.players, f.alliance_id))
        .unwrap_or_else(|| (vec![], None));
        if players.is_empty() && file_alliance_id.is_none() {
            continue;
        }
        let aid = file_alliance_id.unwrap_or_default();
        let accounts = state.accounts.lock().unwrap();
        let tag = accounts
            .get(&inv.from_account)
            .and_then(|a| a.alliance_tag.as_ref())
            .cloned()
            .unwrap_or_default();
        drop(accounts);
        alliances.push(serde_json::json!({
            "name": inv.alliance_name,
            "slug": inv.alliance_slug,
            "alliance_id": aid,
            "alliance_tag": tag,
            "players": players,
            "owner_account": inv.from_account,
            "owner_server": inv.from_server,
            "is_owner": false
        }));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "alliances": alliances
    })))
}

/// Add a player to an alliance (fetch from Kingshot API, save to JSON).
/// Owner is from path; session must be owner or have accepted invite.
#[derive(Deserialize)]
pub struct AddPlayerRequest {
    pub player_id: String,
    #[serde(default)]
    #[allow(dead_code)] // Optional, ignored - alliance comes from owner
    pub alliance_name: Option<String>,
}

pub async fn add_player(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<AddPlayerRequest>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let (_session_account, _) =
        match auth_check_for_alliance_edit(&session, &state, &url_account, server) {
            Ok(v) => v,
            Err(resp) => return Ok(resp),
        };

    let (alliance_id, alliance_name, slug) = {
        let accounts = state.accounts.lock().unwrap();
        match accounts.get(&url_account) {
            Some(a) => match (&a.alliance_id, &a.alliance_name) {
                (Some(id), Some(name)) => {
                    let s = alliance_to_slug(name);
                    (id.clone(), name.clone(), s)
                }
                _ => {
                    return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                        "success": false,
                        "error": "No alliance assigned to this account"
                    })))
                }
            },
            None => {
                return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                    "success": false,
                    "error": "Account not found"
                })))
            }
        }
    };

    let player_id = body.player_id.trim();

    if player_id.is_empty() || !player_id.chars().all(|c| c.is_ascii_digit()) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Valid player ID (digits only) is required"
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

    let mut data =
        load_alliance_file(&state.data_dir, &url_account, server, &slug).unwrap_or(AllianceFile {
            alliance_name: alliance_name.clone(),
            alliance_id: Some(alliance_id.clone()),
            players: vec![],
        });

    if data.alliance_id.is_none() {
        data.alliance_id = Some(alliance_id);
    }

    if data.players.iter().any(|p| p.player_id == player.fid) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Player is already in this alliance"
        })));
    }

    let added_at = chrono::Local::now().to_rfc3339();
    data.players.push(AlliancePlayer {
        player_id: player.fid.clone(),
        name: player.nickname.clone(),
        castle_level: Some(stove_lv_to_label(player.stove_lv)),
        kingdom: Some(player.kid.clone()),
        avatar_image: player.avatar_image.clone(),
        added_at,
    });

    save_alliance_file(&state.data_dir, &url_account, server, &slug, &data).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "player": {
            "player_id": player.fid,
            "name": player.nickname,
            "castle_level": stove_lv_to_label(player.stove_lv),
            "kingdom": player.kid,
            "avatar_image": player.avatar_image
        }
    })))
}

/// Remove a player from an alliance
pub async fn remove_player(
    path: web::Path<(String, u32, String, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server, alliance_slug, player_id_or_name) = path.into_inner();
    let url_account = url_account.to_lowercase();
    let player_id_or_name = player_id_or_name.trim();

    let (session_account, _) =
        match auth_check_for_alliance_edit(&session, &state, &url_account, server) {
            Ok(v) => v,
            Err(resp) => return Ok(resp),
        };

    if !alliance_invites::has_alliance_access(
        state.get_ref(),
        &session_account,
        &url_account,
        server,
        &alliance_slug,
    ) {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized: you can only manage alliances you own or are invited to"
        })));
    }

    let mut data = load_alliance_file(&state.data_dir, &url_account, server, &alliance_slug);
    if data.is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Alliance not found"
        })));
    }
    let mut data = data.take().unwrap();

    let to_remove = if player_id_or_name.chars().all(|c| c.is_ascii_digit()) {
        data.players
            .iter()
            .find(|p| p.player_id == player_id_or_name)
            .map(|p| p.player_id.clone())
    } else {
        data.players
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(player_id_or_name))
            .map(|p| p.player_id.clone())
    };

    let player_id = match to_remove {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Player not found in this alliance"
            })));
        }
    };

    let _before = data.players.len();
    data.players.retain(|p| p.player_id != player_id);

    if data.players.is_empty() {
        let key = alliance_doc_key(&url_account, server, &alliance_slug);
        delete_domain_doc(&state.data_dir, "alliances", &key).ok();
    } else {
        save_alliance_file(&state.data_dir, &url_account, server, &alliance_slug, &data).map_err(
            |e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)),
        )?;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}

/// POST /{account}/{server}/api/alliances/{alliance_slug}/refresh-names - Refetch all player names from Kingshot API
pub async fn refresh_names(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server, alliance_slug) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let (session_account, _) =
        match auth_check_for_alliance_edit(&session, &state, &url_account, server) {
            Ok(v) => v,
            Err(resp) => return Ok(resp),
        };

    if !alliance_invites::has_alliance_access(
        state.get_ref(),
        &session_account,
        &url_account,
        server,
        &alliance_slug,
    ) {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized: you can only manage alliances you own or are invited to"
        })));
    }

    let mut data = load_alliance_file(&state.data_dir, &url_account, server, &alliance_slug);
    if data.is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Alliance not found"
        })));
    }
    let mut data = data.take().unwrap();

    let mut updated = 0u32;
    for player in &mut data.players {
        match kingshot_api::fetch_player(&player.player_id).await {
            Ok(p) => {
                player.name = p.nickname;
                player.castle_level = Some(stove_lv_to_label(p.stove_lv));
                player.kingdom = Some(p.kid);
                player.avatar_image = p.avatar_image;
                updated += 1;
            }
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    save_alliance_file(&state.data_dir, &url_account, server, &alliance_slug, &data).map_err(
        |e| actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e)),
    )?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "updated": updated,
        "total": data.players.len()
    })))
}

/// Auth for alliance edit: owner or invitee with accepted invite
fn auth_check_for_alliance_edit(
    session: &Session,
    state: &web::Data<AppState>,
    owner_account: &str,
    owner_server: u32,
) -> Result<(String, u32), HttpResponse> {
    let session_account: String = session
        .get("account_name")
        .map_err(|_| {
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"success": false, "error": "Session error"}))
        })?
        .ok_or_else(|| {
            HttpResponse::Unauthorized()
                .json(serde_json::json!({"success": false, "error": "Not logged in"}))
        })?;
    let session_server: u32 = session
        .get("server_number")
        .map_err(|_| {
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"success": false, "error": "Session error"}))
        })?
        .ok_or_else(|| {
            HttpResponse::Unauthorized()
                .json(serde_json::json!({"success": false, "error": "Not logged in"}))
        })?;

    if session_account.eq_ignore_ascii_case(owner_account) && session_server == owner_server {
        let alliance_access = {
            let accounts = state.accounts.lock().unwrap();
            accounts
                .get(&session_account)
                .map(|a| a.alliance_access)
                .unwrap_or(false)
        };
        if alliance_access {
            return Ok((session_account, session_server));
        }
    }

    let owner_slug = {
        let accounts = state.accounts.lock().unwrap();
        accounts
            .get(owner_account)
            .and_then(|a| a.alliance_name.as_ref())
            .map(|n| alliance_to_slug(n))
            .unwrap_or_default()
    };
    if owner_slug.is_empty() {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "No alliance assigned"
        })));
    }
    if alliance_invites::has_alliance_access(
        state.get_ref(),
        &session_account,
        owner_account,
        owner_server,
        &owner_slug,
    ) {
        return Ok((session_account, session_server));
    }

    Err(HttpResponse::Forbidden().json(serde_json::json!({
        "success": false,
        "error": "Alliance access required"
    })))
}

fn auth_check(
    session: &Session,
    state: &web::Data<AppState>,
    url_account: &str,
    server: u32,
) -> Result<(String, u32), HttpResponse> {
    let session_account: String = session
        .get("account_name")
        .map_err(|_| {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Session error"
            }))
        })?
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            }))
        })?;
    let session_server: u32 = session
        .get("server_number")
        .map_err(|_| {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Session error"
            }))
        })?
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            }))
        })?;

    if session_account.to_lowercase() != url_account || session_server != server {
        return Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let (alliance_access, has_invites) = {
        let accounts = state.accounts.lock().unwrap();
        let acc = accounts.get(&session_account);
        let access = acc.map(|a| a.alliance_access).unwrap_or(false);
        let my_fc = acc
            .and_then(|a| a.friend_code.as_ref())
            .cloned()
            .unwrap_or_default();
        drop(accounts);
        let invites = alliance_invites::load_invites(&state.data_dir);
        let has_inv = invites
            .values()
            .any(|i| i.to_friend_code.eq_ignore_ascii_case(&my_fc));
        (access, has_inv)
    };
    if !alliance_access && !has_invites {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Alliance access required. Please submit an application or accept an invite."
        })));
    }

    Ok((session_account, session_server))
}
