use std::time::Duration;
use rss_fuse::storage::*;
use rss_fuse::storage::{FeedRepository};
use rss_fuse::feed::{Article, ParsedArticle, Feed, FeedStatus};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗂️  RSS-FUSE Storage System Test");
    println!("=================================\n");

    // Test cache functionality
    test_cache_functionality().await?;
    
    // Test storage traits
    test_storage_functionality().await?;
    
    // Test repository integration
    test_repository_functionality().await?;
    
    println!("🏆 All storage tests completed successfully!");
    
    Ok(())
}

async fn test_cache_functionality() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Testing Cache Functionality...");
    
    // Create cache with custom configuration
    let cache_config = CacheConfig {
        max_entries: 100,
        default_ttl: Duration::from_secs(300), // 5 minutes
        cleanup_interval: Duration::from_secs(60),
        max_memory_mb: 50,
    };
    
    let cache_manager = CacheManager::new(cache_config);
    
    // Test article cache
    let article = create_test_article("cache-test", "test-feed");
    cache_manager.articles.put("test-article".to_string(), article.clone())?;
    
    if let Some(retrieved) = cache_manager.articles.get("test-article") {
        println!("   ✅ Article cache: stored and retrieved successfully");
        assert_eq!(retrieved.id, article.id);
    } else {
        println!("   ❌ Article cache: failed to retrieve");
        return Err("Cache test failed".into());
    }
    
    // Test feed cache
    let feed = create_test_feed("cache-feed");
    cache_manager.feeds.put("cache-feed".to_string(), feed.clone())?;
    
    if let Some(retrieved) = cache_manager.feeds.get("cache-feed") {
        println!("   ✅ Feed cache: stored and retrieved successfully");
        assert_eq!(retrieved.name, feed.name);
    } else {
        println!("   ❌ Feed cache: failed to retrieve");
        return Err("Cache test failed".into());
    }
    
    // Test cache statistics
    let (article_stats, feed_stats) = cache_manager.combined_stats();
    println!("   📊 Cache stats: {} articles, {} feeds", 
             article_stats.total_entries, feed_stats.total_entries);
    println!("   📈 Article hit rate: {:.1}%", article_stats.hit_rate() * 100.0);
    
    println!("   ✅ Cache functionality test passed!\n");
    Ok(())
}

async fn test_storage_functionality() -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Testing Storage Functionality...");
    
    let storage = MemoryStorage::default();
    
    // Test feed storage
    let feed = create_test_feed("storage-feed");
    storage.store_feed(&feed).await?;
    
    let retrieved_feed = storage.get_feed("storage-feed").await?;
    if let Some(feed) = retrieved_feed {
        println!("   ✅ Feed storage: stored and retrieved successfully");
        assert_eq!(feed.name, "storage-feed");
        assert_eq!(feed.articles.len(), 1);
    } else {
        return Err("Storage test failed".into());
    }
    
    // Test article storage
    let article = create_test_article("storage-test", "storage-feed");
    storage.store_article("storage-feed", &article).await?;
    
    let retrieved_article = storage.get_article(&article.id).await?;
    if let Some(article_arc) = retrieved_article {
        println!("   ✅ Article storage: stored and retrieved successfully");
        assert_eq!(article_arc.id, article.id);
    } else {
        return Err("Storage test failed".into());
    }
    
    // Test listing
    let feeds = storage.list_feeds().await?;
    let articles = storage.list_articles("storage-feed").await?;
    println!("   📋 Listed {} feeds and {} articles", feeds.len(), articles.len());
    
    // Test statistics
    let stats = storage.get_stats().await?;
    println!("   📊 Storage stats: {} feeds, {} articles, {} bytes", 
             stats.total_feeds, stats.total_articles, stats.storage_size_bytes);
    
    println!("   ✅ Storage functionality test passed!\n");
    Ok(())
}

async fn test_repository_functionality() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing Repository Integration...");
    
    let repo = RepositoryFactory::memory();
    
    // Test combined feed and cache operations
    let feed = create_test_feed("repo-feed");
    repo.save_feed(feed.clone()).await?;
    
    // First access - cache miss
    let retrieved1 = repo.get_feed("repo-feed").await?;
    assert!(retrieved1.is_some());
    
    // Second access - cache hit
    let retrieved2 = repo.get_feed("repo-feed").await?;
    assert!(retrieved2.is_some());
    
    // Test article operations
    let article = create_test_article("repo-test", "repo-feed");
    repo.save_article("repo-feed", (*article).clone()).await?;
    
    let retrieved_article = repo.get_article(&article.id).await?;
    assert!(retrieved_article.is_some());
    
    // Test search functionality
    let query = ArticleQuery {
        title_contains: Some("Test Article".to_string()),
        ..Default::default()
    };
    let search_results = repo.search_articles(&query).await?;
    println!("   🔍 Search found {} articles", search_results.len());
    
    // Test repository statistics
    let stats = FeedRepository::get_stats(&repo).await?;
    println!("   📊 Repository stats: cache hit rate {:.1}%, avg response time {:.2}ms", 
             stats.cache_hit_rate * 100.0, stats.avg_response_time_ms);
    
    println!("   ✅ Repository integration test passed!\n");
    Ok(())
}

fn create_test_article(id: &str, feed_name: &str) -> std::sync::Arc<Article> {
    let parsed = ParsedArticle {
        title: format!("Test Article {}", id),
        link: format!("https://example.com/{}", id),
        description: Some("Test description".to_string()),
        content: Some("Test content for storage validation".to_string()),
        author: Some("Test Author".to_string()),
        published: Some(Utc::now()),
        guid: Some(id.to_string()),
        categories: vec!["test".to_string(), "storage".to_string()],
    };
    std::sync::Arc::new(Article::new(parsed, feed_name))
}

fn create_test_feed(name: &str) -> Feed {
    let article = (*create_test_article("1", name)).clone();
    Feed {
        name: name.to_string(),
        url: format!("https://example.com/{}.xml", name),
        title: Some(format!("Test Feed {}", name.to_uppercase())),
        description: Some("Test feed for storage validation".to_string()),
        last_updated: Some(Utc::now()),
        articles: vec![article],
        status: FeedStatus::Active,
    }
}