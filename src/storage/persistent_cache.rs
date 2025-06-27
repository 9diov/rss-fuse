use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use crate::feed::{Feed, Article};
use crate::error::{Error, Result};
use crate::storage::cache::CacheEntry;

/// Serializable version of CacheEntry for disk storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCacheEntry<T> {
    pub data: T,
    pub created_at: u64, // Unix timestamp
    pub expires_at: u64, // Unix timestamp
    pub access_count: u64,
    pub last_accessed: u64, // Unix timestamp
}

impl<T> From<CacheEntry<T>> for SerializableCacheEntry<T> {
    fn from(entry: CacheEntry<T>) -> Self {
        Self {
            data: entry.data,
            created_at: entry.created_at.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            expires_at: entry.expires_at.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            access_count: entry.access_count,
            last_accessed: entry.last_accessed.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
        }
    }
}

impl<T> From<SerializableCacheEntry<T>> for CacheEntry<T> {
    fn from(entry: SerializableCacheEntry<T>) -> Self {
        Self {
            data: entry.data,
            created_at: SystemTime::UNIX_EPOCH + Duration::from_secs(entry.created_at),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(entry.expires_at),
            access_count: entry.access_count,
            last_accessed: SystemTime::UNIX_EPOCH + Duration::from_secs(entry.last_accessed),
        }
    }
}

/// Persistent cache data structure for serialization
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentCacheData {
    pub feeds: HashMap<String, SerializableCacheEntry<Feed>>,
    pub articles: HashMap<String, SerializableCacheEntry<Article>>,
    pub cache_version: u32,
    pub saved_at: u64, // Unix timestamp
}

impl Default for PersistentCacheData {
    fn default() -> Self {
        Self {
            feeds: HashMap::new(),
            articles: HashMap::new(),
            cache_version: 1,
            saved_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
        }
    }
}

/// Configuration for persistent cache
#[derive(Debug, Clone)]
pub struct PersistentCacheConfig {
    pub cache_dir: PathBuf,
    pub max_age_days: u64,
    pub max_size_mb: u64,
    pub enable_compression: bool,
}

impl Default for PersistentCacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("rss-fuse"),
            max_age_days: 7, // Keep cache for 1 week
            max_size_mb: 100,
            enable_compression: true,
        }
    }
}

/// Persistent cache manager that saves/loads cache to/from disk
pub struct PersistentCache {
    config: PersistentCacheConfig,
    cache_file: PathBuf,
}

impl PersistentCache {
    pub fn new(config: PersistentCacheConfig) -> Result<Self> {
        // Ensure cache directory exists
        if !config.cache_dir.exists() {
            fs::create_dir_all(&config.cache_dir)
                .map_err(|e| Error::Storage(format!(
                    "Failed to create cache directory '{}': {}", 
                    config.cache_dir.display(), e
                )))?;
        }

        let cache_file = config.cache_dir.join("feeds_cache.json");

        Ok(Self {
            config,
            cache_file,
        })
    }

    /// Load cache data from disk
    pub fn load(&self) -> Result<Option<PersistentCacheData>> {
        if !self.cache_file.exists() {
            tracing::debug!("Cache file does not exist: {}", self.cache_file.display());
            return Ok(None);
        }

        let file_content = fs::read_to_string(&self.cache_file)
            .map_err(|e| Error::Storage(format!(
                "Failed to read cache file '{}': {}", 
                self.cache_file.display(), e
            )))?;

        let cache_data: PersistentCacheData = serde_json::from_str(&file_content)
            .map_err(|e| Error::Serialization(e))?;

        // Check if cache is too old
        let cache_age = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - cache_data.saved_at;

        let max_age_seconds = self.config.max_age_days * 24 * 60 * 60;
        if cache_age > max_age_seconds {
            tracing::info!("Cache file is too old ({} days), ignoring", cache_age / (24 * 60 * 60));
            return Ok(None);
        }

        // Filter out expired entries
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default().as_secs();

        let mut filtered_data = cache_data;
        filtered_data.feeds.retain(|_, entry| entry.expires_at > now);
        filtered_data.articles.retain(|_, entry| entry.expires_at > now);

        tracing::info!("Loaded cache: {} feeds, {} articles", 
                      filtered_data.feeds.len(), filtered_data.articles.len());

        Ok(Some(filtered_data))
    }

