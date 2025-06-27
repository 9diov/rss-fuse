use std::sync::Arc;
use std::time::{Duration, Instant};
use async_trait::async_trait;

use crate::feed::{Feed, Article};
use crate::feed::fetcher::FeedFetcher;
use crate::storage::cache::{CacheManager, CacheConfig};
use crate::storage::persistent_cache::PersistentCacheConfig;
use crate::storage::traits::{
    Storage, FeedRepository, ArticleRepository, RepositoryStats, 
    ArticleQuery, ArticleStats, MemoryStorage, StorageConfig
};
use crate::error::{Error, Result};

/// Combined repository implementation with caching and storage
#[derive(Clone)]
pub struct Repository {
    storage: Arc<dyn Storage>,
    cache: CacheManager,
    fetcher: FeedFetcher,
    metrics: Arc<parking_lot::RwLock<RepositoryMetrics>>,
}

#[derive(Debug, Default)]
struct RepositoryMetrics {
    cache_hits: u64,
    cache_misses: u64,
    storage_reads: u64,
    storage_writes: u64,
    feed_refreshes: u64,
    total_response_time_ms: u64,
    operation_count: u64,
}

impl Repository {
    pub fn new(storage: Arc<dyn Storage>, cache_config: CacheConfig) -> Self {
        Self {
            storage,
            cache: CacheManager::new(cache_config),
            fetcher: FeedFetcher::new(),
            metrics: Arc::new(parking_lot::RwLock::new(RepositoryMetrics::default())),
        }
    }

    /// Create repository with persistent cache
    pub fn with_persistent_cache(storage: Arc<dyn Storage>, cache_config: CacheConfig, 
                                persistent_config: PersistentCacheConfig) -> Result<Self> {
        let cache = CacheManager::with_persistence(cache_config, persistent_config)?;
        
        let mut repo = Self {
            storage,
            cache,
            fetcher: FeedFetcher::new(),
            metrics: Arc::new(parking_lot::RwLock::new(RepositoryMetrics::default())),
        };

        // Enable auto-save for persistent cache
        repo.cache.enable_auto_save();

        Ok(repo)
    }

    /// Save cache to disk manually
    pub fn save_cache(&self) -> Result<()> {
        self.cache.save_to_disk()
    }

    pub fn with_memory_storage() -> Self {
        let storage = Arc::new(MemoryStorage::default());
        Self::new(storage, CacheConfig::default())
    }

    pub fn with_custom_storage(storage: Arc<dyn Storage>) -> Self {
        Self::new(storage, CacheConfig::default())
    }

    fn record_operation_time(&self, duration: Duration) {
        let mut metrics = self.metrics.write();
        metrics.total_response_time_ms += duration.as_millis() as u64;
        metrics.operation_count += 1;
    }

    fn record_cache_hit(&self) {
        self.metrics.write().cache_hits += 1;
    }

    fn record_cache_miss(&self) {
        self.metrics.write().cache_misses += 1;
    }

    fn record_storage_read(&self) {
        self.metrics.write().storage_reads += 1;
    }

    fn record_storage_write(&self) {
        self.metrics.write().storage_writes += 1;
    }

    fn record_feed_refresh(&self) {
        self.metrics.write().feed_refreshes += 1;
    }

    async fn get_feed_from_cache_or_storage(&self, name: &str) -> Result<Option<Feed>> {
        let start = Instant::now();
        
        // Try cache first
        if let Some(feed) = self.cache.feeds.get(name) {
            self.record_cache_hit();
            self.record_operation_time(start.elapsed());
            return Ok(Some(feed));
        }
        
        self.record_cache_miss();
        
        // Fall back to storage
        self.record_storage_read();
        let feed = self.storage.get_feed(name).await?;
        
        // Cache the result if found
        if let Some(ref feed) = feed {
            let _ = self.cache.feeds.put(name.to_string(), feed.clone());
        }
        
        self.record_operation_time(start.elapsed());
        Ok(feed)
    }

