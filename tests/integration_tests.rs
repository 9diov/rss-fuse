use rss_fuse::feed::fetcher::FeedFetcher;
use rss_fuse::fuse::FuseOperations;
use tempfile::TempDir;
use tokio;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

mod test_data;
use test_data::*;

/// Integration tests for the complete RSS-FUSE workflow
/// These tests verify end-to-end functionality from feed fetching to FUSE filesystem operations

#[tokio::test]
async fn test_complete_rss_to_fuse_workflow() {
    // Setup mock server with RSS feeds
    let mock_server = MockServer::start().await;
    
    // Mount different RSS feeds
    Mock::given(method("GET"))
        .and(path("/tech-news.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(TECH_NEWS_RSS)
                .insert_header("content-type", "application/rss+xml")
        )
        .mount(&mock_server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/science-blog.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SCIENCE_BLOG_ATOM)
                .insert_header("content-type", "application/atom+xml")
        )
        .mount(&mock_server)
        .await;

    // Step 1: Fetch feeds
    let fetcher = FeedFetcher::new();
    let tech_feed_url = format!("{}/tech-news.xml", mock_server.uri());
    let science_feed_url = format!("{}/science-blog.xml", mock_server.uri());
    
    let tech_parsed = fetcher.fetch_feed(&tech_feed_url).await.unwrap();
    let science_parsed = fetcher.fetch_feed(&science_feed_url).await.unwrap();
    
    // Step 2: Convert to Feed objects
    let tech_feed = rss_fuse::feed::Feed {
        name: "tech-news".to_string(),
        url: tech_feed_url,
        title: Some(tech_parsed.title.clone()),
        description: tech_parsed.description.clone(),
        last_updated: tech_parsed.last_build_date,
        articles: tech_parsed.articles.into_iter()
            .map(|a| rss_fuse::feed::Article::new(a, "tech-news"))
            .collect(),
        status: rss_fuse::feed::FeedStatus::Active,
    };
    
    let science_feed = rss_fuse::feed::Feed {
        name: "science-blog".to_string(),
        url: science_feed_url,
        title: Some(science_parsed.title.clone()),
        description: science_parsed.description.clone(),
        last_updated: science_parsed.last_build_date,
        articles: science_parsed.articles.into_iter()
            .map(|a| rss_fuse::feed::Article::new(a, "science-blog"))
            .collect(),
        status: rss_fuse::feed::FeedStatus::Active,
    };
    
    // Step 3: Create FUSE filesystem and add feeds
    let fuse_ops = FuseOperations::new();
    
    fuse_ops.filesystem.add_feed(tech_feed).unwrap();
    fuse_ops.filesystem.add_feed(science_feed).unwrap();
    
    // Step 4: Verify filesystem structure
    let stats = fuse_ops.get_stats();
    assert_eq!(stats.feeds_count, 2);
    assert!(stats.total_inodes > 5); // Root + meta + feeds + articles
    
    // Verify root directory contents
    let root_children = fuse_ops.filesystem.list_children(1);
    assert_eq!(root_children.len(), 3); // .rss-fuse + 2 feeds
    
    let feed_names: Vec<String> = root_children.iter()
        .filter(|n| !n.name.starts_with('.'))
        .map(|n| n.name.clone())
        .collect();
    assert!(feed_names.contains(&"tech-news".to_string()));
    assert!(feed_names.contains(&"science-blog".to_string()));
    
    // Step 5: Verify feed directories and articles
    for child in &root_children {
        if child.name == "tech-news" {
            let articles = fuse_ops.filesystem.list_children(child.ino);
            assert_eq!(articles.len(), 3); // Tech news has 3 articles
            
            // Verify article content can be read
            if let Some(first_article) = articles.first() {
                let content = fuse_ops.filesystem.get_article_content(first_article.ino).unwrap();
                assert!(content.contains("AI Revolution in 2024"));
                assert!(content.contains("Tags: AI, Technology"));
                assert!(content.len() > 100); // Should have substantial content
            }
        } else if child.name == "science-blog" {
            let articles = fuse_ops.filesystem.list_children(child.ino);
            assert_eq!(articles.len(), 2); // Science blog has 2 articles
            
            // Verify article filenames are properly sanitized
            for article in &articles {
                assert!(article.name.ends_with(".txt"));
                assert!(!article.name.contains("<"));
                assert!(!article.name.contains(">"));
            }
        }
    }
    
    println!("✅ Complete RSS-to-FUSE workflow test passed!");
}

