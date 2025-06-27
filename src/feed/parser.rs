use crate::feed::{ParsedFeed, ParsedArticle};
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use feed_rs::parser as feed_parser;
use std::io::BufRead;

pub struct FeedParser;

impl FeedParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_feed<R: BufRead>(&self, reader: R) -> Result<ParsedFeed> {
        let feed = feed_parser::parse(reader)
            .map_err(|e| Error::FeedParse(format!("Failed to parse feed: {}", e)))?;

        let title = feed.title.map(|t| t.content).unwrap_or_else(|| "Untitled Feed".to_string());
        let description = feed.description.map(|d| d.content);
        let link = feed.links.first().map(|l| l.href.clone());
        let last_build_date = feed.updated.or(feed.published);

        let articles = feed
            .entries
            .into_iter()
            .map(|entry| {
                let title = entry.title.map(|t| t.content).unwrap_or_else(|| "Untitled".to_string());
                let link = entry.links.first().map(|l| l.href.clone()).unwrap_or_default();
                let description = entry.summary.map(|s| s.content);
                let content = entry.content.map(|c| c.body).flatten();
                let author = entry.authors.first().map(|a| a.name.clone());
                let published = entry.published.or(entry.updated);
                let guid = entry.id;
                let categories = entry.categories.into_iter().map(|c| c.term).collect();

                ParsedArticle {
                    title,
                    link,
                    description,
                    content,
                    author,
                    published,
                    guid: Some(guid),
                    categories,
                }
            })
            .collect();

