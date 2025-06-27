use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::feed::{Feed, Article};
use crate::error::{Error, Result};
use crate::storage::persistent_cache::{PersistentCache, PersistentCacheConfig};

/// Cache entry with expiration tracking
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub access_count: u64,
    pub last_accessed: SystemTime,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        let now = SystemTime::now();
        Self {
            data,
            created_at: now,
            expires_at: now + ttl,
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    pub fn access(&mut self) -> &T {
        self.access_count += 1;
        self.last_accessed = SystemTime::now();
        &self.data
    }

    pub fn age(&self) -> Duration {
        SystemTime::now().duration_since(self.created_at)
            .unwrap_or_default()
    }
}

/// Cache statistics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
    pub total_entries: usize,
    pub memory_usage_bytes: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    pub fn record_expiration(&mut self) {
        self.expirations += 1;
    }
}

/// Configuration for cache behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub default_ttl: Duration,
    pub cleanup_interval: Duration,
    pub max_memory_mb: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_memory_mb: 100,
        }
    }
}

/// Memory-based cache for articles with LRU eviction
#[derive(Clone)]
pub struct ArticleCache {
    cache: Arc<RwLock<LruCache<String, CacheEntry<Arc<Article>>>>>,
    stats: Arc<RwLock<CacheStats>>,
    config: CacheConfig,
}

impl ArticleCache {
    pub fn new(config: CacheConfig) -> Self {
        let capacity = NonZeroUsize::new(config.max_entries)
            .unwrap_or(NonZeroUsize::new(1000).unwrap());
        
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(CacheConfig {
            max_entries: capacity,
            ..Default::default()
        })
    }

    /// Get an article from cache
    pub fn get(&self, article_id: &str) -> Option<Arc<Article>> {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        if let Some(entry) = cache.get_mut(article_id) {
            if entry.is_expired() {
                cache.pop(article_id);
                stats.record_expiration();
                stats.record_miss();
                stats.total_entries = cache.len();
                return None;
            }

            stats.record_hit();
            Some(Arc::clone(entry.access()))
        } else {
            stats.record_miss();
            None
        }
    }

    /// Put an article into cache
    pub fn put(&self, article_id: String, article: Arc<Article>) -> Result<()> {
        let entry = CacheEntry::new(article, self.config.default_ttl);
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        if let Some(_) = cache.put(article_id, entry) {
            stats.record_eviction();
        }

        stats.total_entries = cache.len();
        Ok(())
    }

    /// Put an article with custom TTL
    pub fn put_with_ttl(&self, article_id: String, article: Arc<Article>, ttl: Duration) -> Result<()> {
        let entry = CacheEntry::new(article, ttl);
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        if let Some(_) = cache.put(article_id, entry) {
            stats.record_eviction();
        }

        stats.total_entries = cache.len();
        Ok(())
    }

    /// Remove an article from cache
    pub fn remove(&self, article_id: &str) -> Option<Arc<Article>> {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        let result = cache.pop(article_id).map(|entry| entry.data);
        stats.total_entries = cache.len();
        result
    }

    /// Clear all entries from cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        
        cache.clear();
        stats.total_entries = 0;
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();
        let mut expired_keys = Vec::new();

        // Find expired keys
        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        let count = expired_keys.len();
        for key in expired_keys {
            cache.pop(&key);
            stats.record_expiration();
        }

        stats.total_entries = cache.len();
        count
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Get cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Check if cache contains a key
    pub fn contains(&self, article_id: &str) -> bool {
        let cache = self.cache.read();
        cache.contains(article_id)
    }

    /// Get number of entries in cache
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }

    /// Get all cache keys (for debugging/testing)
    pub fn keys(&self) -> Vec<String> {
        let cache = self.cache.read();
        cache.iter().map(|(k, _)| k.clone()).collect()
    }
}

