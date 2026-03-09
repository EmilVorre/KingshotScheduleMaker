//! Kingshot Player API integration - fetches player info (nickname, etc.) by player ID.
//! Also supports gift code redemption via the Century gift code API.

use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://kingshot-giftcode.centurygame.com/api/player";
const GIFTCODE_URL: &str = "https://kingshot-giftcode.centurygame.com/api/gift_code";
const SECRET: &str = "mN4!pQs6JrYwV9";

/// Result of a gift code redemption attempt for one player.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RedemptionResult {
    pub player_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PlayerData {
    pub nickname: String,
    pub fid: String,
    pub stove_lv: u32,
    pub kid: String,
    pub avatar_image: Option<String>,
    pub stove_lv_content: Option<String>,
}

/// Manually parse player data from JSON object to handle API's inconsistent types.
fn parse_player_data(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<PlayerData, String> {
    let nickname = obj
        .get("nickname")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let fid = obj
        .get("fid")
        .map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let stove_lv = obj
        .get("stove_lv")
        .map(|v| match v {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
            serde_json::Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        })
        .unwrap_or(0);
    let kid = obj
        .get("kid")
        .map(|v| match v {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let avatar_image = obj
        .get("avatar_image")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let stove_lv_content = obj
        .get("stove_lv_content")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    Ok(PlayerData {
        nickname,
        fid,
        stove_lv,
        kid,
        avatar_image,
        stove_lv_content,
    })
}

fn build_signed_body(fid: &str) -> String {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let base = format!("fid={}&time={}", fid, timestamp_ms);
    let to_hash = format!("{}{}", base, SECRET);
    let sign = format!("{:x}", md5::compute(to_hash.as_bytes()));

    format!("sign={}&{}", sign, base)
}

/// Build signed body for gift code redemption. Uses time in milliseconds (per Discord bot).
fn build_giftcode_body(fid: &str, giftcode: &str) -> String {
    let time_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    // Keys sorted: cdk, fid, time
    let base = format!("cdk={}&fid={}&time={}", giftcode, fid, time_ms);
    let to_hash = format!("{}{}", base, SECRET);
    let sign = format!("{:x}", md5::compute(to_hash.as_bytes()));
    format!("sign={}&{}", sign, base)
}

/// Clean gift code: remove invisible Unicode (e.g. RLM) that can contaminate codes.
fn clean_giftcode(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && *c != '\u{200f}' && *c != '\u{200e}')
        .collect::<String>()
        .trim()
        .to_string()
}

/// Convert stove_lv (1-84) to human-readable castle level label.
/// 1-30: Level 1..Level 30; 31-34: 30-1..30-4; 35+: TG 1, TG 1 - 1, etc.
pub fn stove_lv_to_label(lv: u32) -> String {
    match lv {
        1..=30 => format!("Level {}", lv),
        31..=34 => format!("30-{}", lv - 30),
        _ if lv >= 35 => {
            let tg = ((lv - 35) / 5) + 1;
            let sub = (lv - 35) % 5;
            if sub == 0 {
                format!("TG {}", tg)
            } else {
                format!("TG {} - {}", tg, sub)
            }
        }
        _ => format!("Level {}", lv),
    }
}

/// Fetch player data by ID from the Kingshot API.
/// Returns the player's nickname (character name) on success.
pub async fn fetch_player(fid: &str) -> Result<PlayerData, String> {
    let fid = fid.trim();
    if fid.is_empty() {
        return Err("Player ID is required".to_string());
    }
    if !fid.chars().all(|c| c.is_ascii_digit()) {
        return Err("Player ID must contain only digits".to_string());
    }

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let body = build_signed_body(fid);

    let response = client
        .post(API_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let raw: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // Manually parse to handle API's inconsistent types (fid as int, stove_lv as "", etc.)
    let data_value = raw.get("data");
    let err_code = raw.get("err_code").and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_u64().map(|u| u as u32),
        serde_json::Value::String(s) if !s.is_empty() => s.parse().ok(),
        _ => None,
    });

    match status.as_u16() {
        200 => {
            if let Some(data_obj) = data_value.and_then(|v| v.as_object()) {
                let data = parse_player_data(data_obj)?;
                Ok(data)
            } else if err_code == Some(40004)
                || (err_code == Some(40001)
                    && raw
                        .get("msg")
                        .and_then(|m| m.as_str())
                        .map_or(false, |m| m.to_lowercase().contains("role not exist")))
            {
                Err("Player not found".to_string())
            } else {
                Err(format!(
                    "API error: {:?} - {}",
                    err_code,
                    raw.get("msg").and_then(|m| m.as_str()).unwrap_or("")
                ))
            }
        }
        429 => Err("Rate limit exceeded. Please wait a moment and try again.".to_string()),
        _ => Err(format!(
            "HTTP {}: {}",
            status,
            raw.get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
        )),
    }
}

/// Redeem a gift code for a player. Establishes session via player API, then redeems.
/// Returns status and human-readable message.
pub async fn redeem_giftcode(player_id: &str, giftcode: &str) -> RedemptionResult {
    let fid = player_id.trim();
    let giftcode = clean_giftcode(giftcode);

    if fid.is_empty() || !fid.chars().all(|c| c.is_ascii_digit()) {
        return RedemptionResult {
            player_id: player_id.to_string(),
            status: "ERROR".to_string(),
            message: "Invalid player ID".to_string(),
        };
    }
    if giftcode.is_empty() {
        return RedemptionResult {
            player_id: player_id.to_string(),
            status: "ERROR".to_string(),
            message: "Invalid gift code".to_string(),
        };
    }

    let client = match Client::builder()
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return RedemptionResult {
                player_id: player_id.to_string(),
                status: "ERROR".to_string(),
                message: format!("Failed to create client: {}", e),
            };
        }
    };

    // Step 1: Establish session via player API (required for cookies)
    let player_body = build_signed_body(fid);
    let player_resp = match client
        .post(API_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Origin", "https://kingshot-giftcode.centurygame.com")
        .body(player_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return RedemptionResult {
                player_id: player_id.to_string(),
                status: "ERROR".to_string(),
                message: format!("Login failed: {}", e),
            };
        }
    };

    let login_raw: serde_json::Value = player_resp.json().await.unwrap_or_default();
    let login_ok = login_raw
        .get("msg")
        .and_then(|m| m.as_str())
        .map_or(false, |m| m == "success");

    if !login_ok {
        return RedemptionResult {
            player_id: player_id.to_string(),
            status: "LOGIN_FAILED".to_string(),
            message: "Could not establish session for player".to_string(),
        };
    }

    // Step 2: Redeem gift code (cookies from step 1 are sent automatically)
    let gift_body = build_giftcode_body(fid, &giftcode);
    let gift_resp = match client
        .post(GIFTCODE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Origin", "https://kingshot-giftcode.centurygame.com")
        .body(gift_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return RedemptionResult {
                player_id: player_id.to_string(),
                status: "ERROR".to_string(),
                message: format!("Redemption request failed: {}", e),
            };
        }
    };

    let raw: serde_json::Value = match gift_resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return RedemptionResult {
                player_id: player_id.to_string(),
                status: "ERROR".to_string(),
                message: format!("Invalid response: {}", e),
            };
        }
    };

    let msg = raw
        .get("msg")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .trim_matches('.');
    let err_code = raw.get("err_code").and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_u64().map(|u| u as u32),
        serde_json::Value::String(s) if !s.is_empty() => s.parse().ok(),
        _ => None,
    });

    let (status, message) = match msg {
        "SUCCESS" => ("SUCCESS", "Redeemed successfully".to_string()),
        "RECEIVED" if err_code == Some(40008) => ("RECEIVED", "Already redeemed".to_string()),
        "SAME TYPE EXCHANGE" if err_code == Some(40011) => {
            ("SAME_TYPE_EXCHANGE", "Already had same reward".to_string())
        }
        "TIME ERROR" if err_code == Some(40007) => ("TIME_ERROR", "Code expired".to_string()),
        "CDK NOT FOUND" if err_code == Some(40014) => {
            ("CDK_NOT_FOUND", "Code invalid or not found".to_string())
        }
        "USED" if err_code == Some(40005) => ("USAGE_LIMIT", "Usage limit reached".to_string()),
        "TIMEOUT RETRY" if err_code == Some(40004) => {
            ("TIMEOUT_RETRY", "Rate limited, try again later".to_string())
        }
        "NOT LOGIN" => ("LOGIN_EXPIRED", "Session expired".to_string()),
        "STOVE_LV ERROR" if err_code == Some(40006) => (
            "TOO_SMALL_SPEND_MORE",
            "Town Center level too low".to_string(),
        ),
        "RECHARGE_MONEY ERROR" if err_code == Some(40017) => {
            ("TOO_POOR_SPEND_MORE", "VIP level too low".to_string())
        }
        _ if msg.to_lowercase().contains("sign error") => {
            ("SIGN_ERROR", "API sign error".to_string())
        }
        _ => ("UNKNOWN", format!("{} (err_code: {:?})", msg, err_code)),
    };

    RedemptionResult {
        player_id: player_id.to_string(),
        status: status.to_string(),
        message,
    }
}
