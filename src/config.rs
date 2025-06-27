use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::error::{ConfigError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub feeds: HashMap<String, String>,
    pub settings: Settings,
    #[serde(default)]
    pub fuse: FilesystemConfig,
    #[serde(default)]
    pub cache: CacheSettings,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    
    #[serde(default = "default_cache_duration")]
    pub cache_duration: u64,
    
    #[serde(default = "default_max_articles")]
    pub max_articles: usize,
    
    #[serde(default = "default_concurrent_fetches")]
    pub concurrent_fetches: usize,
    
    #[serde(default = "default_article_content")]
    pub article_content: bool,
    
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: usize,
    
    #[serde(default = "default_max_article_size")]
    pub max_article_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemConfig {
    #[serde(default = "default_mount_options")]
    pub mount_options: Vec<String>,
    
    #[serde(default = "default_file_permissions")]
    pub file_permissions: u32,
    
    #[serde(default = "default_dir_permissions")]
    pub dir_permissions: u32,
    
    #[serde(default)]
    pub allow_other: bool,
    
    #[serde(default = "default_auto_unmount")]
    pub auto_unmount: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    #[serde(default = "default_max_size_mb")]
    pub max_size_mb: usize,
    
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    
    #[serde(default)]
    pub log_to_file: bool,
    
    #[serde(default = "default_log_file")]
    pub log_file: String,
    
    #[serde(default)]
    pub json_format: bool,
    
    #[serde(default)]
    pub console: bool,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|_| ConfigError::NotFound(path.as_ref().display().to_string()))?;
        
        let config: Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    pub fn load_with_env<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = Self::load(path)?;
        config.apply_env_overrides();
        Ok(config)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Invalid(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        // Don't require feeds to be configured for basic validation
        for (name, url) in &self.feeds {
            if name.is_empty() {
                return Err(ConfigError::Invalid("Feed name cannot be empty".to_string()).into());
            }
            
            url::Url::parse(url)
                .map_err(|_| ConfigError::InvalidUrl(url.clone()))?;
        }
        
        if self.settings.refresh_interval == 0 {
            return Err(ConfigError::Invalid("Refresh interval must be greater than 0".to_string()).into());
        }
        
        if self.settings.max_articles == 0 {
            return Err(ConfigError::Invalid("Max articles must be greater than 0".to_string()).into());
        }
        
        Ok(())
    }
    
    fn apply_env_overrides(&mut self) {
        if let Ok(interval) = std::env::var("RSS_FUSE_REFRESH_INTERVAL") {
            if let Ok(val) = interval.parse() {
                self.settings.refresh_interval = val;
            }
        }
        
        if let Ok(level) = std::env::var("RSS_FUSE_LOG_LEVEL") {
            self.logging.level = level;
        }
        
        if let Ok(max_articles) = std::env::var("RSS_FUSE_MAX_ARTICLES") {
            if let Ok(val) = max_articles.parse() {
                self.settings.max_articles = val;
            }
        }
    }
    
    pub fn default() -> Self {
        Self {
            feeds: HashMap::new(),
            settings: Settings::default(),
            fuse: FilesystemConfig::default(),
            cache: CacheSettings::default(),
            logging: LoggingConfig::default(),
        }
    }
    
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join("rss-fuse"))
            .ok_or_else(|| ConfigError::Invalid("Could not determine config directory".to_string()).into())
    }
    
    pub fn data_dir() -> Result<PathBuf> {
        dirs::data_dir()
            .map(|dir| dir.join("rss-fuse"))
            .ok_or_else(|| ConfigError::Invalid("Could not determine data directory".to_string()).into())
    }
    
    pub fn cache_dir() -> Result<PathBuf> {
        dirs::cache_dir()
            .map(|dir| dir.join("rss-fuse"))
            .ok_or_else(|| ConfigError::Invalid("Could not determine cache directory".to_string()).into())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval: default_refresh_interval(),
            cache_duration: default_cache_duration(),
            max_articles: default_max_articles(),
            concurrent_fetches: default_concurrent_fetches(),
            article_content: default_article_content(),
            user_agent: default_user_agent(),
            timeout: default_timeout(),
            retry_attempts: default_retry_attempts(),
            max_article_size: default_max_article_size(),
        }
    }
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            mount_options: default_mount_options(),
            file_permissions: default_file_permissions(),
            dir_permissions: default_dir_permissions(),
            allow_other: false,
            auto_unmount: default_auto_unmount(),
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            max_size_mb: default_max_size_mb(),
            cleanup_interval: default_cleanup_interval(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            log_to_file: false,
            log_file: default_log_file(),
            json_format: false,
            console: false,
        }
    }
}

fn default_refresh_interval() -> u64 { 300 }
fn default_cache_duration() -> u64 { 3600 }
fn default_max_articles() -> usize { 100 }
fn default_concurrent_fetches() -> usize { 5 }
fn default_article_content() -> bool { true }
fn default_user_agent() -> String { 
    format!("RSS-FUSE/{}", env!("CARGO_PKG_VERSION"))
}
fn default_timeout() -> u64 { 30 }
fn default_retry_attempts() -> usize { 3 }
fn default_max_article_size() -> usize { 1024 * 1024 } // 1MB

fn default_mount_options() -> Vec<String> {
    vec!["ro".to_string(), "auto_unmount".to_string()]
}
fn default_file_permissions() -> u32 { 0o644 }
fn default_dir_permissions() -> u32 { 0o755 }
fn default_auto_unmount() -> bool { true }

fn default_log_level() -> String { "info".to_string() }
fn default_max_size_mb() -> usize { 100 }
fn default_cleanup_interval() -> u64 { 300 }
fn default_log_file() -> String { "logs/rss-fuse.log".to_string() }