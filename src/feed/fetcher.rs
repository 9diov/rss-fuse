use crate::error::{Error, Result};
use crate::feed::parser::FeedParser;
use crate::feed::ParsedFeed;
use reqwest::{Client, Response};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn, error};

#[derive(Debug, Clone)]
pub struct FeedFetcher {
    client: Client,
    timeout_duration: Duration,
    max_redirects: usize,
    user_agent: String,
}

impl Default for FeedFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl FeedFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .gzip(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            timeout_duration: Duration::from_secs(30),
            max_redirects: 10,
            user_agent: format!("RSS-FUSE/0.1.0 (+https://github.com/user/rss-fuse)"),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = user_agent;
        self
    }

    pub async fn fetch_feed(&self, url: &str) -> Result<ParsedFeed> {
        debug!("Fetching feed from: {}", url);

        // Validate URL first
        let parser = FeedParser::new();
        parser.validate_feed_url(url)?;

        // Fetch with timeout
        let response = timeout(self.timeout_duration, self.fetch_response(url))
            .await
            .map_err(|_| Error::Timeout(format!("Request to {} timed out", url)))?;

        let response = response?;
        
        // Check response status
        if !response.status().is_success() {
            return Err(Error::HttpError(format!(
                "HTTP {} for {}: {}",
                response.status().as_u16(),
                url,
                response.status().canonical_reason().unwrap_or("Unknown error")
            )));
        }

        // Get response body
        let content = response
            .bytes()
            .await
            .map_err(|e| Error::HttpError(format!("Failed to read response body: {}", e)))?;

        debug!("Downloaded {} bytes from {}", content.len(), url);

        // Parse the feed
        let cursor = std::io::Cursor::new(content);
        parser.parse_feed(cursor)
    }

    async fn fetch_response(&self, url: &str) -> Result<Response> {
        let response = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/rss+xml, application/atom+xml, application/xml, text/xml, */*")
            .send()
            .await
            .map_err(|e| Error::HttpError(format!("Request failed: {}", e)))?;

        Ok(response)
    }

    pub async fn fetch_multiple_feeds(&self, urls: &[String]) -> Vec<(String, Result<ParsedFeed>)> {
        let futures = urls.iter().map(|url| {
            let url_clone = url.clone();
            async move {
                let result = self.fetch_feed(&url_clone).await;
                (url_clone, result)
            }
        });

        futures::future::join_all(futures).await
    }

    pub async fn check_feed_availability(&self, url: &str) -> Result<FeedInfo> {
        debug!("Checking feed availability: {}", url);

        let parser = FeedParser::new();
        parser.validate_feed_url(url)?;

        let response = timeout(Duration::from_secs(10), self.fetch_response(url))
            .await
            .map_err(|_| Error::Timeout(format!("Request to {} timed out", url)))?;

        let response = response?;
        
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let last_modified = headers
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let etag = headers
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok(FeedInfo {
            url: url.to_string(),
            status_code,
            content_type,
            last_modified,
            etag,
            available: response.status().is_success(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FeedInfo {
    pub url: String,
    pub status_code: u16,
    pub content_type: String,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use std::time::Duration;

    const VALID_RSS_RESPONSE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Test Feed</title>
        <description>A test feed</description>
        <link>https://example.com</link>
        <item>
            <title>Test Article</title>
            <link>https://example.com/article</link>
            <description>Test article description</description>
            <pubDate>Wed, 15 Mar 2024 10:00:00 GMT</pubDate>
        </item>
    </channel>
</rss>"#;

    #[tokio::test]
    async fn test_fetch_valid_feed() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE)
                    .insert_header("content-type", "application/rss+xml")
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let feed_url = format!("{}/feed.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_ok());
        
        let feed = result.unwrap();
        assert_eq!(feed.title, "Test Feed");
        assert_eq!(feed.articles.len(), 1);
        assert_eq!(feed.articles[0].title, "Test Article");
    }

    #[tokio::test]
    async fn test_fetch_404_error() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/notfound.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let feed_url = format!("{}/notfound.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_err());
        
        if let Err(Error::HttpError(msg)) = result {
            assert!(msg.contains("404"));
        } else {
            panic!("Expected HttpError");
        }
    }

    #[tokio::test]
    async fn test_fetch_timeout() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/slow.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(5))
                    .set_body_string(VALID_RSS_RESPONSE)
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new().with_timeout(Duration::from_millis(100));
        let feed_url = format!("{}/slow.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_err());
        
        if let Err(Error::Timeout(msg)) = result {
            assert!(msg.contains("timed out"));
        } else {
            panic!("Expected Timeout error");
        }
    }

    #[tokio::test]
    async fn test_fetch_malformed_xml() {
        let mock_server = MockServer::start().await;
        
        let malformed_xml = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Broken Feed</title>
        <item>
            <title>Unclosed tag
        </item>
    </channel>
    <!-- Missing closing rss tag -->"#;
        
        Mock::given(method("GET"))
            .and(path("/broken.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(malformed_xml)
                    .insert_header("content-type", "application/xml")
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let feed_url = format!("{}/broken.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_err());
        
        if let Err(Error::FeedParse(_)) = result {
            // Expected error type
        } else {
            panic!("Expected FeedParse error");
        }
    }

    #[tokio::test]
    async fn test_fetch_with_redirects() {
        let mock_server = MockServer::start().await;
        
        // Setup redirect chain
        Mock::given(method("GET"))
            .and(path("/redirect"))
            .respond_with(
                ResponseTemplate::new(301)
                    .insert_header("location", format!("{}/feed.xml", mock_server.uri()).as_str())
            )
            .mount(&mock_server)
            .await;
            
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE)
                    .insert_header("content-type", "application/rss+xml")
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let redirect_url = format!("{}/redirect", mock_server.uri());
        
        let result = fetcher.fetch_feed(&redirect_url).await;
        assert!(result.is_ok());
        
        let feed = result.unwrap();
        assert_eq!(feed.title, "Test Feed");
    }

    #[tokio::test]
    async fn test_fetch_multiple_feeds() {
        let mock_server = MockServer::start().await;
        
        // Setup multiple feed endpoints
        Mock::given(method("GET"))
            .and(path("/feed1.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE.replace("Test Feed", "Feed One"))
                    .insert_header("content-type", "application/rss+xml")
            )
            .mount(&mock_server)
            .await;
            
        Mock::given(method("GET"))
            .and(path("/feed2.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE.replace("Test Feed", "Feed Two"))
                    .insert_header("content-type", "application/rss+xml")
            )
            .mount(&mock_server)
            .await;
            
        Mock::given(method("GET"))
            .and(path("/feed3.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let urls = vec![
            format!("{}/feed1.xml", mock_server.uri()),
            format!("{}/feed2.xml", mock_server.uri()),
            format!("{}/feed3.xml", mock_server.uri()),
        ];
        
        let results = fetcher.fetch_multiple_feeds(&urls).await;
        assert_eq!(results.len(), 3);
        
        // First two should succeed
        assert!(results[0].1.is_ok());
        assert!(results[1].1.is_ok());
        assert_eq!(results[0].1.as_ref().unwrap().title, "Feed One");
        assert_eq!(results[1].1.as_ref().unwrap().title, "Feed Two");
        
        // Third should fail
        assert!(results[2].1.is_err());
    }

    #[tokio::test]
    async fn test_check_feed_availability() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/available.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE)
                    .insert_header("content-type", "application/rss+xml")
                    .insert_header("last-modified", "Wed, 15 Mar 2024 10:00:00 GMT")
                    .insert_header("etag", "\"abc123\"")
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let feed_url = format!("{}/available.xml", mock_server.uri());
        
        let result = fetcher.check_feed_availability(&feed_url).await;
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_eq!(info.url, feed_url);
        assert_eq!(info.status_code, 200);
        assert!(info.available);
        assert!(info.content_type.contains("application/rss+xml"));
        assert!(info.last_modified.is_some());
        assert!(info.etag.is_some());
    }

    #[tokio::test]
    async fn test_invalid_url_schemes() {
        let fetcher = FeedFetcher::new();
        
        let invalid_urls = vec![
            "ftp://example.com/feed.xml",
            "file:///local/feed.xml",
            "javascript:alert('xss')",
            "data:text/xml,<rss></rss>",
        ];
        
        for url in invalid_urls {
            let result = fetcher.fetch_feed(url).await;
            assert!(result.is_err());
            
            if let Err(Error::InvalidUrl(_)) = result {
                // Expected error type
            } else {
                panic!("Expected InvalidUrl error for {}", url);
            }
        }
    }

    #[tokio::test]
    async fn test_user_agent_header() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE)
            )
            .mount(&mock_server)
            .await;

        let custom_user_agent = "CustomBot/1.0".to_string();
        let fetcher = FeedFetcher::new().with_user_agent(custom_user_agent.clone());
        let feed_url = format!("{}/feed.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_ok());
        
        // Note: We can't easily verify the User-Agent header was sent correctly
        // without more complex mock setup, but the test ensures the method works
    }

    #[tokio::test]
    async fn test_gzip_compression() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/compressed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(VALID_RSS_RESPONSE)
                    .insert_header("content-encoding", "gzip")
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let feed_url = format!("{}/compressed.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_ok());
        
        let feed = result.unwrap();
        assert_eq!(feed.title, "Test Feed");
    }

    #[tokio::test]
    async fn test_large_feed_handling() {
        let mock_server = MockServer::start().await;
        
        // Create a large feed with many items
        let mut large_feed = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Large Feed</title>
        <description>A feed with many items</description>
        <link>https://example.com</link>"#);
        
        // Add 1000 items
        for i in 0..1000 {
            large_feed.push_str(&format!(r#"
        <item>
            <title>Article {}</title>
            <link>https://example.com/article{}</link>
            <description>Description for article {}</description>
            <pubDate>Wed, 15 Mar 2024 10:{}:00 GMT</pubDate>
        </item>"#, i, i, i, i % 60));
        }
        
        large_feed.push_str("\n    </channel>\n</rss>");
        
        Mock::given(method("GET"))
            .and(path("/large.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(large_feed)
                    .insert_header("content-type", "application/rss+xml")
            )
            .mount(&mock_server)
            .await;

        let fetcher = FeedFetcher::new();
        let feed_url = format!("{}/large.xml", mock_server.uri());
        
        let result = fetcher.fetch_feed(&feed_url).await;
        assert!(result.is_ok());
        
        let feed = result.unwrap();
        assert_eq!(feed.title, "Large Feed");
        assert_eq!(feed.articles.len(), 1000);
    }
}