/// Feed cache for storing complete feed metadata
#[derive(Clone)]
pub struct FeedCache {
    feeds: Arc<RwLock<HashMap<String, CacheEntry<Feed>>>>,
    stats: Arc<RwLock<CacheStats>>,
    config: CacheConfig,
}

impl FeedCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            feeds: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }

    /// Get a feed from cache
    pub fn get(&self, feed_name: &str) -> Option<Feed> {
        let mut feeds = self.feeds.write();
        let mut stats = self.stats.write();

        if let Some(entry) = feeds.get_mut(feed_name) {
            if entry.is_expired() {
                feeds.remove(feed_name);
                stats.record_expiration();
                stats.record_miss();
                stats.total_entries = feeds.len();
                return None;
            }

            stats.record_hit();
            Some(entry.access().clone())
        } else {
            stats.record_miss();
            None
        }
    }

    /// Put a feed into cache
    pub fn put(&self, feed_name: String, feed: Feed) -> Result<()> {
        let entry = CacheEntry::new(feed, self.config.default_ttl);
        let mut feeds = self.feeds.write();
        let mut stats = self.stats.write();

        feeds.insert(feed_name, entry);
        stats.total_entries = feeds.len();
        Ok(())
    }

    /// Remove a feed from cache
    pub fn remove(&self, feed_name: &str) -> Option<Feed> {
        let mut feeds = self.feeds.write();
        let mut stats = self.stats.write();

        let result = feeds.remove(feed_name).map(|entry| entry.data);
        stats.total_entries = feeds.len();
        result
    }

    /// Clear all feeds from cache
    pub fn clear(&self) {
        let mut feeds = self.feeds.write();
        let mut stats = self.stats.write();
        
        feeds.clear();
        stats.total_entries = 0;
    }

    /// Clean up expired feeds
    pub fn cleanup_expired(&self) -> usize {
        let mut feeds = self.feeds.write();
        let mut stats = self.stats.write();
        let mut expired_keys = Vec::new();

        // Find expired keys
        for (key, entry) in feeds.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        let count = expired_keys.len();
        for key in expired_keys {
            feeds.remove(&key);
            stats.record_expiration();
        }

        stats.total_entries = feeds.len();
        count
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Get all feed names
    pub fn feed_names(&self) -> Vec<String> {
        let feeds = self.feeds.read();
        feeds.keys().cloned().collect()
    }

    /// Get number of feeds in cache
    pub fn len(&self) -> usize {
        self.feeds.read().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.feeds.read().is_empty()
    }
}

/// Combined cache manager for both articles and feeds
#[derive(Clone)]
pub struct CacheManager {
    pub articles: ArticleCache,
    pub feeds: FeedCache,
    persistent_cache: Option<Arc<PersistentCache>>,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        let article_config = CacheConfig {
            max_entries: config.max_entries,
            ..config.clone()
        };

        let feed_config = CacheConfig {
            max_entries: config.max_entries / 10, // Fewer feeds than articles
            ..config
        };