        Ok(ParsedFeed {
            title,
            description,
            link,
            last_build_date,
            articles,
        })
    }

    pub fn validate_feed_url(&self, url: &str) -> Result<()> {
        let parsed_url = url::Url::parse(url)
            .map_err(|e| Error::InvalidUrl(format!("Invalid URL: {}", e)))?;

        match parsed_url.scheme() {
            "http" | "https" => Ok(()),
            scheme => Err(Error::InvalidUrl(format!("Unsupported scheme: {}", scheme))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const RSS_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Test RSS Feed</title>
        <description>A test RSS feed for unit testing</description>
        <link>https://example.com</link>
        <lastBuildDate>Wed, 15 Mar 2024 10:00:00 GMT</lastBuildDate>
        <item>
            <title>First Article</title>
            <link>https://example.com/first</link>
            <description>This is the first test article</description>
            <author>test@example.com (Test Author)</author>
            <pubDate>Wed, 15 Mar 2024 09:00:00 GMT</pubDate>
            <guid>https://example.com/first</guid>
            <category>test</category>
            <category>sample</category>
        </item>
        <item>
            <title>Second Article</title>
            <link>https://example.com/second</link>
            <description>This is the second test article</description>
            <pubDate>Wed, 15 Mar 2024 08:00:00 GMT</pubDate>
            <guid>unique-guid-123</guid>
        </item>
    </channel>
</rss>"#;

    const ATOM_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Test Atom Feed</title>
    <subtitle>A test Atom feed for unit testing</subtitle>
    <link href="https://example.com"/>
    <updated>2024-03-15T10:00:00Z</updated>
    <id>https://example.com/feed</id>
    <entry>
        <title>Atom Article One</title>
        <link href="https://example.com/atom1"/>
        <id>https://example.com/atom1</id>
        <updated>2024-03-15T09:00:00Z</updated>
        <published>2024-03-15T09:00:00Z</published>
        <summary>Summary of the first atom article</summary>
        <content type="html">&lt;p&gt;Full content of the first atom article&lt;/p&gt;</content>
        <author>
            <name>Atom Author</name>
            <email>atom@example.com</email>
        </author>
        <category term="atom"/>
        <category term="test"/>
    </entry>
</feed>"#;

    const MALFORMED_XML: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Broken Feed</title>
        <item>
            <title>Unclosed tag
            <link>https://example.com/broken</link>
        </item>
    </channel>
    <!-- Missing closing rss tag -->"#;

    #[test]
    fn test_parse_rss_feed() {
        let parser = FeedParser::new();
        let cursor = Cursor::new(RSS_SAMPLE.as_bytes());
        
        let result = parser.parse_feed(cursor).unwrap();
        
        assert_eq!(result.title, "Test RSS Feed");
        assert_eq!(result.description, Some("A test RSS feed for unit testing".to_string()));
        assert_eq!(result.link, Some("https://example.com/".to_string()));
        assert_eq!(result.articles.len(), 2);
        
        let first_article = &result.articles[0];
        assert_eq!(first_article.title, "First Article");
        assert_eq!(first_article.link, "https://example.com/first");
        assert_eq!(first_article.description, Some("This is the first test article".to_string()));
        assert_eq!(first_article.guid, Some("https://example.com/first".to_string()));
        assert_eq!(first_article.categories, vec!["test", "sample"]);
        assert!(first_article.published.is_some());
    }

    #[test]
    fn test_parse_atom_feed() {
        let parser = FeedParser::new();
        let cursor = Cursor::new(ATOM_SAMPLE.as_bytes());
        
        let result = parser.parse_feed(cursor).unwrap();
        
        assert_eq!(result.title, "Test Atom Feed");
        assert_eq!(result.description, Some("A test Atom feed for unit testing".to_string()));
        assert_eq!(result.articles.len(), 1);
        
        let article = &result.articles[0];
        assert_eq!(article.title, "Atom Article One");
        assert_eq!(article.link, "https://example.com/atom1");
        assert_eq!(article.content, Some("<p>Full content of the first atom article</p>".to_string()));
        assert_eq!(article.author, Some("Atom Author".to_string()));
        assert_eq!(article.categories, vec!["atom", "test"]);
    }

    #[test]
    fn test_parse_malformed_xml() {
        let parser = FeedParser::new();
        let cursor = Cursor::new(MALFORMED_XML.as_bytes());
        
        let result = parser.parse_feed(cursor);
        assert!(result.is_err());
        
        if let Err(Error::FeedParse(msg)) = result {
            assert!(msg.contains("Failed to parse feed"));
        } else {
            panic!("Expected FeedParse error");
        }
    }

    #[test]
    fn test_empty_feed() {
        let parser = FeedParser::new();
        let empty_rss = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Empty Feed</title>
    </channel>
</rss>"#;
        
        let cursor = Cursor::new(empty_rss.as_bytes());
        let result = parser.parse_feed(cursor).unwrap();
        
        assert_eq!(result.title, "Empty Feed");
        assert_eq!(result.articles.len(), 0);
    }

    #[test]
    fn test_feed_with_missing_titles() {
        let parser = FeedParser::new();
        let no_title_feed = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <item>
            <link>https://example.com/notitle</link>
            <description>Article without title</description>
        </item>
    </channel>
</rss>"#;
        
        let cursor = Cursor::new(no_title_feed.as_bytes());
        let result = parser.parse_feed(cursor).unwrap();
        
        assert_eq!(result.title, "Untitled Feed");
        assert_eq!(result.articles.len(), 1);
        assert_eq!(result.articles[0].title, "Untitled");
    }

    #[test]
    fn test_validate_feed_url_valid() {
        let parser = FeedParser::new();
        
        assert!(parser.validate_feed_url("https://example.com/feed.xml").is_ok());
        assert!(parser.validate_feed_url("http://example.com/rss").is_ok());
        assert!(parser.validate_feed_url("https://subdomain.example.com/path/to/feed?param=value").is_ok());
    }

    #[test]
    fn test_validate_feed_url_invalid() {
        let parser = FeedParser::new();
        
        assert!(parser.validate_feed_url("not-a-url").is_err());
        assert!(parser.validate_feed_url("ftp://example.com/feed").is_err());
        assert!(parser.validate_feed_url("file:///local/feed.xml").is_err());
        assert!(parser.validate_feed_url("").is_err());
        assert!(parser.validate_feed_url("javascript:alert('xss')").is_err());
    }

    #[test]
    fn test_feed_with_html_entities() {
        let parser = FeedParser::new();
        let html_entities_feed = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Feed with &amp; HTML &lt;entities&gt;</title>
        <item>
            <title>Article with &quot;quotes&quot; &amp; symbols</title>
            <description>&lt;p&gt;HTML content with &amp;amp; entities&lt;/p&gt;</description>
            <link>https://example.com/entities</link>
        </item>
    </channel>
</rss>"#;
        
        let cursor = Cursor::new(html_entities_feed.as_bytes());
        let result = parser.parse_feed(cursor).unwrap();
        
        assert_eq!(result.title, "Feed with & HTML <entities>");
        assert_eq!(result.articles[0].title, r#"Article with "quotes" & symbols"#);
    }

    #[test]
    fn test_feed_with_cdata() {
        let parser = FeedParser::new();
        let cdata_feed = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>CDATA Feed</title>
        <item>
            <title><![CDATA[Article with <HTML> in CDATA]]></title>
            <description><![CDATA[<p>This is <strong>HTML</strong> content in CDATA</p>]]></description>
            <link>https://example.com/cdata</link>
        </item>
    </channel>
</rss>"#;
        
        let cursor = Cursor::new(cdata_feed.as_bytes());
        let result = parser.parse_feed(cursor).unwrap();
        
        assert_eq!(result.articles[0].title, "Article with <HTML> in CDATA");
        assert!(result.articles[0].description.as_ref().unwrap().contains("<strong>HTML</strong>"));
    }
}