use serde::Deserialize;
use tracing::{info, warn};

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub channels: Vec<String>,
    pub go_compat_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8888,
            channels: vec!["tgsearchers6".to_string()],
            go_compat_url: None,
        }
    }
}

impl AppConfig {
    pub fn from_file() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        if let Some(dir) = exe_dir {
            let config_path = dir.join("config.yaml");
            if config_path.exists() {
                match std::fs::read_to_string(&config_path) {
                    Ok(content) => match serde_yml::from_str::<Self>(&content) {
                        Ok(config) => {
                            info!("加载配置文件: {}", config_path.display());
                            return config;
                        }
                        Err(e) => {
                            warn!("解析配置文件失败: {}: {}, 使用默认配置", config_path.display(), e);
                        }
                    },
                    Err(e) => {
                        warn!("读取配置文件失败: {}: {}, 使用默认配置", config_path.display(), e);
                    }
                }
            } else {
                warn!("配置文件不存在: {}, 使用默认配置: {}", config_path.display(), config_path.display());
            }
        }

        info!("使用默认配置");
        Self::default()
    }
}
