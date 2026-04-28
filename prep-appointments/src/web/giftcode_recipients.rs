//! Giftcode recipients: which alliance players get gift codes auto-redeemed.
//! Also handles redemption and fetching codes from the distribution API.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::giftcode_api;
use crate::kingshot_api;

use super::alliance_invites;
use super::persistence::{list_domain_docs, load_domain_doc, save_domain_doc};
use super::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct GiftcodeRecipientsFile {
    player_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct RedeemedCodesFile {
    redeemed_codes: Vec<String>,
}

fn recipients_key(account: &str, server: u32) -> String {
    format!("{}_{}", account.to_lowercase(), server)
}

fn redeemed_key(account: &str, server: u32) -> String {
    format!("{}_{}", account.to_lowercase(), server)
}

pub(crate) async fn load_redeemed_internal(
    data_dir: &str,
    account: &str,
    server: u32,
) -> std::collections::HashSet<String> {
    let key = redeemed_key(account, server);
    load_domain_doc::<RedeemedCodesFile>(data_dir, "giftcode_redeemed", &key)
        .await
        .map(|d| d.redeemed_codes.into_iter().collect())
        .unwrap_or_default()
}

pub(crate) async fn add_redeemed_code_internal(
    data_dir: &str,
    account: &str,
    server: u32,
    code: &str,
) -> std::io::Result<()> {
    let mut codes = load_redeemed_internal(data_dir, account, server).await;
    let code_clean = code.trim().to_uppercase();
    if code_clean.is_empty() {
        return Ok(());
    }
    codes.insert(code_clean);
    let data = RedeemedCodesFile {
        redeemed_codes: codes.into_iter().collect(),
    };
    let key = redeemed_key(account, server);
    save_domain_doc(data_dir, "giftcode_redeemed", &key, &data).await
}

pub(crate) async fn load_recipients_internal(
    data_dir: &str,
    account: &str,
    server: u32,
) -> Vec<String> {
    let key = recipients_key(account, server);
    load_domain_doc::<GiftcodeRecipientsFile>(data_dir, "giftcode_recipients", &key)
        .await
        .map(|d| d.player_ids)
        .unwrap_or_default()
}

async fn load_recipients(data_dir: &str, account: &str, server: u32) -> Vec<String> {
    load_recipients_internal(data_dir, account, server).await
}

async fn save_recipients(
    data_dir: &str,
    account: &str,
    server: u32,
    player_ids: &[String],
) -> std::io::Result<()> {
    let data = GiftcodeRecipientsFile {
        player_ids: player_ids.to_vec(),
    };
    let key = recipients_key(account, server);
    save_domain_doc(data_dir, "giftcode_recipients", &key, &data).await
}

pub(crate) async fn list_accounts_with_recipients(data_dir: &str) -> Vec<(String, u32)> {
    list_domain_docs::<GiftcodeRecipientsFile>(data_dir, "giftcode_recipients", None)
        .await
        .into_iter()
        .filter_map(|(key, v)| {
            if v.player_ids.is_empty() {
                return None;
            }
            let (account, server_str) = key.split_once('_')?;
            let server = server_str.parse::<u32>().ok()?;
            Some((account.to_string(), server))
        })
        .collect()
}

/// GET /{account}/{server}/api/giftcode-recipients - List selected player IDs for auto-redeem
pub async fn get_recipients(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let (session_account, _) = match auth_check(&session, &state, &url_account, server).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let player_ids = load_recipients(&state.data_dir, &session_account, server).await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "player_ids": player_ids
    })))
}

#[derive(Deserialize)]
pub struct SetRecipientsRequest {
    pub player_ids: Vec<String>,
}

/// PUT /{account}/{server}/api/giftcode-recipients - Save selected player IDs
pub async fn set_recipients(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<SetRecipientsRequest>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let (session_account, _) = match auth_check(&session, &state, &url_account, server).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let player_ids: Vec<String> = body
        .player_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .collect();

    save_recipients(&state.data_dir, &session_account, server, &player_ids)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "player_ids": player_ids
    })))
}

async fn auth_check(
    session: &Session,
    state: &web::Data<AppState>,
    url_account: &str,
    server: u32,
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
    let session_server: u32 = match session.get("server_number") {
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

    if session_account.to_lowercase() != url_account || session_server != server {
        return Err(HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"})));
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
        let invites = alliance_invites::load_invites(&state.data_dir).await;
        let has_inv = invites
            .values()
            .any(|i| i.to_friend_code.eq_ignore_ascii_case(&my_fc));
        (access, has_inv)
    };
    if !alliance_access && !has_invites {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Alliance access required"
        })));
    }

    Ok((session_account, session_server))
}

#[derive(Deserialize)]
pub struct RedeemRequest {
    pub giftcode: String,
}

/// POST /{account}/{server}/api/redeem-giftcode - Redeem a gift code for all selected recipients
pub async fn redeem_giftcode(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    body: web::Json<RedeemRequest>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let (session_account, _) = match auth_check(&session, &state, &url_account, server).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    let giftcode = body.giftcode.trim().to_string();
    if giftcode.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Gift code is required"
        })));
    }

    let player_ids = load_recipients(&state.data_dir, &session_account, server).await;
    if player_ids.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "No recipients selected",
            "results": []
        })));
    }

    let mut results = Vec::new();
    for player_id in &player_ids {
        let result = kingshot_api::redeem_giftcode(player_id, &giftcode).await;
        results.push(serde_json::json!({
            "player_id": result.player_id,
            "status": result.status,
            "message": result.message
        }));
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "results": results
    })))
}

/// GET /{account}/{server}/api/fetch-giftcodes - Fetch gift codes from distribution API
pub async fn fetch_giftcodes(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account, server) = path.into_inner();
    let url_account = url_account.to_lowercase();

    let _ = match auth_check(&session, &state, &url_account, server).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };

    match giftcode_api::fetch_giftcodes().await {
        Ok(codes) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "codes": codes.iter().map(|c| serde_json::json!({ "code": c.code, "date": c.date })).collect::<Vec<_>>()
        }))),
        Err(e) => Ok(HttpResponse::BadGateway().json(serde_json::json!({
            "success": false,
            "error": e
        }))),
    }
}