#[tokio::test]
async fn test_concurrent_feed_processing_and_fuse_operations() {
    let mock_server = MockServer::start().await;
    
    // Setup multiple feeds
    let feeds = vec![
        ("feed1", SIMPLE_RSS),
        ("feed2", TECH_RSS), 
        ("feed3", SCIENCE_RSS),
        ("feed4", NEWS_RSS),
    ];
    
    for (name, content) in &feeds {
        Mock::given(method("GET"))
            .and(path(&format!("/{}.xml", name)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(*content)
                    .insert_header("content-type", "application/rss+xml")
            )
            .mount(&mock_server)
            .await;
    }
    
    // Concurrently fetch all feeds
    let fetcher = FeedFetcher::new();
    let urls: Vec<String> = feeds.iter()
        .map(|(name, _)| format!("{}/{}.xml", mock_server.uri(), name))
        .collect();
    
    let start_time = std::time::Instant::now();
    let results = fetcher.fetch_multiple_feeds(&urls).await;
    let fetch_duration = start_time.elapsed();
    
    // Verify all feeds fetched successfully
    assert_eq!(results.len(), 4);
    for (url, result) in &results {
        assert!(result.is_ok(), "Failed to fetch {}", url);
    }
    
    // Create FUSE filesystem
    let fuse_ops = FuseOperations::new();
    
    // Add all feeds to filesystem
    let add_start = std::time::Instant::now();
    for (i, (url, result)) in results.into_iter().enumerate() {
        let parsed_feed = result.unwrap();
        let feed_name = format!("feed{}", i + 1);
        
        let feed = rss_fuse::feed::Feed {
            name: feed_name.clone(),
            url,
            title: Some(parsed_feed.title.clone()),
            description: parsed_feed.description.clone(),
            last_updated: parsed_feed.last_build_date,
            articles: parsed_feed.articles.into_iter()
                .map(|a| rss_fuse::feed::Article::new(a, &feed_name))
                .collect(),
            status: rss_fuse::feed::FeedStatus::Active,
        };
        
        fuse_ops.filesystem.add_feed(feed).unwrap();
    }
    let add_duration = add_start.elapsed();
    
    // Verify final state
    let final_stats = fuse_ops.get_stats();
    assert_eq!(final_stats.feeds_count, 4);
    
    // Performance assertions
    assert!(fetch_duration < std::time::Duration::from_secs(10), 
            "Concurrent fetching took too long: {:?}", fetch_duration);
    assert!(add_duration < std::time::Duration::from_secs(1),
            "Adding feeds to FUSE took too long: {:?}", add_duration);
    
    // Verify filesystem browsability
    let root_children = fuse_ops.filesystem.list_children(1);
    assert_eq!(root_children.len(), 5); // .rss-fuse + 4 feeds
    
    println!("✅ Concurrent processing test passed!");
    println!("   Fetch time: {:?}, Add time: {:?}", fetch_duration, add_duration);
}

#[tokio::test]
async fn test_feed_lifecycle_with_fuse_updates() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/dynamic-feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(TECH_NEWS_RSS)
                .insert_header("content-type", "application/rss+xml")
        )
        .mount(&mock_server)
        .await;
    
    let fuse_ops = FuseOperations::new();
    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/dynamic-feed.xml", mock_server.uri());
    
    // Initial state
    let initial_stats = fuse_ops.get_stats();
    assert_eq!(initial_stats.feeds_count, 0);
    
    // Add feed
    let parsed_feed = fetcher.fetch_feed(&feed_url).await.unwrap();
    let feed = rss_fuse::feed::Feed {
        name: "dynamic-feed".to_string(),
        url: feed_url,
        title: Some(parsed_feed.title.clone()),
        description: parsed_feed.description.clone(),
        last_updated: parsed_feed.last_build_date,
        articles: parsed_feed.articles.into_iter()
            .map(|a| rss_fuse::feed::Article::new(a, "dynamic-feed"))
            .collect(),
        status: rss_fuse::feed::FeedStatus::Active,
    };
    
    fuse_ops.filesystem.add_feed(feed).unwrap();
    
    // Verify feed added
    let after_add_stats = fuse_ops.get_stats();
    assert_eq!(after_add_stats.feeds_count, 1);
    assert!(after_add_stats.total_inodes > initial_stats.total_inodes);
    
    // Verify feed directory exists
    let feed_node = fuse_ops.filesystem.get_node_by_name(1, "dynamic-feed").unwrap();
    assert!(feed_node.is_directory());
    
    let articles = fuse_ops.filesystem.list_children(feed_node.ino);
    assert_eq!(articles.len(), 3);
    
    // Test reading article content
    let first_article = &articles[0];
    let content = fuse_ops.filesystem.get_article_content(first_article.ino).unwrap();
    assert!(content.contains("Title:"));
    assert!(content.contains("Link:"));
    assert!(content.contains("---"));
    
    // Remove feed
    fuse_ops.filesystem.remove_feed("dynamic-feed").unwrap();
    
    // Verify feed removed
    let after_remove_stats = fuse_ops.get_stats();
    assert_eq!(after_remove_stats.feeds_count, 0);
    
    // Verify feed directory no longer exists
    assert!(fuse_ops.filesystem.get_node_by_name(1, "dynamic-feed").is_none());
    
    println!("✅ Feed lifecycle test passed!");
}