    async fn store_feed_in_cache_and_storage(&self, feed: Feed) -> Result<()> {
        let start = Instant::now();
        
        // Store in both cache and persistent storage
        let feed_name = feed.name.clone();
        
        // Cache feed
        let _ = self.cache.feeds.put(feed_name.clone(), feed.clone());
        
        // Cache articles
        for article in &feed.articles {
            let _ = self.cache.articles.put(article.id.clone(), Arc::new(article.clone()));
        }
        
        // Store persistently
        self.record_storage_write();
        self.storage.store_feed(&feed).await?;
        
        self.record_operation_time(start.elapsed());
        Ok(())
    }
}

#[async_trait]
impl FeedRepository for Repository {
    async fn get_feed(&self, name: &str) -> Result<Option<Feed>> {
        self.get_feed_from_cache_or_storage(name).await
    }

    async fn save_feed(&self, feed: Feed) -> Result<()> {
        self.store_feed_in_cache_and_storage(feed).await
    }

    async fn update_feed(&self, feed: Feed) -> Result<()> {
        // Same as save for now, but could include update-specific logic
        self.store_feed_in_cache_and_storage(feed).await
    }

    async fn delete_feed(&self, name: &str) -> Result<()> {
        let start = Instant::now();
        
        // Remove from cache
        self.cache.feeds.remove(name);
        
        // Remove articles from cache
        let article_ids = self.storage.list_articles(name).await?;
        for article_id in article_ids {
            self.cache.articles.remove(&article_id);
        }
        
        // Remove from storage
        self.record_storage_write();
        self.storage.remove_feed(name).await?;
        
        self.record_operation_time(start.elapsed());
        Ok(())
    }

    async fn list_feeds(&self) -> Result<Vec<String>> {
        let start = Instant::now();
        self.record_storage_read();
        let result = self.storage.list_feeds().await;
        self.record_operation_time(start.elapsed());
        result
    }

    async fn get_feed_with_articles(&self, name: &str) -> Result<Option<Feed>> {
        // For now, same as get_feed since we store complete feeds
        self.get_feed_from_cache_or_storage(name).await
    }

    async fn refresh_feed(&self, name: &str, url: &str) -> Result<Feed> {
        let start = Instant::now();
        self.record_feed_refresh();
        
        // Fetch fresh feed data
        let parsed_feed = self.fetcher.fetch_feed(url).await
            .map_err(|e| Error::HttpError(format!("Failed to refresh feed {}: {}", name, e)))?;
        
        // Convert to Feed object
        let feed = Feed {
            name: name.to_string(),
            url: url.to_string(),
            title: Some(parsed_feed.title),
            description: parsed_feed.description,
            last_updated: parsed_feed.last_build_date,
            articles: parsed_feed.articles.into_iter()
                .map(|a| Article::new(a, name))
                .collect(),
            status: crate::feed::FeedStatus::Active,
        };
        
        // Store the refreshed feed
        self.store_feed_in_cache_and_storage(feed.clone()).await?;
        
        // Save to disk immediately after refresh
        if let Err(e) = self.save_cache() {
            tracing::warn!("Failed to save cache after feed refresh: {}", e);
        } else {
            tracing::debug!("Cache saved to disk after refreshing feed: {}", name);
        }
        
        self.record_operation_time(start.elapsed());
        Ok(feed)
    }

    /// Load feed with cache-first strategy: return cached content immediately,
    /// then refresh in background and return fresh content if available
    async fn load_feed_cache_first(&self, name: &str, url: &str) -> Result<Option<Feed>> {
        let start = Instant::now();
        
        // First, try to get cached/stored content immediately
        let cached_feed = self.get_feed_from_cache_or_storage(name).await?;
        
        if let Some(ref feed) = cached_feed {
            self.record_operation_time(start.elapsed());
            return Ok(Some(feed.clone()));
        }
        
        // If no cached content, return None (caller can show loading placeholder)
        // Background refresh will be triggered separately
        self.record_operation_time(start.elapsed());
        Ok(None)
    }

    /// Refresh feed in background and update cache/storage
    async fn refresh_feed_background(&self, name: &str, url: &str) -> Result<Option<Feed>> {
        let start = Instant::now();
        
        match self.refresh_feed(name, url).await {
            Ok(feed) => {
                self.record_operation_time(start.elapsed());
                Ok(Some(feed))
            }
            Err(e) => {
                // Log error but don't fail - cached content is still valid
                tracing::warn!("Background refresh failed for feed {}: {}", name, e);
                self.record_operation_time(start.elapsed());
                Ok(None)
            }
        }
    }