        Self {
            articles: ArticleCache::new(article_config),
            feeds: FeedCache::new(feed_config),
            persistent_cache: None,
        }
    }

    /// Create a new cache manager with persistent storage
    pub fn with_persistence(config: CacheConfig, persistent_config: PersistentCacheConfig) -> Result<Self> {
        let article_config = CacheConfig {
            max_entries: config.max_entries,
            ..config.clone()
        };

        let feed_config = CacheConfig {
            max_entries: config.max_entries / 10, // Fewer feeds than articles
            ..config
        };

        let persistent_cache = PersistentCache::new(persistent_config)?;

        let mut manager = Self {
            articles: ArticleCache::new(article_config),
            feeds: FeedCache::new(feed_config),
            persistent_cache: Some(Arc::new(persistent_cache)),
        };

        // Load existing cache from disk
        manager.load_from_disk()?;

        Ok(manager)
    }

    /// Load cache data from disk
    pub fn load_from_disk(&mut self) -> Result<()> {
        if let Some(ref persistent_cache) = self.persistent_cache {
            if let Some(cache_data) = persistent_cache.load()? {
                tracing::info!("Loading persistent cache: {} feeds, {} articles", 
                              cache_data.feeds.len(), cache_data.articles.len());

                // Load feeds into cache
                for (feed_name, entry_data) in cache_data.feeds {
                    let cache_entry: CacheEntry<Feed> = entry_data.into();
                    if !cache_entry.is_expired() {
                        let _ = self.feeds.put(feed_name, cache_entry.data);
                    }
                }

                // Load articles into cache
                for (article_id, entry_data) in cache_data.articles {
                    let cache_entry: CacheEntry<Article> = entry_data.into();
                    if !cache_entry.is_expired() {
                        let _ = self.articles.put(article_id, Arc::new(cache_entry.data));
                    }
                }

                tracing::info!("Loaded persistent cache successfully");
            } else {
                tracing::debug!("No persistent cache found or cache expired");
            }
        }
        Ok(())
    }

    /// Save cache data to disk
    pub fn save_to_disk(&self) -> Result<()> {
        if let Some(ref persistent_cache) = self.persistent_cache {
            // Get current cache contents
            let feeds = {
                let feeds = self.feeds.feeds.read();
                feeds.clone()
            };

            let articles = {
                let articles = self.articles.cache.read();
                // Convert LruCache to HashMap for persistence
                let mut article_map = HashMap::new();
                for (k, v) in articles.iter() {
                    article_map.insert(k.clone(), v.clone());
                }
                article_map
            };

            tracing::info!("Saving cache to disk: {} feeds, {} articles", 
                         feeds.len(), articles.len());
            persistent_cache.save(&feeds, &articles)?;
            tracing::info!("Cache saved successfully to: {}", 
                         persistent_cache.cache_path().display());
        } else {
            tracing::warn!("No persistent cache configured - cannot save to disk");
        }
        Ok(())
    }

    /// Enable automatic cache persistence
    pub fn enable_auto_save(&self) {
        if self.persistent_cache.is_some() {
            let manager = self.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(300)); // Save every 5 minutes
                loop {
                    interval.tick().await;
                    if let Err(e) = manager.save_to_disk() {
                        tracing::warn!("Failed to auto-save cache: {}", e);
                    }
                }
            });
        }
    }

    /// Cleanup expired entries in both caches
    pub fn cleanup_expired(&self) -> (usize, usize) {
        let article_expired = self.articles.cleanup_expired();
        let feed_expired = self.feeds.cleanup_expired();
        (article_expired, feed_expired)
    }

    /// Get combined statistics
    pub fn combined_stats(&self) -> (CacheStats, CacheStats) {
        (self.articles.stats(), self.feeds.stats())
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.articles.clear();
        self.feeds.clear();
    }

    /// Get total memory usage estimate
    pub fn estimated_memory_usage(&self) -> usize {
        // Rough estimate - in production this would be more sophisticated
        self.articles.len() * 1024 + self.feeds.len() * 512
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{ParsedArticle, FeedStatus};
    use chrono::Utc;

    fn create_test_article(id: &str) -> Arc<Article> {
        let parsed = ParsedArticle {
            title: format!("Test Article {}", id),
            link: format!("https://example.com/{}", id),
            description: Some("Test description".to_string()),
            content: Some("Test content".to_string()),
            author: Some("Test Author".to_string()),
            published: Some(Utc::now()),
            guid: Some(id.to_string()),
            categories: vec!["test".to_string()],
        };
        Arc::new(Article::new(parsed, "test-feed"))
    }

    fn create_test_feed(name: &str) -> Feed {
        Feed {
            name: name.to_string(),
            url: format!("https://example.com/{}.xml", name),
            title: Some(format!("Test Feed {}", name)),
            description: Some("Test feed description".to_string()),
            last_updated: Some(Utc::now()),
            articles: vec![],
            status: FeedStatus::Active,
        }
    }

    #[test]
    fn test_article_cache_basic_operations() {
        let cache = ArticleCache::with_capacity(10);
        let article = create_test_article("test1");

        // Test put and get
        cache.put("test1".to_string(), article.clone()).unwrap();
        let retrieved = cache.get("test1").unwrap();
        assert_eq!(retrieved.id, article.id);

        // Test cache hit
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_article_cache_miss() {
        let cache = ArticleCache::with_capacity(10);
        
        let result = cache.get("nonexistent");
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_article_cache_eviction() {
        let cache = ArticleCache::with_capacity(2);
        
        // Fill cache beyond capacity
        cache.put("article1".to_string(), create_test_article("1")).unwrap();
        cache.put("article2".to_string(), create_test_article("2")).unwrap();
        cache.put("article3".to_string(), create_test_article("3")).unwrap();

        // Should only have 2 entries
        assert_eq!(cache.len(), 2);
        
        // article1 should have been evicted
        assert!(cache.get("article1").is_none());
        assert!(cache.get("article2").is_some() || cache.get("article3").is_some());
    }

    #[test]
    fn test_cache_expiration() {
        let config = CacheConfig {
            default_ttl: Duration::from_millis(10), // Very short TTL for testing
            ..Default::default()
        };
        let cache = ArticleCache::new(config);
        
        cache.put("test".to_string(), create_test_article("test")).unwrap();
        
        // Should be available immediately
        assert!(cache.get("test").is_some());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));
        
        // Should be expired now
        assert!(cache.get("test").is_none());
        
        let stats = cache.stats();
        assert_eq!(stats.expirations, 1);
    }

    #[test]
    fn test_feed_cache_basic_operations() {
        let cache = FeedCache::new(CacheConfig::default());
        let feed = create_test_feed("test-feed");

        cache.put("test-feed".to_string(), feed.clone()).unwrap();
        let retrieved = cache.get("test-feed").unwrap();
        assert_eq!(retrieved.name, feed.name);
    }

    #[test]
    fn test_cache_manager() {
        let manager = CacheManager::default();
        
        // Test article cache
        let article = create_test_article("test");
        manager.articles.put("test".to_string(), article.clone()).unwrap();
        assert!(manager.articles.get("test").is_some());
        
        // Test feed cache
        let feed = create_test_feed("test-feed");
        manager.feeds.put("test-feed".to_string(), feed.clone()).unwrap();
        assert!(manager.feeds.get("test-feed").is_some());
        
        // Test cleanup
        let (article_expired, feed_expired) = manager.cleanup_expired();
        assert_eq!(article_expired, 0); // No expired entries yet
        assert_eq!(feed_expired, 0);
    }

    #[test]
    fn test_cache_entry_access_tracking() {
        let article = create_test_article("test");
        let mut entry = CacheEntry::new(article, Duration::from_secs(3600));
        
        assert_eq!(entry.access_count, 0);
        
        entry.access();
        assert_eq!(entry.access_count, 1);
        
        entry.access();
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        
        assert_eq!(stats.hit_rate(), 0.0);
        
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        
        // 2 hits out of 3 total = 66.67%
        assert!((stats.hit_rate() - 0.6666666666666666).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_cleanup_expired() {
        let config = CacheConfig {
            default_ttl: Duration::from_millis(10),
            ..Default::default()
        };
        let cache = ArticleCache::new(config);
        
        // Add some articles
        cache.put("article1".to_string(), create_test_article("1")).unwrap();
        cache.put("article2".to_string(), create_test_article("2")).unwrap();
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));
        
        // Add a fresh article
        cache.put("article3".to_string(), create_test_article("3")).unwrap();
        
        // Cleanup should remove 2 expired articles
        let expired_count = cache.cleanup_expired();
        assert_eq!(expired_count, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache.get("article3").is_some());
    }
}