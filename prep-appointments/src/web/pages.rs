//! HTML page handlers.

use actix_web::{HttpResponse, Result};

/// SPA index - serves the React app's index.html for client-side routing.
/// Sends no-cache headers so browsers fetch fresh HTML after each deploy.
pub async fn spa_index() -> Result<HttpResponse> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static/dist/index.html");
    let body = match std::fs::read_to_string(&path) {
        Ok(html) => html,
        Err(_) => include_str!("../../templates/index.html").to_string(),
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
        .insert_header(("Pragma", "no-cache"))
        .insert_header(("Expires", "0"))
        .body(body))
}

// Page handlers removed - React SPA handles all page routes via client-side routing.
// spa_index serves the React app; API routes handle data.