    async fn get_stats(&self) -> Result<RepositoryStats> {
        let storage_stats = self.storage.get_stats().await?;
        let cache_stats = self.cache.articles.stats();
        let metrics = self.metrics.read();
        
        let cache_hit_rate = if metrics.cache_hits + metrics.cache_misses > 0 {
            metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64
        } else {
            0.0
        };
        
        let avg_response_time_ms = if metrics.operation_count > 0 {
            metrics.total_response_time_ms as f64 / metrics.operation_count as f64
        } else {
            0.0
        };
        
        Ok(RepositoryStats {
            storage: storage_stats,
            cache: cache_stats,
            cache_hit_rate,
            avg_response_time_ms,
        })
    }
}

#[async_trait]
impl ArticleRepository for Repository {
    async fn get_article(&self, article_id: &str) -> Result<Option<Arc<Article>>> {
        let start = Instant::now();
        
        // Try cache first
        if let Some(article) = self.cache.articles.get(article_id) {
            self.record_cache_hit();
            self.record_operation_time(start.elapsed());
            return Ok(Some(article));
        }
        
        self.record_cache_miss();
        
        // Fall back to storage
        self.record_storage_read();
        let article = self.storage.get_article(article_id).await?;
        
        // Cache the result if found
        if let Some(ref article) = article {
            let _ = self.cache.articles.put(article_id.to_string(), Arc::clone(article));
        }
        
        self.record_operation_time(start.elapsed());
        Ok(article)
    }

    async fn save_article(&self, feed_name: &str, article: Article) -> Result<()> {
        let start = Instant::now();
        let article_id = article.id.clone();
        
        // Cache article
        let _ = self.cache.articles.put(article_id, Arc::new(article.clone()));
        
        // Store persistently
        self.record_storage_write();
        self.storage.store_article(feed_name, &article).await?;
        
        self.record_operation_time(start.elapsed());
        Ok(())
    }

    async fn save_articles(&self, feed_name: &str, articles: Vec<Article>) -> Result<()> {
        let start = Instant::now();
        
        // Cache all articles
        for article in &articles {
            let _ = self.cache.articles.put(article.id.clone(), Arc::new(article.clone()));
        }
        
        // Store all articles
        for article in &articles {
            self.record_storage_write();
            self.storage.store_article(feed_name, article).await?;
        }
        
        self.record_operation_time(start.elapsed());
        Ok(())
    }

    async fn list_articles(&self, feed_name: &str) -> Result<Vec<String>> {
        let start = Instant::now();
        self.record_storage_read();
        let result = self.storage.list_articles(feed_name).await;
        self.record_operation_time(start.elapsed());
        result
    }

    async fn search_articles(&self, query: &ArticleQuery) -> Result<Vec<Arc<Article>>> {
        let start = Instant::now();
        
        // Get all article IDs for the feed (if specified)
        let article_ids = if let Some(feed_name) = &query.feed_name {
            self.storage.list_articles(feed_name).await?
        } else {
            // This is simplified - in a real implementation we'd have proper indexing
            let feeds = self.storage.list_feeds().await?;
            let mut all_ids = Vec::new();
            for feed in feeds {
                let mut ids = self.storage.list_articles(&feed).await?;
                all_ids.append(&mut ids);
            }
            all_ids
        };
        
        // Fetch and filter articles
        let mut results = Vec::new();
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        
        for (i, article_id) in article_ids.iter().enumerate() {
            if i < offset {
                continue;
            }
            if results.len() >= limit {
                break;
            }
            
            if let Some(article) = self.get_article(article_id).await? {
                // Apply filters
                let mut matches = true;
                
                if let Some(title_filter) = &query.title_contains {
                    if !article.title.to_lowercase().contains(&title_filter.to_lowercase()) {
                        matches = false;
                    }
                }
                
                if let Some(content_filter) = &query.content_contains {
                    if let Some(content) = &article.content {
                        if !content.to_lowercase().contains(&content_filter.to_lowercase()) {
                            matches = false;
                        }
                    } else {
                        matches = false;
                    }
                }
                
                if let Some(author_filter) = &query.author {
                    if article.author.as_ref().map_or(true, |a| a != author_filter) {
                        matches = false;
                    }
                }
                
                if !query.tags.is_empty() {
                    let article_tags: std::collections::HashSet<_> = article.tags.iter().collect();
                    let query_tags: std::collections::HashSet<_> = query.tags.iter().collect();
                    if !query_tags.is_subset(&article_tags) {
                        matches = false;
                    }
                }
                
                if let Some(date_from) = query.date_from {
                    if article.published.map_or(true, |d| d < date_from) {
                        matches = false;
                    }
                }
                
                if let Some(date_to) = query.date_to {
                    if article.published.map_or(true, |d| d > date_to) {
                        matches = false;
                    }
                }
                
                if matches {
                    results.push(article);
                }
            }
        }
        
        self.record_operation_time(start.elapsed());
        Ok(results)
    }

