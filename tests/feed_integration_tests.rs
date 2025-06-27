use rss_fuse::feed::{parser::FeedParser, fetcher::FeedFetcher, Article, ParsedArticle};
use std::collections::HashMap;
use std::io::Cursor;
use std::time::Duration;
use tokio;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};

mod test_data;
use test_data::*;

/// Integration tests for feed parsing and fetching functionality
/// These tests verify end-to-end behavior of the feed system

#[tokio::test]
async fn test_end_to_end_rss_processing() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/tech-news.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(TECH_NEWS_RSS)
                .insert_header("content-type", "application/rss+xml")
                .insert_header("last-modified", "Thu, 16 Mar 2024 12:00:00 GMT")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/tech-news.xml", mock_server.uri());
    
    // Fetch and parse the feed
    let parsed_feed = fetcher.fetch_feed(&feed_url).await.unwrap();
    
    // Verify feed metadata
    assert_eq!(parsed_feed.title, "Tech News Daily");
    assert_eq!(parsed_feed.description, Some("Latest technology news and updates".to_string()));
    assert_eq!(parsed_feed.articles.len(), 3);
    
    // Convert to Article objects and verify content
    let articles: Vec<Article> = parsed_feed.articles
        .into_iter()
        .map(|parsed| Article::new(parsed, "tech-news"))
        .collect();
    
    // Verify first article
    let first_article = &articles[0];
    assert_eq!(first_article.title, "AI Revolution in 2024");
    assert!(first_article.link.contains("ai-revolution-2024"));
    assert!(first_article.tags.contains(&"AI".to_string()));
    assert!(first_article.tags.contains(&"Technology".to_string()));
    
    // Verify filename generation
    assert_eq!(first_article.filename(), "AI Revolution in 2024.txt");
    
    // Verify text format
    let text_content = first_article.to_text();
    assert!(text_content.contains("Title: AI Revolution in 2024"));
    assert!(text_content.contains("Author: John Doe"));
    assert!(text_content.contains("Tags: AI, Technology"));
    assert!(text_content.contains("The artificial intelligence landscape"));
}

#[tokio::test]
async fn test_end_to_end_atom_processing() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/science-blog.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SCIENCE_BLOG_ATOM)
                .insert_header("content-type", "application/atom+xml")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/science-blog.xml", mock_server.uri());
    
    let parsed_feed = fetcher.fetch_feed(&feed_url).await.unwrap();
    
    assert_eq!(parsed_feed.title, "Science Discoveries");
    assert_eq!(parsed_feed.articles.len(), 2);
    
    let articles: Vec<Article> = parsed_feed.articles
        .into_iter()
        .map(|parsed| Article::new(parsed, "science-blog"))
        .collect();
    
    // Verify content extraction from HTML
    let first_article = &articles[0];
    assert!(first_article.content.is_some());
    assert!(first_article.content.as_ref().unwrap().contains("quantum computing"));
}

#[tokio::test]
async fn test_concurrent_feed_fetching() {
    let mock_server = MockServer::start().await;
    
    // Setup multiple feeds
    let feeds = vec![
        ("news", NEWS_RSS),
        ("tech", TECH_RSS),
        ("science", SCIENCE_RSS),
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
    
    let fetcher = FeedFetcher::new();
    let urls: Vec<String> = feeds.iter()
        .map(|(name, _)| format!("{}/{}.xml", mock_server.uri(), name))
        .collect();
    
    // Fetch all feeds concurrently
    let start_time = std::time::Instant::now();
    let results = fetcher.fetch_multiple_feeds(&urls).await;
    let elapsed = start_time.elapsed();
    
    // Verify all feeds were fetched successfully
    assert_eq!(results.len(), 3);
    for (url, result) in results {
        assert!(result.is_ok(), "Failed to fetch {}: {:?}", url, result);
    }
    
    // Concurrent fetching should be faster than sequential
    // (This is a rough check - actual timing may vary)
    assert!(elapsed < Duration::from_secs(5));
}

#[tokio::test]
async fn test_feed_error_handling_and_recovery() {
    let mock_server = MockServer::start().await;
    
    // Setup different error conditions
    Mock::given(method("GET"))
        .and(path("/not-found.xml"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/server-error.xml"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/timeout.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_string(SIMPLE_RSS)
        )
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
    
    let fetcher = FeedFetcher::new().with_timeout(Duration::from_secs(1));
    
    // Test 404 error
    let result = fetcher.fetch_feed(&format!("{}/not-found.xml", mock_server.uri())).await;
    assert!(result.is_err());
    
    // Test server error
    let result = fetcher.fetch_feed(&format!("{}/server-error.xml", mock_server.uri())).await;
    assert!(result.is_err());
    
    // Test timeout
    let result = fetcher.fetch_feed(&format!("{}/timeout.xml", mock_server.uri())).await;
    assert!(result.is_err());
    
    // Test malformed XML
    let result = fetcher.fetch_feed(&format!("{}/malformed.xml", mock_server.uri())).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_feed_with_special_characters_and_encoding() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/unicode.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(UNICODE_RSS)
                .insert_header("content-type", "application/rss+xml; charset=utf-8")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/unicode.xml", mock_server.uri());
    
    let parsed_feed = fetcher.fetch_feed(&feed_url).await.unwrap();
    
    // Verify Unicode characters are preserved
    assert!(parsed_feed.title.contains("Café"));
    assert!(parsed_feed.articles[0].title.contains("Naïve"));
    assert!(parsed_feed.articles[0].title.contains("résumé"));
    
    let article = Article::new(parsed_feed.articles[0].clone(), "unicode-test");
    
    // Verify filename sanitization handles Unicode
    let filename = article.filename();
    assert!(filename.len() > 0);
    assert!(filename.ends_with(".txt"));
}

#[tokio::test]
async fn test_article_id_generation_and_deduplication() {
    let parser = FeedParser::new();
    
    // Test with GUID
    let article_with_guid = ParsedArticle {
        title: "Test Article".to_string(),
        link: "https://example.com/test".to_string(),
        description: None,
        content: None,
        author: None,
        published: None,
        guid: Some("unique-guid-123".to_string()),
        categories: vec![],
    };
    
    let article1 = Article::new(article_with_guid.clone(), "test-feed");
    assert_eq!(article1.id, "unique-guid-123");
    
    // Test without GUID (should use hash)
    let article_without_guid = ParsedArticle {
        guid: None,
        ..article_with_guid.clone()
    };
    
    let article2 = Article::new(article_without_guid, "test-feed");
    assert!(article2.id.starts_with("test-feed:"));
    assert!(article2.id.len() > 20); // Hash should be longer
    
    // Same article should generate same ID
    let article_duplicate = ParsedArticle {
        guid: None,
        ..article_with_guid
    };
    
    let article3 = Article::new(article_duplicate, "test-feed");
    assert_eq!(article2.id, article3.id);
}

#[tokio::test]
async fn test_feed_caching_headers() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/cached-feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SIMPLE_RSS)
                .insert_header("content-type", "application/rss+xml")
                .insert_header("etag", "\"abc123\"")
                .insert_header("last-modified", "Wed, 15 Mar 2024 10:00:00 GMT")
                .insert_header("cache-control", "max-age=3600")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/cached-feed.xml", mock_server.uri());
    
    // Check feed availability to get caching headers
    let info = fetcher.check_feed_availability(&feed_url).await.unwrap();
    
    assert_eq!(info.status_code, 200);
    assert!(info.available);
    assert_eq!(info.etag, Some("\"abc123\"".to_string()));
    assert_eq!(info.last_modified, Some("Wed, 15 Mar 2024 10:00:00 GMT".to_string()));
}

