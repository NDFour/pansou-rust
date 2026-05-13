use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub log_file: String,
    pub concurrency: usize,
    pub cache_ttl: u64,
    pub max_cache_size: usize,
    pub channels: Vec<String>,
    #[serde(default)]
    pub post_search_endpoint: String,
    // SEO 相关
    #[serde(default)]
    pub domain: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8888,
            log_level: "info".to_string(),
            log_file: String::new(),
            concurrency: 3,
            cache_ttl: 300, // 5 minutes
            max_cache_size: 512,
            channels: vec!["tgsearchers6".to_string()],
            post_search_endpoint: String::new(),
            domain: String::new(),
        }
    }
}

impl AppConfig {
    pub fn from_file() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        let mut loaded = None;
        if let Some(dir) = exe_dir {
            let config_path = dir.join("config.yaml");
            if config_path.exists() {
                match std::fs::read_to_string(&config_path) {
                    Ok(content) => match serde_yml::from_str::<Self>(&content) {
                        Ok(config) => {
                            loaded = Some((config_path, config));
                        }
                        Err(e) => {
                            eprintln!("解析配置文件失败: {}: {}, 使用默认配置", config_path.display(), e);
                        }
                    },
                    Err(e) => {
                        eprintln!("读取配置文件失败: {}: {}, 使用默认配置", config_path.display(), e);
                    }
                }
            } else {
                eprintln!("配置文件不存在: {}, 使用默认配置", config_path.display());
            }
        }

        loaded.map(|(_, c)| c).unwrap_or_default()
    }
}
