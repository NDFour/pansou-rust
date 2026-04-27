use std::{collections::HashMap, env};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub auth_enabled: bool,
    pub auth_users: HashMap<String, String>,
    pub auth_token_expiry_hours: i64,
    pub auth_jwt_secret: String,
    pub channels: Vec<String>,
    pub go_compat_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8888);
        let auth_enabled = matches!(
            env::var("AUTH_ENABLED").ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE")
        );
        let auth_users = parse_auth_users(env::var("AUTH_USERS").unwrap_or_default());
        let auth_token_expiry_hours = env::var("AUTH_TOKEN_EXPIRY")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(24);
        let auth_jwt_secret = env::var("AUTH_JWT_SECRET")
            .unwrap_or_else(|_| format!("pansou-rust-secret-{}", chrono::Utc::now().timestamp()));
        let channels = parse_list(env::var("CHANNELS").unwrap_or_else(|_| "tgsearchers6".to_string()));
        let go_compat_url = env::var("GO_COMPAT_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        Self {
            port,
            auth_enabled,
            auth_users,
            auth_token_expiry_hours,
            auth_jwt_secret,
            channels,
            go_compat_url,
        }
    }
}

fn parse_list(input: String) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_auth_users(input: String) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for item in input.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let parts: Vec<&str> = item.splitn(2, ':').collect();
        if parts.len() == 2 {
            let user = parts[0].trim();
            let pass = parts[1].trim();
            if !user.is_empty() && !pass.is_empty() {
                out.insert(user.to_string(), pass.to_string());
            }
        }
    }
    out
}
