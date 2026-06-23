// RedNode-OS – API Authentication
//
// Simple token-based auth for the CNS API.
// For a personal single-user OS, we don't need full JWT/OAuth.
// A static bearer token (set via environment variable) is sufficient
// and far simpler than a full auth system.
//
// Security model:
// - RedNode is on VLAN 50, behind pfSense
// - Only your devices (VLAN 10) can reach port 8787
// - The bearer token adds a second layer — even if someone gets on VLAN 10,
//   they can't control RedNode without the token
// - Token is stored in Flutter Secure Storage (Android Keystore) on mobile
// - Token is set once and doesn't expire (personal use, not multi-tenant)
//
// For production/multi-user: replace with JWT + refresh tokens

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

/// The bearer token. Set via REDNODE_API_TOKEN environment variable.
/// If not set, auth is DISABLED (dev mode).
fn get_required_token() -> Option<String> {
    let token = std::env::var("REDNODE_API_TOKEN").ok()?;
    if token.is_empty() {
        return None;
    }
    Some(token)
}

/// Axum middleware: check Authorization header for bearer token.
/// Skips auth for: /health, /events (WebSocket upgrade), and when
/// REDNODE_API_TOKEN is not set.
pub async fn auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();

    // Always allow these without auth:
    // - /health (monitoring)
    // - /events (WebSocket — auth checked after upgrade if needed)
    if path == "/health" || path == "/events" {
        return Ok(next.run(req).await);
    }

    // If no token is configured, auth is disabled (dev mode)
    let required_token = match get_required_token() {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };

    // Extract bearer token from Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let provided_token = if let Some(token) = auth_header.strip_prefix("Bearer ") {
        token.trim()
    } else {
        // Also check query parameter (for browser/dashboard convenience)
        let query = req.uri().query().unwrap_or("");
        let token_param = query
            .split('&')
            .find(|p| p.starts_with("token="))
            .and_then(|p| p.strip_prefix("token="));
        match token_param {
            Some(t) => t,
            None => {
                tracing::warn!(path = %path, "API request without auth token");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    };

    // Constant-time comparison to prevent timing attacks
    if constant_time_eq(provided_token.as_bytes(), required_token.as_bytes()) {
        Ok(next.run(req).await)
    } else {
        tracing::warn!(path = %path, "API request with invalid auth token");
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Constant-time byte comparison (prevents timing side-channels).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Generate a random API token (called once during first setup).
pub fn generate_token() -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(uuid::Uuid::new_v4().to_string().as_bytes());
    hasher.update(chrono::Utc::now().to_rfc3339().as_bytes());
    format!("rn_{:x}", hasher.finalize())
}