#[tokio::test]
async fn test_feed_content_negotiation() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/feed"))
        .and(header("accept", "application/rss+xml, application/atom+xml, application/xml, text/xml, */*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SIMPLE_RSS)
                .insert_header("content-type", "application/rss+xml")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/feed", mock_server.uri());
    
    let result = fetcher.fetch_feed(&feed_url).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_large_feed_memory_efficiency() {
    let mock_server = MockServer::start().await;
    
    // Create a feed with many large articles
    let mut large_feed = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Large Feed</title>
        <description>A feed with large articles</description>
        <link>https://example.com</link>"#);
    
    let large_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);
    
    for i in 0..100 {
        large_feed.push_str(&format!(r#"
        <item>
            <title>Large Article {}</title>
            <link>https://example.com/large{}</link>
            <description><![CDATA[{}]]></description>
            <pubDate>Wed, 15 Mar 2024 10:{}:00 GMT</pubDate>
        </item>"#, i, i, large_content, i % 60));
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

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/large-feed.xml", mock_server.uri());
    
    let result = fetcher.fetch_feed(&feed_url).await;
    assert!(result.is_ok());
    
    let feed = result.unwrap();
    assert_eq!(feed.articles.len(), 100);
    
    // Verify articles can be converted to text format
    let articles: Vec<Article> = feed.articles.into_iter()
        .take(5) // Only test first 5 to avoid excessive memory usage
        .map(|parsed| Article::new(parsed, "large-feed"))
        .collect();
    
    for article in articles {
        let text = article.to_text();
        assert!(text.contains("Large Article"));
        assert!(text.contains("Lorem ipsum"));
    }
}

#[tokio::test]
async fn test_feed_with_relative_urls() {
    let mock_server = MockServer::start().await;
    
    let rss_with_relative_urls = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Relative URLs Feed</title>
        <description>Feed with relative URLs</description>
        <link>{}</link>
        <item>
            <title>Article with Relative URL</title>
            <link>/article/relative-url</link>
            <description>This article has a relative URL</description>
            <pubDate>Wed, 15 Mar 2024 10:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#, mock_server.uri());
    
    Mock::given(method("GET"))
        .and(path("/relative.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(rss_with_relative_urls)
                .insert_header("content-type", "application/rss+xml")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/relative.xml", mock_server.uri());
    
    let result = fetcher.fetch_feed(&feed_url).await;
    assert!(result.is_ok());
    
    let feed = result.unwrap();
    assert_eq!(feed.articles.len(), 1);
    
    // The article should have the relative URL as-is
    // (URL resolution would be handled by a separate component)
    assert_eq!(feed.articles[0].link, "/article/relative-url");
}

#[tokio::test]
async fn test_feed_with_custom_namespaces() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/namespaced.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(NAMESPACED_RSS)
                .insert_header("content-type", "application/rss+xml")
        )
        .mount(&mock_server)
        .await;

    let fetcher = FeedFetcher::new();
    let feed_url = format!("{}/namespaced.xml", mock_server.uri());
    
    let result = fetcher.fetch_feed(&feed_url).await;
    assert!(result.is_ok());
    
    let feed = result.unwrap();
    assert_eq!(feed.title, "Namespaced Feed");
    assert_eq!(feed.articles.len(), 1);
    
    // Verify that custom namespace elements are handled gracefully
    let article = &feed.articles[0];
    assert_eq!(article.title, "Article with Custom Elements");
}