    async fn delete_article(&self, article_id: &str) -> Result<()> {
        let start = Instant::now();
        
        // Remove from cache
        self.cache.articles.remove(article_id);
        
        // Remove from storage
        self.record_storage_write();
        self.storage.remove_article(article_id).await?;
        
        self.record_operation_time(start.elapsed());
        Ok(())
    }

    async fn delete_feed_articles(&self, feed_name: &str) -> Result<usize> {
        let start = Instant::now();
        
        // Get all article IDs for the feed
        let article_ids = self.storage.list_articles(feed_name).await?;
        let count = article_ids.len();
        
        // Remove from cache
        for article_id in &article_ids {
            self.cache.articles.remove(article_id);
        }
        
        // Remove from storage (this is simplified - in a real DB we'd use a batch operation)
        for article_id in article_ids {
            self.record_storage_write();
            self.storage.remove_article(&article_id).await?;
        }
        
        self.record_operation_time(start.elapsed());
        Ok(count)
    }

    async fn get_stats(&self) -> Result<ArticleStats> {
        let start = Instant::now();
        
        // Get all feeds and their articles
        let feeds = self.storage.list_feeds().await?;
        let mut articles_by_feed = std::collections::HashMap::new();
        let mut total_articles = 0;
        let mut total_size = 0;
        let mut oldest_date = None;
        let mut newest_date = None;
        
        for feed_name in feeds {
            let article_ids = self.storage.list_articles(&feed_name).await?;
            articles_by_feed.insert(feed_name.clone(), article_ids.len());
            total_articles += article_ids.len();
            
            // Sample some articles for statistics
            for article_id in article_ids.iter().take(10) {
                if let Some(article) = self.storage.get_article(article_id).await? {
                    total_size += article.content.as_ref().map_or(0, |c| c.len());
                    
                    if let Some(published) = article.published {
                        match oldest_date {
                            None => oldest_date = Some(published),
                            Some(current) if published < current => oldest_date = Some(published),
                            _ => {}
                        }
                        
                        match newest_date {
                            None => newest_date = Some(published),
                            Some(current) if published > current => newest_date = Some(published),
                            _ => {}
                        }
                    }
                }
            }
        }
        
        let avg_article_size = if total_articles > 0 {
            total_size / total_articles
        } else {
            0
        };
        
        self.record_operation_time(start.elapsed());
        
        Ok(ArticleStats {
            total_articles,
            articles_by_feed,
            avg_article_size,
            oldest_article: oldest_date,
            newest_article: newest_date,
        })
    }
}

/// Repository factory for easy creation with different backends
pub struct RepositoryFactory;

impl RepositoryFactory {
    pub fn memory() -> Repository {
        Repository::with_memory_storage()
    }
    
    pub fn with_config(storage_config: StorageConfig, cache_config: CacheConfig) -> Repository {
        let storage = Arc::new(MemoryStorage::new(storage_config));
        Repository::new(storage, cache_config)
    }

