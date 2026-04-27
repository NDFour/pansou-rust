use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub username: String,
    pub exp: i64,
}

pub fn generate_token(username: &str, secret: &str, expiry_hours: i64) -> anyhow::Result<(String, i64)> {
    let expires_at = (Utc::now() + Duration::hours(expiry_hours)).timestamp();
    let claims = Claims {
        username: username.to_string(),
        exp: expires_at,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok((token, expires_at))
}

pub fn validate_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if !state.config.auth_enabled {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();
    if path.starts_with("/api/auth/login") || path.starts_with("/api/auth/logout") || path.starts_with("/api/health")
    {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if auth_header.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"未授权：缺少认证令牌","code":"AUTH_TOKEN_MISSING"})),
        ));
    }
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"未授权：令牌格式错误","code":"AUTH_TOKEN_INVALID_FORMAT"})),
        ))?;
    let claims = validate_token(token, &state.config.auth_jwt_secret).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"未授权：令牌无效或已过期","code":"AUTH_TOKEN_INVALID"})),
        )
    })?;

    req.extensions_mut().insert(claims.username);
    Ok(next.run(req).await)
}
