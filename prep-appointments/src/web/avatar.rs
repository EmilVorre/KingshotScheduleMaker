//! Avatar caching: fetch profile pictures from Kingshot API and cache locally.

use actix_web::{web, HttpResponse, Result};
use std::path::Path;

use crate::kingshot_api;

/// Get or fetch avatar for a player ID.
/// Checks cache first; if missing, fetches from Kingshot API, downloads the image, and caches it.
pub async fn get_avatar(
    path: web::Path<String>,
    state: web::Data<super::state::AppState>,
) -> Result<HttpResponse> {
    let player_id = path.into_inner().trim().to_string();
    if player_id.is_empty() || !player_id.chars().all(|c| c.is_ascii_digit()) {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let avatars_dir = format!("{}/avatars", state.data_dir);
    std::fs::create_dir_all(&avatars_dir).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to create avatars dir: {}", e))
    })?;

    // Try common extensions for cached file
    let cached_path = [
        format!("{}/{}.png", avatars_dir, player_id),
        format!("{}/{}.jpg", avatars_dir, player_id),
        format!("{}/{}.jpeg", avatars_dir, player_id),
        format!("{}/{}.webp", avatars_dir, player_id),
    ]
    .into_iter()
    .find(|p| Path::new(p).exists());

    if let Some(ref path) = cached_path {
        if let Ok(bytes) = std::fs::read(path) {
            let content_type = if path.ends_with(".png") {
                "image/png"
            } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                "image/jpeg"
            } else if path.ends_with(".webp") {
                "image/webp"
            } else {
                "image/png"
            };
            return Ok(HttpResponse::Ok()
                .content_type(content_type)
                .body(bytes));
        }
    }

    // Not cached: fetch from Kingshot API
    let player = match kingshot_api::fetch_player(&player_id).await {
        Ok(p) => p,
        Err(_) => return Ok(HttpResponse::NotFound().finish()),
    };

    let avatar_url = match player.avatar_image {
        Some(url) if !url.is_empty() => url,
        _ => return Ok(HttpResponse::NotFound().finish()),
    };

    // Download the image
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let response = client
        .get(&avatar_url)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    if !response.status().is_success() {
        return Ok(HttpResponse::NotFound().finish());
    }

    // Determine extension from URL or Content-Type (before consuming response)
    let ext = response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            if ct.contains("jpeg") || ct.contains("jpg") {
                "jpg"
            } else if ct.contains("webp") {
                "webp"
            } else {
                "png"
            }
        })
        .unwrap_or_else(|| {
            if avatar_url.contains(".jpg") || avatar_url.contains(".jpeg") {
                "jpg"
            } else if avatar_url.contains(".webp") {
                "webp"
            } else {
                "png"
            }
        });

    let bytes = response
        .bytes()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let cache_path = format!("{}/{}.{}", avatars_dir, player_id, ext);
    if std::fs::write(&cache_path, &bytes).is_err() {
        // Cache write failed but we can still serve the response
    }

    let content_type = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    };

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .body(bytes))
}