#[tokio::test]
async fn test_configuration_integration_with_fuse() {
    let fuse_ops = FuseOperations::new();
    
    // Create configuration
    let config_content = r#"[feeds]
"tech-news" = "https://example.com/tech.xml"
"science-blog" = "https://example.com/science.xml"

[settings]
refresh_interval = 300
cache_duration = 3600
max_articles = 50
article_content = true
"#.to_string();
    
    // Update configuration in filesystem
    fuse_ops.filesystem.update_config(config_content.clone());
    
    // Verify config file exists in .rss-fuse directory
    let meta_node = fuse_ops.filesystem.get_node_by_name(1, ".rss-fuse").unwrap();
    assert!(meta_node.is_directory());
    
    let config_node = fuse_ops.filesystem.get_node_by_name(meta_node.ino, "config.toml").unwrap();
    assert!(config_node.is_file());
    assert_eq!(config_node.size, config_content.len() as u64);
    
    // Verify meta directory structure
    let meta_children = fuse_ops.filesystem.list_children(meta_node.ino);
    assert_eq!(meta_children.len(), 3); // config.toml, logs, cache
    
    let child_names: Vec<String> = meta_children.iter().map(|n| n.name.clone()).collect();
    assert!(child_names.contains(&"config.toml".to_string()));
    assert!(child_names.contains(&"logs".to_string()));
    assert!(child_names.contains(&"cache".to_string()));
    
    println!("✅ Configuration integration test passed!");
}

