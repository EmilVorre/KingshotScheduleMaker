//! HTML page handlers.

use actix_web::{HttpResponse, Result};

/// SPA index - serves the React app's index.html for client-side routing
pub async fn spa_index() -> Result<HttpResponse> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static/dist/index.html");
    match std::fs::read_to_string(&path) {
        Ok(html) => Ok(HttpResponse::Ok().content_type("text/html").body(html)),
        Err(_) => {
            // Fallback when React build doesn't exist yet
            let html = include_str!("../../templates/index.html");
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
    }
}

// Page handlers removed - React SPA handles all page routes via client-side routing.
// spa_index serves the React app; API routes handle data.