    /// Save cache data to disk
    pub fn save(&self, feeds: &HashMap<String, CacheEntry<Feed>>, 
                articles: &HashMap<String, CacheEntry<Arc<Article>>>) -> Result<()> {
        
        // Convert to serializable format
        let feed_entries: HashMap<String, SerializableCacheEntry<Feed>> = feeds
            .iter()
            .filter(|(_, entry)| !entry.is_expired())
            .map(|(k, v)| (k.clone(), v.clone().into()))
            .collect();

        let article_entries: HashMap<String, SerializableCacheEntry<Article>> = articles
            .iter()
            .filter(|(_, entry)| !entry.is_expired())
            .map(|(k, v)| (k.clone(), SerializableCacheEntry {
                data: (*v.data).clone(), // Dereference Arc<Article>
                created_at: v.created_at.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                expires_at: v.expires_at.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                access_count: v.access_count,
                last_accessed: v.last_accessed.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
            }))
            .collect();

        let cache_data = PersistentCacheData {
            feeds: feed_entries,
            articles: article_entries,
            cache_version: 1,
            saved_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
        };

        // Serialize to JSON
        let json_content = serde_json::to_string_pretty(&cache_data)
            .map_err(|e| Error::Serialization(e))?;

        // Write to temporary file first, then rename (atomic operation)
        let temp_file = self.cache_file.with_extension("tmp");
        fs::write(&temp_file, json_content)
            .map_err(|e| Error::Storage(format!(
                "Failed to write cache to '{}': {}", 
                temp_file.display(), e
            )))?;

        fs::rename(&temp_file, &self.cache_file)
            .map_err(|e| Error::Storage(format!(
                "Failed to rename cache file '{}' to '{}': {}", 
                temp_file.display(), self.cache_file.display(), e
            )))?;

        tracing::info!("Saved cache: {} feeds, {} articles to {}", 
                      cache_data.feeds.len(), cache_data.articles.len(),
                      self.cache_file.display());

        Ok(())
    }

    /// Check current cache file size
    pub fn cache_size_mb(&self) -> f64 {
        if let Ok(metadata) = fs::metadata(&self.cache_file) {
            metadata.len() as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }

    /// Clean up old cache files and check size limits
    pub fn cleanup(&self) -> Result<()> {
        // Check file size
        if self.cache_size_mb() > self.config.max_size_mb as f64 {
            tracing::warn!("Cache file size ({:.1} MB) exceeds limit ({} MB), removing cache",
                          self.cache_size_mb(), self.config.max_size_mb);
            if self.cache_file.exists() {
                fs::remove_file(&self.cache_file)
                    .map_err(|e| Error::Storage(format!(
                        "Failed to remove oversized cache file '{}': {}", 
                        self.cache_file.display(), e
                    )))?
            }
        }

        // Clean up temporary files
        if let Ok(entries) = fs::read_dir(&self.config.cache_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".tmp") {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }

        Ok(())
    }

    /// Get cache file path for debugging
    pub fn cache_path(&self) -> &Path {
        &self.cache_file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::feed::{Article, ParsedArticle};
    use chrono::Utc;

    fn create_test_article(title: &str) -> Article {
        let parsed = ParsedArticle {
            title: title.to_string(),
            link: format!("https://example.com/{}", title.to_lowercase()),
            description: Some(format!("Description for {}", title)),
            content: None,
            author: Some("Test Author".to_string()),
            published: Some(Utc::now()),
            guid: Some(format!("guid-{}", title.to_lowercase())),
            categories: vec!["test".to_string()],
        };
        Article::new(parsed, "test-feed")
    }

    fn create_test_feed(name: &str, article_count: usize) -> Feed {
        let articles = (0..article_count)
            .map(|i| create_test_article(&format!("Article {}", i)))
            .collect();

        Feed {
            name: name.to_string(),
            url: format!("https://example.com/{}.rss", name),
            title: Some(format!("Test Feed {}", name)),
            description: Some(format!("Description for {}", name)),
            last_updated: Some(Utc::now()),
            articles,
            status: crate::feed::FeedStatus::Active,
        }
    }

    #[test]
    fn test_persistent_cache_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistentCacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let cache = PersistentCache::new(config).unwrap();

        // Create test data
        let mut feeds = HashMap::new();
        let mut articles = HashMap::new();

        let test_feed = create_test_feed("tech-news", 3);
        let feed_entry = CacheEntry::new(test_feed, Duration::from_secs(3600));
        feeds.insert("tech-news".to_string(), feed_entry);

        let test_article = create_test_article("Test Article");
        let article_entry = CacheEntry::new(Arc::new(test_article), Duration::from_secs(3600));
        articles.insert("test-id".to_string(), article_entry);

        // Save cache
        cache.save(&feeds, &articles).unwrap();

        // Load cache
        let loaded_data = cache.load().unwrap().unwrap();

        assert_eq!(loaded_data.feeds.len(), 1);
        assert_eq!(loaded_data.articles.len(), 1);
        assert!(loaded_data.feeds.contains_key("tech-news"));
        assert!(loaded_data.articles.contains_key("test-id"));
    }

    #[test]
    fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistentCacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_age_days: 0, // Expire immediately
            ..Default::default()
        };

        let cache = PersistentCache::new(config).unwrap();

        // Create test data with short TTL
        let mut feeds = HashMap::new();
        let test_feed = create_test_feed("tech-news", 1);
        let feed_entry = CacheEntry::new(test_feed, Duration::from_secs(1));
        feeds.insert("tech-news".to_string(), feed_entry);

        // Save cache
        cache.save(&feeds, &HashMap::new()).unwrap();

        // Sleep to ensure expiration
        std::thread::sleep(Duration::from_secs(2));

        // Load cache - should be empty due to expiration
        let loaded_data = cache.load().unwrap();
        assert!(loaded_data.is_none() || loaded_data.unwrap().feeds.is_empty());
    }
}