#[tokio::test]
async fn test_error_handling_across_modules() {
    let mock_server = MockServer::start().await;
    
    // Setup various error conditions
    Mock::given(method("GET"))
        .and(path("/not-found.xml"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/malformed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(MALFORMED_XML)
                .insert_header("content-type", "application/xml")
        )
        .mount(&mock_server)
        .await;
    
    let fuse_ops = FuseOperations::new();
    let fetcher = FeedFetcher::new();
    
    // Test 404 error handling
    let not_found_url = format!("{}/not-found.xml", mock_server.uri());
    let result = fetcher.fetch_feed(&not_found_url).await;
    assert!(result.is_err());
    
    // Verify FUSE filesystem remains stable after error
    let stats_after_error = fuse_ops.get_stats();
    assert_eq!(stats_after_error.feeds_count, 0);
    
    // Test malformed XML handling
    let malformed_url = format!("{}/malformed.xml", mock_server.uri());
    let result = fetcher.fetch_feed(&malformed_url).await;
    assert!(result.is_err());
    
    // Verify filesystem still accessible
    let root_children = fuse_ops.filesystem.list_children(1);
    assert_eq!(root_children.len(), 1); // Should still have .rss-fuse
    assert_eq!(root_children[0].name, ".rss-fuse");
    
    // Test invalid mount point
    let invalid_path = std::path::Path::new("/nonexistent/path");
    let mount_result = fuse_ops.validate_mount_point(invalid_path);
    assert!(mount_result.is_err());
    
    println!("✅ Error handling test passed!");
}

#[tokio::test]
async fn test_memory_efficiency_with_large_feeds() {
    let mock_server = MockServer::start().await;
    
    // Create a large feed with many articles
    let mut large_feed = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Large Feed Test</title>
        <description>A feed with many articles for memory testing</description>
        <link>https://example.com</link>"#);
    
    let large_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100);
    
    // Add 100 articles
    for i in 0..100 {
        large_feed.push_str(&format!(r#"
        <item>
            <title>Large Article {}</title>
            <link>https://example.com/large{}</link>
            <description><![CDATA[{}]]></description>
            <pubDate>Wed, 15 Mar 2024 10:{}:00 GMT</pubDate>
            <guid>large-article-{}</guid>
        </item>"#, i, i, large_content, i % 60, i));
    }
    
    large_feed.push_str("\n    </channel>\n</rss>");
    
    Mock::given(method("GET"))
        .and(path("/large-feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(large_feed)
                .insert_header("content-type", "application/rss+xml")
        )
        .mount(&mock_server)
        .await;
    
    let fuse_ops = FuseOperations::new();
    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/large-feed.xml", mock_server.uri());
    
    // Measure memory before
    let initial_stats = fuse_ops.get_stats();
    
    // Fetch and add large feed
    let parsed_feed = fetcher.fetch_feed(&feed_url).await.unwrap();
    assert_eq!(parsed_feed.articles.len(), 100);
    
    let feed = rss_fuse::feed::Feed {
        name: "large-feed".to_string(),
        url: feed_url,
        title: Some(parsed_feed.title.clone()),
        description: parsed_feed.description.clone(),
        last_updated: parsed_feed.last_build_date,
        articles: parsed_feed.articles.into_iter()
            .map(|a| rss_fuse::feed::Article::new(a, "large-feed"))
            .collect(),
        status: rss_fuse::feed::FeedStatus::Active,
    };
    
    fuse_ops.filesystem.add_feed(feed).unwrap();
    
    // Verify all articles are accessible
    let feed_node = fuse_ops.filesystem.get_node_by_name(1, "large-feed").unwrap();
    let articles = fuse_ops.filesystem.list_children(feed_node.ino);
    assert_eq!(articles.len(), 100);
    
    // Test random access to articles (filesystem should handle this efficiently)
    let indices = vec![0, 25, 50, 75, 99];
    for &idx in &indices {
        if let Some(article) = articles.get(idx) {
            let content = fuse_ops.filesystem.get_article_content(article.ino).unwrap();
            assert!(content.contains(&format!("Large Article {}", idx)));
            assert!(content.len() > 1000); // Should have substantial content
        }
    }
    
    let final_stats = fuse_ops.get_stats();
    assert_eq!(final_stats.feeds_count, 1);
    assert_eq!(final_stats.total_inodes, initial_stats.total_inodes + 101); // +1 feed dir + 100 articles
    
    println!("✅ Memory efficiency test passed!");
    println!("   Articles processed: 100");
    println!("   Inodes created: {}", final_stats.total_inodes - initial_stats.total_inodes);
}

#[tokio::test]
async fn test_mount_point_validation_integration() {
    let temp_dir = TempDir::new().unwrap();
    let mount_point = temp_dir.path();
    
    let fuse_ops = FuseOperations::new();
    
    // Test successful validation
    let result = fuse_ops.validate_mount_point(mount_point);
    assert!(result.is_ok());
    
    // Create a file in the directory
    let file_path = mount_point.join("test.txt");
    std::fs::write(&file_path, "test content").unwrap();
    
    // Should still validate (just warns about non-empty)
    let result = fuse_ops.validate_mount_point(mount_point);
    assert!(result.is_ok());
    
    // Test with file instead of directory
    let result = fuse_ops.validate_mount_point(&file_path);
    assert!(result.is_err());
    
    // Test with non-existent path
    let non_existent = mount_point.join("does-not-exist");
    let result = fuse_ops.validate_mount_point(&non_existent);
    assert!(result.is_err());
    
    println!("✅ Mount point validation integration test passed!");
}

/// Helper function to create a test feed with specified number of articles
fn create_test_feed_with_articles(name: &str, count: usize) -> rss_fuse::feed::Feed {
    let articles: Vec<rss_fuse::feed::Article> = (0..count).map(|i| {
        let parsed = rss_fuse::feed::ParsedArticle {
            title: format!("Test Article {} from {}", i + 1, name),
            link: format!("https://example.com/{}/article-{}", name, i + 1),
            description: Some(format!("Description for article {}", i + 1)),
            content: Some(format!("Content for article {} in {}", i + 1, name)),
            author: Some(format!("Author {}", i + 1)),
            published: Some(chrono::Utc::now()),
            guid: Some(format!("{}-{}", name, i + 1)),
            categories: vec![name.to_string(), "test".to_string()],
        };
        rss_fuse::feed::Article::new(parsed, name)
    }).collect();
    
    rss_fuse::feed::Feed {
        name: name.to_string(),
        url: format!("https://example.com/{}.xml", name),
        title: Some(format!("{} Feed", name.to_uppercase())),
        description: Some(format!("Test feed for {}", name)),
        last_updated: Some(chrono::Utc::now()),
        articles,
        status: rss_fuse::feed::FeedStatus::Active,
    }
}