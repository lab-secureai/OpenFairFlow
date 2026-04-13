use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::collections::HashSet;
#[cfg(feature = "server")]
use std::sync::{LazyLock, Mutex};

#[cfg(feature = "server")]
static SESSIONS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

#[cfg(feature = "server")]
fn generate_session_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(feature = "server")]
pub fn create_session() -> String {
    let token = generate_session_token();
    SESSIONS.lock().unwrap().insert(token.clone());
    token
}

#[cfg(feature = "server")]
pub fn validate_session(token: &str) -> bool {
    SESSIONS.lock().unwrap().contains(token)
}

#[cfg(feature = "server")]
pub fn remove_session(token: &str) {
    SESSIONS.lock().unwrap().remove(token);
}

#[cfg(feature = "server")]
fn extract_session_from_cookie(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("session_token="))
        .map(|s| s["session_token=".len()..].to_string())
}

// ── Server functions ──────────────────────────────────────────────────────

#[post("/api/auth/login")]
pub async fn login_server(username: String, password: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let expected_user = std::env::var("AUTH_USERNAME").unwrap_or_else(|_| "admin".to_string());
        let expected_pass = std::env::var("AUTH_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        if username == expected_user && password == expected_pass {
            let token = create_session();

            let cookie =
                format!("session_token={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400");

            if let Some(ctx) = dioxus_fullstack::FullstackContext::current() {
                ctx.add_response_header(
                    axum::http::header::SET_COOKIE,
                    axum::http::HeaderValue::from_str(&cookie)
                        .map_err(|e| ServerFnError::new(format!("Header error: {e}")))?,
                );
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (username, password);
        Ok(false)
    }
}

#[post("/api/auth/check")]
pub async fn check_auth_server() -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if let Some(ctx) = dioxus_fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            let valid = parts
                .headers
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(extract_session_from_cookie)
                .is_some_and(|t| validate_session(&t));
            return Ok(valid);
        }
        Ok(false)
    }
    #[cfg(not(feature = "server"))]
    {
        Ok(false)
    }
}

#[post("/api/auth/logout")]
pub async fn logout_server() -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        if let Some(ctx) = dioxus_fullstack::FullstackContext::current() {
            // Remove session from store
            let token = {
                let parts = ctx.parts_mut();
                parts
                    .headers
                    .get("cookie")
                    .and_then(|v| v.to_str().ok())
                    .and_then(extract_session_from_cookie)
            };
            if let Some(token) = token {
                remove_session(&token);
            }

            // Clear cookie
            ctx.add_response_header(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_static(
                    "session_token=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
                ),
            );
        }
    }
    Ok(())
}

// ── Middleware ─────────────────────────────────────────────────────────────

#[cfg(feature = "server")]
pub async fn auth_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let path = req.uri().path().to_string();

    // Allow login page, auth API, and static assets through
    if path == "/login"
        || path.starts_with("/api/auth/")
        || path.starts_with("/assets/")
        || path.starts_with("/_dioxus/")
        || path.starts_with("/public/")
        || path.starts_with("/wasm/")
    {
        return next.run(req).await;
    }

    // Check for session cookie
    let has_valid_session = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(extract_session_from_cookie)
        .is_some_and(|token| validate_session(&token));

    if has_valid_session {
        return next.run(req).await;
    }

    // API requests get 401, page requests get redirected
    if path.starts_with("/api/") {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    axum::response::Redirect::temporary("/login").into_response()
}
