# Kingshot Player API – Rust Integration Guide

A quick guide to call the Kingshot player API from a Rust project. This API returns player info (nickname, TC level, etc.) given a player ID (`fid`).

---

## API Overview

| Property | Value |
|----------|-------|
| **URL** | `https://kingshot-giftcode.centurygame.com/api/player` |
| **Method** | `POST` |
| **Content-Type** | `application/x-www-form-urlencoded` |
| **Secret** | `mN4!pQs6JrYwV9` |

---

## Sign Generation

The API requires a signed request. Steps:

1. **Build the base string** (order matters):
   ```
   fid={player_id}&time={timestamp_ms}
   ```
   - `timestamp_ms` = current Unix time in **milliseconds**

2. **Compute the sign**:
   ```
   sign = MD5(base_string + SECRET)
   ```
   Append the secret to the base string, then take the MD5 hash (lowercase hex).

3. **Build the final body**:
   ```
   sign={sign}&fid={player_id}&time={timestamp_ms}
   ```

---

## Example Rust Implementation

### 1. Add dependencies (`Cargo.toml`)

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
md5 = "0.7"
```

### 2. Code

```rust
use reqwest::Client;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://kingshot-giftcode.centurygame.com/api/player";
const SECRET: &str = "mN4!pQs6JrYwV9";

#[derive(Debug, Deserialize)]
pub struct PlayerData {
    pub nickname: String,
    pub fid: String,
    pub stove_lv: u32,        // TC level (numeric)
    pub kid: String,          // Kingdom ID
    pub avatar_image: Option<String>,
    pub stove_lv_content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub msg: Option<String>,
    pub err_code: Option<u32>,
    pub data: Option<PlayerData>,
}

/// Generate the signed form body for the API request.
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

/// Fetch player data by ID.
pub async fn fetch_player(fid: &str) -> Result<PlayerData, String> {
    let client = Client::builder()
        // If you get SSL errors, you may need to disable cert verification (see note below)
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
    let json: ApiResponse = response.json().await.map_err(|e| e.to_string())?;

    match status.as_u16() {
        200 => {
            if let Some(data) = json.data {
                Ok(data)
            } else if json.err_code == Some(40004)
                || (json.err_code == Some(40001)
                    && json.msg.as_ref().map_or(false, |m| m.to_lowercase().contains("role not exist")))
            {
                Err("Player not found".to_string())
            } else {
                Err(format!(
                    "API error: {:?} - {}",
                    json.err_code,
                    json.msg.unwrap_or_default()
                ))
            }
        }
        429 => Err("Rate limit exceeded. Wait ~60 seconds.".to_string()),
        _ => Err(format!("HTTP {}: {:?}", status, json.msg)),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fid = "46765089"; // Example player ID
    let player = fetch_player(fid).await?;
    println!("{} (ID: {}) - TC Level: {}", player.nickname, player.fid, player.stove_lv);
    Ok(())
}
```

For async, add `tokio`:

```toml
tokio = { version = "1", features = ["full"] }
```

---

## Response Fields (success)

| Field | Type | Description |
|-------|------|-------------|
| `nickname` | string | Player display name |
| `fid` | string | Player ID |
| `stove_lv` | number | Town Center level (numeric) |
| `kid` | string | Kingdom ID |
| `avatar_image` | string? | Avatar URL |
| `stove_lv_content` | string? | TC level icon/asset URL |

---

## TC Level Mapping (optional)

If you want human-readable TC labels like the Discord bot:

| stove_lv | Label |
|----------|-------|
| 1–30 | Level 1, Level 2, … Level 30 |
| 31–34 | 30-1, 30-2, 30-3, 30-4 |
| 35–39 | TG 1, TG 1 - 1, … TG 1 - 4 |
| 40–44 | TG 2, TG 2 - 1, … TG 2 - 4 |
| … | … |
| 80–84 | TG 10, TG 10 - 1, … TG 10 - 4 |

---

## Error Codes

| err_code | Meaning |
|----------|---------|
| 40004 | Player does not exist |
| 40001 | Role not exist (player not found) |
| HTTP 429 | Rate limit – wait before retrying |

---

## Rate Limits

- The bot limits itself to **~30 requests per 60 seconds**.
- Use a similar limit in your Rust client to avoid 429s.
- Add a delay (e.g. 2 seconds) between requests if you batch calls.

---

## SSL Note

The Python bot disables SSL verification (`CERT_NONE`). The Rust example uses `danger_accept_invalid_certs(true)` for compatibility. For production, prefer proper certificate validation and only relax it if the API’s certificate is known to be problematic.

---

## Minimal Example (no structs)

```rust
let fid = "12345678";
let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
let base = format!("fid={}&time={}", fid, ts);
let sign = format!("{:x}", md5::compute(format!("{}{}", base, SECRET).as_bytes()));
let body = format!("sign={}&{}", sign, base);

let client = reqwest::Client::builder().danger_accept_invalid_certs(true).build()?;
let res = client.post(API_URL)
    .header("Content-Type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await?;

let json: serde_json::Value = res.json().await?;
let nickname = json["data"]["nickname"].as_str().unwrap_or("?");
let stove_lv = json["data"]["stove_lv"].as_u64().unwrap_or(0);
```
