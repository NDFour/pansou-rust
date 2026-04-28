use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub channels: Vec<String>,
    pub go_compat_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8888);
        let channels = parse_list(env::var("CHANNELS").unwrap_or_else(|_| "tgsearchers6".to_string()));
        let go_compat_url = env::var("GO_COMPAT_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        Self {
            port,
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
