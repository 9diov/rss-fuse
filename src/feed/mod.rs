// pub mod manager;
pub mod fetcher;
pub mod parser;
// pub mod cache;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub name: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
    pub articles: Vec<Article>,
    pub status: FeedStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedStatus {
    Active,
    Error(String),
    Updating,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub read: bool,
    pub cached_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ParsedFeed {
    pub title: String,
    pub description: Option<String>,
    pub link: Option<String>,
    pub last_build_date: Option<DateTime<Utc>>,
    pub articles: Vec<ParsedArticle>,
}

#[derive(Debug, Clone)]
pub struct ParsedArticle {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub guid: Option<String>,
    pub categories: Vec<String>,
}

#[derive(Debug)]
pub struct FeedResult {
    pub feed_name: String,
    pub success: bool,
    pub error: Option<String>,
    pub articles_added: usize,
    pub articles_updated: usize,
}

impl Article {
    pub fn new(parsed: ParsedArticle, feed_name: &str) -> Self {
        let id = parsed.guid.unwrap_or_else(|| {
            format!("{}:{}", feed_name, 
                blake3::hash(parsed.link.as_bytes()).to_hex().to_string())
        });
        
        Self {
            id,
            title: parsed.title,
            link: parsed.link,
            description: parsed.description,
            content: parsed.content,
            author: parsed.author,
            published: parsed.published,
            updated: None,
            tags: parsed.categories,
            read: false,
            cached_at: Some(Utc::now()),
        }
    }
    
    /// Legacy method for backward compatibility - returns plain text format
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        
        text.push_str(&format!("Title: {}\n", self.title));
        
        if let Some(author) = &self.author {
            text.push_str(&format!("Author: {}\n", author));
        }
        
        if let Some(published) = &self.published {
            text.push_str(&format!("Published: {}\n", published.format("%Y-%m-%d %H:%M:%S UTC")));
        }
        
        text.push_str(&format!("Link: {}\n", self.link));
        
        if !self.tags.is_empty() {
            text.push_str(&format!("Tags: {}\n", self.tags.join(", ")));
        }
        
        text.push_str("\n---\n\n");
        
        if let Some(content) = &self.content {
            text.push_str(content);
        } else if let Some(description) = &self.description {
            text.push_str(description);
        } else {
            text.push_str("No content available. Visit the link above to read the full article.");
        }
        
        text
    }

    /// Convert article to Markdown format with YAML frontmatter
    pub fn to_markdown(&self, feed_name: &str) -> crate::error::Result<String> {
        use crate::content::ContentExtractor;
        let extractor = ContentExtractor::new()?;
        extractor.extract_article(self, feed_name)
    }
    
    /// Get filename with .txt extension (legacy)
    pub fn filename(&self) -> String {
        let title = self.title
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                c if c.is_control() => '-',
                c => c,
            })
            .collect::<String>();
        
        let truncated = if title.len() > 100 {
            format!("{}...", &title[..97])
        } else {
            title
        };
        
        format!("{}.txt", truncated)
    }

    /// Get filename with .md extension for Markdown format
    pub fn markdown_filename(&self) -> String {
        let title = self.title
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                c if c.is_control() => '-',
                c => c,
            })
            .collect::<String>();
        
        let truncated = if title.len() > 100 {
            format!("{}...", &title[..97])
        } else {
            title
        };
        
        format!("{}.md", truncated)
    }
}