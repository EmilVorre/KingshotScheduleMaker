//! Gift code distribution API - fetches new gift codes from the shared bot API.
//! Same source as Kingshot Discord Bot: https://github.com/kingshot-project/Kingshot-Discord-Bot

use reqwest::Client;
use std::time::Duration;

const DISTRIBUTION_API_URL: &str = "http://ks-gift-code-api.whiteout-bot.com/giftcode_api.php";
/// API key from Kingshot Discord Bot - may need env override for production.
const API_KEY: &str = "super_secret_bot_token_nobody_will_ever_find";

/// A gift code from the distribution API.
#[derive(Debug, Clone)]
pub struct GiftCode {
    pub code: String,
    pub date: String,
}

/// Fetch available gift codes from the distribution API.
/// Returns list of (code, date) or error string.
pub async fn fetch_giftcodes() -> Result<Vec<GiftCode>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(DISTRIBUTION_API_URL)
        .header("X-API-Key", API_KEY)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("API returned {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    if let Some(error) = body.get("error").and_then(|v| v.as_str()) {
        return Err(error.to_string());
    }
    if let Some(detail) = body.get("detail").and_then(|v| v.as_str()) {
        return Err(detail.to_string());
    }

    let codes_raw = body
        .get("codes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Invalid API response: missing 'codes'".to_string())?;

    let mut result = Vec::new();
    for item in codes_raw {
        let line = item.as_str().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let code = parts[0].to_string();
            let date = parts[1].to_string();
            if code.chars().all(|c| c.is_ascii_alphanumeric()) {
                result.push(GiftCode { code, date });
            }
        }
    }

    Ok(result)
}