    /// Create repository with persistent cache
    pub fn with_persistent_cache(storage_config: StorageConfig, cache_config: CacheConfig,
                                persistent_config: PersistentCacheConfig) -> Result<Repository> {
        let storage = Arc::new(MemoryStorage::new(storage_config));
        Repository::with_persistent_cache(storage, cache_config, persistent_config)
    }
    
    pub async fn create_with_cleanup_task(
        storage_config: StorageConfig,
        cache_config: CacheConfig,
    ) -> Repository {
        let repo = Self::with_config(storage_config, cache_config);
        
        // Start background cleanup task
        let cache_manager = repo.cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                let (articles_cleaned, feeds_cleaned) = cache_manager.cleanup_expired();
                if articles_cleaned > 0 || feeds_cleaned > 0 {
                    tracing::debug!(
                        "Cache cleanup: {} articles, {} feeds",
                        articles_cleaned,
                        feeds_cleaned
                    );
                }
            }
        });
        
        repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{ParsedArticle, FeedStatus};
    use chrono::Utc;

    fn create_test_article(id: &str, feed_name: &str) -> Article {
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
        Article::new(parsed, feed_name)
    }

    fn create_test_feed(name: &str) -> Feed {
        let article = create_test_article("1", name);
        Feed {
            name: name.to_string(),
            url: format!("https://example.com/{}.xml", name),
            title: Some(format!("Test Feed {}", name)),
            description: Some("Test feed description".to_string()),
            last_updated: Some(Utc::now()),
            articles: vec![article],
            status: FeedStatus::Active,
        }
    }

    #[tokio::test]
    async fn test_repository_feed_operations() {
        let repo = RepositoryFactory::memory();
        let feed = create_test_feed("test-feed");

        // Save feed
        repo.save_feed(feed.clone()).await.unwrap();

        // Get feed
        let retrieved = repo.get_feed("test-feed").await.unwrap().unwrap();
        assert_eq!(retrieved.name, "test-feed");

        // List feeds
        let feeds = repo.list_feeds().await.unwrap();
        assert_eq!(feeds.len(), 1);
        assert!(feeds.contains(&"test-feed".to_string()));
    }

    #[tokio::test]
    async fn test_repository_article_operations() {
        let repo = RepositoryFactory::memory();
        let article = create_test_article("test-article", "test-feed");
        let article_id = article.id.clone();

        // Save article
        repo.save_article("test-feed", article).await.unwrap();

        // Get article
        let retrieved = repo.get_article(&article_id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, article_id);

        // List articles
        let articles = repo.list_articles("test-feed").await.unwrap();
        assert_eq!(articles.len(), 1);
        assert!(articles.contains(&article_id));
    }

    #[tokio::test]
    async fn test_repository_caching() {
        let repo = RepositoryFactory::memory();
        let feed = create_test_feed("test-feed");

        // Save feed
        repo.save_feed(feed.clone()).await.unwrap();

        // First get - cache miss
        let _ = repo.get_feed("test-feed").await.unwrap();
        
        // Second get - should be cache hit
        let _ = repo.get_feed("test-feed").await.unwrap();

        // Check stats
        let stats = FeedRepository::get_stats(&repo).await.unwrap();
        assert!(stats.cache_hit_rate > 0.0);
    }

    #[tokio::test]
    async fn test_repository_search() {
        let repo = RepositoryFactory::memory();
        let feed = create_test_feed("test-feed");

        repo.save_feed(feed).await.unwrap();

        // Search by feed name
        let query = ArticleQuery {
            feed_name: Some("test-feed".to_string()),
            ..Default::default()
        };
        let results = repo.search_articles(&query).await.unwrap();
        assert_eq!(results.len(), 1);

        // Search by title
        let query = ArticleQuery {
            title_contains: Some("Test Article".to_string()),
            ..Default::default()
        };
        let results = repo.search_articles(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_repository_deletion() {
        let repo = RepositoryFactory::memory();
        let feed = create_test_feed("test-feed");

        // Save feed
        repo.save_feed(feed).await.unwrap();

        // Verify it exists
        assert!(repo.get_feed("test-feed").await.unwrap().is_some());

        // Delete feed
        repo.delete_feed("test-feed").await.unwrap();

        // Verify it's gone
        assert!(repo.get_feed("test-feed").await.unwrap().is_none());
    }
}