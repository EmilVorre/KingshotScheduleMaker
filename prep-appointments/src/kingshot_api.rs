//! Kingshot Player API integration - fetches player info (nickname, etc.) by player ID.

use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://kingshot-giftcode.centurygame.com/api/player";
const SECRET: &str = "mN4!pQs6JrYwV9";

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
